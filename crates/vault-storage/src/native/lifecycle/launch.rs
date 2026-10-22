//! Owned runtime process and nonblocking byte pipes. Installation selection and
//! effective configuration checks belong to the private operation controller.
//! No renderer-provided path, command, environment or process id is accepted.
use super::super::*;
use super::helper::{ExitObservation, OwnedFamily};
use crate::lifecycle_model::Fault;
use std::collections::BTreeMap;
use windows_sys::Win32::System::{Pipes::*, Threading::*};

const PIPE_BYTES: u32 = 65536;

struct Pipe {
    server: NonblockingPipe,
    client: File,
}

/// Unlike std::fs::File on Windows, preserve empty nonblocking pipe versus EOF.
pub(super) struct NonblockingPipe(File);
impl AsRawHandle for NonblockingPipe {
    fn as_raw_handle(&self) -> std::os::windows::io::RawHandle {
        self.0.as_raw_handle()
    }
}
impl Read for NonblockingPipe {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let size = u32::try_from(bytes.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        if size == 0 {
            return Ok(0);
        }
        let mut count = 0;
        // SAFETY: validated nonblocking byte-pipe handle and writable exact buffer.
        if unsafe {
            ReadFile(
                self.as_raw_handle(),
                bytes.as_mut_ptr(),
                size,
                &mut count,
                null_mut(),
            )
        } != 0
        {
            return Ok(count as usize);
        }
        match unsafe { GetLastError() } {
            ERROR_NO_DATA | ERROR_PIPE_LISTENING => Err(std::io::ErrorKind::WouldBlock.into()),
            ERROR_BROKEN_PIPE => Ok(0),
            code => Err(std::io::Error::from_raw_os_error(code as i32)),
        }
    }
}
impl Write for NonblockingPipe {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let size = u32::try_from(bytes.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        if size == 0 {
            return Ok(0);
        }
        let mut count = 0;
        // SAFETY: validated nonblocking byte-pipe handle and borrowed exact buffer.
        if unsafe {
            WriteFile(
                self.as_raw_handle(),
                bytes.as_ptr(),
                size,
                &mut count,
                null_mut(),
            )
        } != 0
        {
            return if count == 0 {
                Err(std::io::ErrorKind::WouldBlock.into())
            } else {
                Ok(count as usize)
            };
        }
        match unsafe { GetLastError() } {
            ERROR_NO_DATA | ERROR_BROKEN_PIPE => Err(std::io::ErrorKind::BrokenPipe.into()),
            ERROR_PIPE_BUSY | ERROR_PIPE_LISTENING => Err(std::io::ErrorKind::WouldBlock.into()),
            code => Err(std::io::Error::from_raw_os_error(code as i32)),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Pipe {
    fn new(input: bool, security: &Security) -> Result<Self, StorageError> {
        let id: String = random_id()?.iter().map(|b| format!("{b:02x}")).collect();
        let name: Vec<u16> = format!("\\\\.\\pipe\\CodexAccounts.Runtime.{id}\0")
            .encode_utf16()
            .collect();
        let attrs = security.attrs();
        // SAFETY: fresh unpredictable local name, first instance, noninherited
        // server, current-user/SYSTEM DACL. Remote clients are rejected.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                // Duplex grants the parent read-attributes access needed to
                // verify PIPE_NOWAIT. The inherited stdin client is read-only.
                (if input {
                    PIPE_ACCESS_DUPLEX
                } else {
                    PIPE_ACCESS_INBOUND
                }) | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                PIPE_BYTES,
                PIPE_BYTES,
                0,
                &attrs,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(last());
        }
        // SAFETY: successful uniquely owned file handle transferred exactly once.
        let server = unsafe { File::from_raw_handle(handle) };
        let client_attrs = SECURITY_ATTRIBUTES {
            bInheritHandle: 1,
            ..attrs
        };
        // SAFETY: connects only the newly created local endpoint. This client end
        // is inherited only through the explicit three-handle process list.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                if input { GENERIC_READ } else { GENERIC_WRITE },
                0,
                &client_attrs,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(last());
        }
        let client = unsafe { File::from_raw_handle(handle) };
        // Connect may report that CreateFile already completed the connection.
        if unsafe { ConnectNamedPipe(server.as_raw_handle(), null_mut()) } == 0
            && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED
        {
            return Err(last());
        }
        let mut mode = 0;
        // SAFETY: inspect the exact retained server handle before handing it to
        // safe polling code; a blocking handle is never returned.
        if unsafe {
            GetNamedPipeHandleStateW(
                server.as_raw_handle(),
                &mut mode,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                0,
            )
        } == 0
            || mode & PIPE_NOWAIT == 0
            || mode & PIPE_READMODE_MESSAGE != 0
        {
            return Err(StorageError::Io);
        }
        Ok(Self {
            server: NonblockingPipe(server),
            client,
        })
    }
}

struct Attributes {
    storage: Vec<usize>,
}
impl Attributes {
    fn new(handles: &mut [HANDLE; 3]) -> Result<Self, StorageError> {
        let mut bytes = 0;
        // SAFETY: documented size-query call; the first call intentionally fails.
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes);
        }
        if bytes == 0 || bytes > 65536 {
            return Err(StorageError::InputLimit);
        }
        let mut value = Self {
            storage: vec![0; bytes.div_ceil(std::mem::size_of::<usize>())],
        };
        if unsafe { InitializeProcThreadAttributeList(value.ptr(), 1, 0, &mut bytes) } == 0 {
            // No initialized list exists to delete on this failure.
            value.storage.clear();
            return Err(last());
        }
        // SAFETY: initialized aligned list and the three owned inheritable pipe
        // handles. The caller retains this array through CreateProcessW.
        if unsafe {
            UpdateProcThreadAttribute(
                value.ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_mut_ptr().cast(),
                std::mem::size_of_val(handles),
                null_mut(),
                null(),
            )
        } == 0
        {
            return Err(last());
        }
        Ok(value)
    }
    fn ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        if !self.storage.is_empty() {
            // SAFETY: exactly one successfully initialized list in retained storage.
            unsafe {
                DeleteProcThreadAttributeList(self.ptr());
            }
        }
    }
}

/// Parent-only pipe endpoints are nonblocking. Native ownership stays with the
/// controller until the runtime closes stdin and this family is observed exited.
pub(super) struct RuntimeChild {
    pub(super) input: Option<NonblockingPipe>,
    pub(super) output: Option<NonblockingPipe>,
    pub(super) error: Option<NonblockingPipe>,
    family: OwnedFamily,
    _image: File,
}
impl Drop for RuntimeChild {
    fn drop(&mut self) {
        // Closing a kill-on-close job requests asynchronous termination. Retain
        // ownership while bounded reaping runs, including error/unwind paths,
        // before releasing handles and the caller's home guard.
        self.input.take();
        self.output.take();
        self.error.take();
        let _ = self.family.shutdown_after_input_closed();
        // Explicit close_and_wait remains the required success receipt. Drop is
        // best-effort cleanup and never authorizes a subsequent credential effect.
    }
}

/// Drop order closes the protocol endpoints before the contained process family.
pub(super) struct RuntimeConnection {
    pub(super) session: Option<codex_accounts_runtime::PipeSession>,
    pub(super) cancellation: codex_accounts_runtime::PipeCancellation,
    child: RuntimeChild,
}
impl RuntimeConnection {
    pub(super) fn open(
        installation: &codex_accounts_discovery::OfficialInstallation,
        home: &Path,
        login: bool,
        registered_core: bool,
    ) -> Result<Self, StorageError> {
        installation
            .revalidate()
            .map_err(|_| StorageError::ExternalChange)?;
        let mut child = RuntimeChild::spawn(installation.runtime_path(), home, registered_core)?;
        let result = codex_accounts_runtime::PipeSession::from_nonblocking_io(
            Box::new(child.input.take().ok_or(StorageError::Io)?),
            Box::new(child.output.take().ok_or(StorageError::Io)?),
            Box::new(child.error.take().ok_or(StorageError::Io)?),
            login,
        );
        match result {
            Ok((session, cancellation)) => Ok(Self {
                session: Some(session),
                cancellation,
                child,
            }),
            Err(_) => {
                child
                    .close_and_wait()
                    .map_err(|_| StorageError::RecoveryRequired)?;
                Err(StorageError::Io)
            }
        }
    }
    pub(super) fn close_and_wait(&mut self) -> Result<ExitObservation, Fault> {
        // Dropping the session closes stdin even after a protocol/deadline error.
        // Close all readers too: no raw stderr is retained or logged here.
        self.session.take();
        self.child.close_and_wait()
    }
}

impl RuntimeChild {
    /// Private launch primitive. Caller must select/verify the official runtime
    /// and effective home first. This performs no version-allowlist admission.
    pub(super) fn spawn(
        runtime: &Path,
        home: &Path,
        registered_core: bool,
    ) -> Result<Self, StorageError> {
        validate_path(runtime)?;
        validate_path(home)?;
        let image = open_file(
            runtime,
            GENERIC_READ | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        // MSIX images may have package-manager-owned hardlinks and capability
        // ACLs. Their authority is OfficialInstallation, not vault-file policy.
        // Keep the image pinned without imposing single-link credential rules.
        let image_stamp = stamp(&image)?;
        if unsafe { GetFileType(image.as_raw_handle()) } != FILE_TYPE_DISK
            || image_stamp.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
                != 0
        {
            return Err(StorageError::UnsafePath);
        }
        let security = Security::new()?;
        let input = Pipe::new(true, &security)?;
        let output = Pipe::new(false, &security)?;
        let error = Pipe::new(false, &security)?;
        let mut handles = [
            input.client.as_raw_handle(),
            output.client.as_raw_handle(),
            error.client.as_raw_handle(),
        ];
        let mut attrs = Attributes::new(&mut handles)?;
        let startup = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: std::mem::size_of::<STARTUPINFOEXW>() as u32,
                dwFlags: STARTF_USESTDHANDLES,
                hStdInput: handles[0],
                hStdOutput: handles[1],
                hStdError: handles[2],
                ..Default::default()
            },
            lpAttributeList: attrs.ptr(),
        };
        let application = wide(runtime)?;
        let executable = runtime.to_str().ok_or(StorageError::UnsafePath)?;
        if executable.contains('"') {
            return Err(StorageError::UnsafePath);
        }
        let mut command: Vec<u16> = format!("\"{executable}\" app-server\0")
            .encode_utf16()
            .collect();
        let environment = environment(home, registered_core)?;
        let directory = wide(home)?;
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: fixed command and explicit executable, private environment,
        // exactly three inherited handles; owned child starts suspended.
        if unsafe {
            CreateProcessW(
                application.as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                1,
                CREATE_SUSPENDED
                    | CREATE_UNICODE_ENVIRONMENT
                    | CREATE_NO_WINDOW
                    | EXTENDED_STARTUPINFO_PRESENT,
                environment.as_ptr().cast(),
                directory.as_ptr(),
                &startup.StartupInfo,
                &mut info,
            )
        } == 0
        {
            return Err(last());
        }
        let process = Token(info.hProcess);
        let thread = Token(info.hThread);
        let family = match OwnedFamily::for_suspended_child(process.0, info.dwProcessId) {
            Ok(family) => family,
            Err(_) => {
                // Only this just-created, still-suspended child can be terminated.
                // It has never run user code or created descendants.
                unsafe {
                    TerminateProcess(process.0, 1);
                }
                if unsafe { WaitForSingleObject(process.0, 5000) } != WAIT_OBJECT_0 {
                    return Err(StorageError::RecoveryRequired);
                }
                return Err(StorageError::Io);
            }
        };
        // Parent copies of child endpoints must close so stdin close/EOF works.
        drop(input.client);
        drop(output.client);
        drop(error.client);
        let mut child = Self {
            input: Some(input.server),
            output: Some(output.server),
            error: Some(error.server),
            family,
            _image: image,
        };
        // SAFETY: original thread of the exclusively owned child, assigned to a
        // non-breakaway kill-on-close job before any application code executes.
        if unsafe { ResumeThread(thread.0) } != 1 {
            child
                .close_and_wait()
                .map_err(|_| StorageError::RecoveryRequired)?;
            return Err(StorageError::Io);
        }
        Ok(child)
    }
    pub(super) fn close_and_wait(&mut self) -> Result<ExitObservation, Fault> {
        self.input.take();
        self.family.shutdown_after_input_closed()
    }
}

fn environment(home: &Path, registered_core: bool) -> Result<Vec<u16>, StorageError> {
    let mut values = BTreeMap::<String, OsString>::new();
    for name in [
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "WINDIR",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "PROGRAMDATA",
    ] {
        if let Some(value) = std::env::var_os(name) {
            values.insert(name.to_owned(), value);
        }
    }
    if !values.contains_key("SYSTEMROOT") {
        return Err(StorageError::InvalidData);
    }
    values.insert("CODEX_HOME".to_owned(), home.as_os_str().to_owned());
    if registered_core {
        values.insert(
            "CODEX_WINDOWS_REGISTERED_CORE".to_owned(),
            OsString::from("1"),
        );
    }
    let mut block = Vec::new();
    for (name, value) in values {
        block.extend(name.encode_utf16());
        block.push(u16::from(b'='));
        let value: Vec<_> = value.encode_wide().collect();
        if value.contains(&0) {
            return Err(StorageError::InvalidData);
        }
        block.extend(value);
        block.push(0);
        if block.len() >= 32766 {
            return Err(StorageError::InputLimit);
        }
    }
    block.push(0);
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "explicit native configuration and login-start/cancel smoke in a fresh empty synthetic home"]
    fn official_runtime_protocol_in_empty_home() {
        use codex_accounts_runtime::PipeEvent;
        eprintln!("runtime smoke: fixture");
        let fixture = super::super::super::tests::Sandbox::new();
        let security = Security::new().unwrap();
        assert_ne!(
            unsafe { CreateDirectoryW(wide(&fixture.0).unwrap().as_ptr(), &security.attrs()) },
            0
        );
        eprintln!("runtime smoke: discover");
        let installation = codex_accounts_discovery::OfficialInstallation::discover().unwrap();
        eprintln!("runtime smoke: open");
        let registered_core = installation.contained_core() && installation.provisioned_core();
        let mut connection =
            RuntimeConnection::open(&installation, &fixture.0, true, registered_core).unwrap();
        eprintln!("runtime smoke: initialize");
        let session = connection.session.as_mut().unwrap();
        session.initialize().unwrap();
        loop {
            match session.next_event().unwrap() {
                PipeEvent::Initialized {
                    platform_os, home, ..
                } => {
                    assert_eq!(platform_os.as_str(), "windows");
                    assert_eq!(Path::new(home.as_str()), fixture.0);
                    break;
                }
                PipeEvent::Written(_) | PipeEvent::IgnoredNotification => {}
                _ => panic!("unexpected initialize event"),
            }
        }
        session.initialized().unwrap();
        eprintln!("runtime smoke: initialized");
        assert!(matches!(
            session.next_event().unwrap(),
            PipeEvent::Written(_)
        ));
        session.read_config_requirements().unwrap();
        eprintln!("runtime smoke: requirements");
        let policy = loop {
            match session.next_event().unwrap() {
                PipeEvent::Requirements(value) => break value.requirements,
                PipeEvent::Written(_) | PipeEvent::IgnoredNotification => {}
                _ => panic!("unexpected requirements event"),
            }
        };
        session.read_config().unwrap();
        eprintln!("runtime smoke: config");
        let config = loop {
            match session.next_event().unwrap() {
                PipeEvent::Config(value) => break value.config,
                PipeEvent::Written(_) | PipeEvent::IgnoredNotification => {}
                _ => panic!("unexpected config event"),
            }
        };
        assert_eq!(
            super::super::runtime_context::compatible(config, policy),
            Ok(()),
            "official runtime defaults must permit managed sign-in"
        );
        // Start and cancel the official flow without opening its URL or
        // completing authentication. Never inspect an existing user's home.
        eprintln!("runtime smoke: login start");
        session.start_login().unwrap();
        loop {
            match session.next_event().unwrap() {
                PipeEvent::LoginStarted { .. } => break,
                PipeEvent::Written(_)
                | PipeEvent::IgnoredNotification
                | PipeEvent::AccountUpdated
                | PipeEvent::RateLimitsUpdated(_) => {}
                _ => panic!("unexpected login-start event"),
            }
        }
        eprintln!("runtime smoke: login cancel");
        connection.cancellation.request();
        loop {
            match session.next_event().unwrap() {
                PipeEvent::CancellationAcknowledged => break,
                PipeEvent::Written(_)
                | PipeEvent::IgnoredNotification
                | PipeEvent::AccountUpdated
                | PipeEvent::RateLimitsUpdated(_)
                | PipeEvent::CancellationProgress
                | PipeEvent::LoginCompletionBuffered => {}
                _ => panic!("unexpected login-cancel event"),
            }
        }
        eprintln!("runtime smoke: close and wait");
        connection.close_and_wait().unwrap();
        eprintln!("runtime smoke: revalidate");
        installation.revalidate().unwrap();
        assert!(!fixture.0.join("auth.json").exists());
        eprintln!("runtime smoke: complete");
    }

    #[test]
    fn private_byte_pipes_are_nonblocking_and_transfer_only_written_bytes() {
        let security = Security::new().unwrap();
        let mut output = Pipe::new(false, &security).unwrap();
        let mut buffer = [0u8; 32];
        let pending = output.server.read(&mut buffer).unwrap_err();
        assert_eq!(pending.kind(), std::io::ErrorKind::WouldBlock);
        output.client.write_all(b"SYNTHETIC\n").unwrap();
        let count = output.server.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..count], b"SYNTHETIC\n");
        drop(output.client);
        assert_eq!(output.server.read(&mut buffer).unwrap(), 0);

        let mut input = Pipe::new(true, &security).unwrap();
        input.server.write_all(b"SYNTHETIC\n").unwrap();
        let count = input.client.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..count], b"SYNTHETIC\n");
    }

    #[test]
    fn newly_created_runtime_child_is_contained_and_reaped() {
        let fixture = super::super::super::tests::Sandbox::new();
        let security = Security::new().unwrap();
        assert_ne!(
            unsafe { CreateDirectoryW(wide(&fixture.0).unwrap().as_ptr(), &security.attrs()) },
            0
        );
        // A Rust test executable interprets the fixed production 'app-server'
        // argument as a filter with no matching tests, then exits. This exercises
        // real suspended creation, stdio inheritance, job assignment and reaping
        // without invoking an official runtime or accessing authentication.
        let mut child =
            RuntimeChild::spawn(&std::env::current_exe().unwrap(), &fixture.0, false).unwrap();
        let exit = child.close_and_wait().unwrap();
        assert!(!exit.forced);
        assert!(child.family.poll_exit().unwrap());
    }
}
