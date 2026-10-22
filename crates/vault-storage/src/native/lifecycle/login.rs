//! Managed browser sign-in uses a generated private home. The selected Desktop
//! home is preserved before helper startup. Reauthenticating its current account
//! installs the fresh capture through the journaled same-profile transaction.
use super::super::*;
use super::{
    home::HomeLock,
    launch::RuntimeConnection,
    process::Inventory,
    runtime_context::{self, RuntimeContext},
    target::Target,
};
use crate::{
    auth_capture, LoginCancellation, LoginStage, OperationError, ProfileId, ProfileText,
    ReauthenticationResult, Vault,
};
use codex_accounts_runtime::{
    LoginCompletion, PipeEvent, PipeSession, PipeSessionError, SessionError,
};
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
use zeroize::Zeroizing;
#[path = "login_record.rs"]
mod record;

const NAMESPACE: &str = "CodexAccounts-Login-v1";
const MARKER: &str = "owner-v1";
const MARKER_BYTES: &[u8] = b"CodexAccounts isolated login homes v1\n";
// The official helper populates thousands of bundled/runtime support entries
// while the browser is open. Keep a bounded, generous inventory for that tree.
const MAX_ENTRIES: usize = 65536;

struct Completion(LoginCancellation);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.detach();
        self.0.stage(LoginStage::Finished);
    }
}

impl Vault {
    /// Sign in through the official helper and save an encrypted profile without
    /// changing the account selected in Desktop's home. URLs stay in the backend.
    pub fn add_account(
        &mut self,
        label: ProfileText,
        cancel: LoginCancellation,
    ) -> Result<ProfileId, OperationError> {
        self.login_account(label, None, cancel)
            .map(|result| result.profile)
    }

    /// Refresh one intended saved account. A different account is never adopted
    /// or saved by this operation; the separate Add Account action remains available.
    pub fn reauthenticate(
        &mut self,
        profile: ProfileId,
        cancel: LoginCancellation,
    ) -> Result<ReauthenticationResult, OperationError> {
        self.storage.ensure_operation_idle()?;
        let label = self
            .storage
            .profiles()?
            .into_iter()
            .find(|entry| entry.id == profile)
            .ok_or(StorageError::Missing)?
            .label;
        super::consent::reauthenticate(&label)?;
        self.login_account(ProfileText::new(label)?, Some(profile), cancel)
    }

    fn login_account(
        &mut self,
        label: ProfileText,
        intended: Option<ProfileId>,
        cancel: LoginCancellation,
    ) -> Result<ReauthenticationResult, OperationError> {
        let _completion = Completion(cancel.clone());
        cancel.stage(LoginStage::Preparing);
        self.storage.ensure_operation_idle()?;
        cancelled(&cancel)?;
        let mut context = RuntimeContext::discover()?;
        context.quiesce()?;
        let namespace =
            Namespace::open(&known_folder(&FOLDERID_LocalAppData)?.join(NAMESPACE), true)?
                .ok_or(OperationError("E_TEMP_CLEANUP_FAILED"))?;
        namespace
            .cleanup_all(&self.root)
            .map_err(|_| OperationError("E_TEMP_CLEANUP_FAILED"))?;
        let current = Target::open_file_backend(context.home())?;
        context.check_selection()?;
        // AuthManager may refresh or remove credentials even for config/read.
        // Preserve the outgoing account durably before starting that helper.
        let before = capture_optional(&current)?;
        let identity = before
            .as_ref()
            .map(|value| {
                let (issuer, subject, workspace) = value.identity.components();
                codex_accounts_vault::Identity::new(issuer.into(), subject.into(), workspace.into())
                    .map_err(StorageError::from)
            })
            .transpose()?;
        let current_profile = if let Some(capture) = before {
            Some(self.storage.save_current(
                &self.root,
                capture,
                Some(ProfileText::new("Current account".into())?),
            )?)
        } else {
            None
        };
        let install_current = intended.filter(|profile| Some(*profile) == current_profile);
        cancelled(&cancel)?;
        let record = record::Onboarding::begin(&namespace, &self.root, intended)?;
        let path = record.home(&namespace);
        let result = (|| {
            let configuration = context.check_configuration();
            if matches!(configuration, Err(crate::journal::Failure::HelperStuck)) {
                return Err(OperationError("E_HELPER_STUCK"));
            }
            context.revalidate()?;
            context.quiesce()?;
            let after = capture_optional(&current)?;
            if after.as_ref().map(|value| &value.identity) != identity.as_ref() {
                return Err(OperationError("E_EXTERNAL_CHANGE"));
            }
            if let Some(capture) = after {
                self.storage.save_current(&self.root, capture, None)?;
            }
            configuration?;
            cancelled(&cancel)?;
            namespace.create_home(&path)?;
            self.login_in_home(&context, &path, intended, &cancel)
        })();
        cancel.stage(LoginStage::Cleaning);
        // A failed reap leaves all scratch data intact for a later safe retry.
        if let Err(error) = &result {
            if error.code() == "E_HELPER_STUCK" {
                return Err(*error);
            }
        }
        namespace
            .cleanup_registered(&record)
            .map_err(|_| OperationError("E_TEMP_CLEANUP_FAILED"))?;
        // Nothing has published the fresh capture yet. Drop both namespaces'
        // guards before the installer obtains its own freshly checked home lock.
        drop(record);
        drop(current);
        drop(context);
        drop(namespace);
        let capture = result?;
        cancelled(&cancel)?;
        // Adding another account does not authorize replacing the current
        // account's credentials. Use explicit reauthentication for that repair.
        if intended.is_none() && identity.as_ref() == Some(&capture.identity) {
            return Err(OperationError("E_ALREADY_CURRENT"));
        }
        cancel.stage(LoginStage::Saving);
        if let Some(profile) = install_current {
            // The transaction atomically registers its fresh target and recovery.
            // Cancellation after writes remains journaled forward recovery.
            let installation =
                self.install_reauthenticated_current(profile, capture, cancel.clone())?;
            Ok(ReauthenticationResult {
                profile,
                installation: Some(installation),
            })
        } else {
            self.save_login_capture(capture, label, intended)
                .map(|profile| ReauthenticationResult {
                    profile,
                    installation: None,
                })
        }
    }

    fn login_in_home(
        &mut self,
        context: &RuntimeContext,
        path: &Path,
        intended: Option<ProfileId>,
        cancel: &LoginCancellation,
    ) -> Result<crate::Capture, OperationError> {
        let target = Target::open_file_backend(path)?;
        context.revalidate()?;
        context.quiesce()?;
        let mut connection = RuntimeConnection::open(
            context.installation(),
            path,
            true,
            context.registered_core(),
        )
        .map_err(|error| {
            if error == StorageError::RecoveryRequired {
                OperationError("E_HELPER_STUCK")
            } else {
                error.into()
            }
        })?;
        let result = cancel
            .attach(connection.cancellation.clone())
            .and_then(|()| {
                protocol(
                    connection
                        .session
                        .as_mut()
                        .ok_or(OperationError("E_LOGIN_FAILED"))?,
                    path,
                    cancel,
                )
            });
        let reaped = connection
            .close_and_wait()
            .map_err(|_| OperationError("E_HELPER_STUCK"));
        cancel.detach();
        reaped?;
        result?;
        cancelled(cancel)?;
        context.revalidate()?;
        context.quiesce()?;
        let capture = capture_optional(&target)?.ok_or(OperationError("E_LOGIN_FAILED"))?;
        self.check_login_identity(&capture, intended)?;
        Ok(capture)
    }

    fn save_login_capture(
        &mut self,
        capture: crate::Capture,
        label: ProfileText,
        intended: Option<ProfileId>,
    ) -> Result<ProfileId, OperationError> {
        self.check_login_identity(&capture, intended)?;
        // The durable active association still describes the real Desktop home.
        self.storage
            .save_login(&self.root, capture, label)
            .map_err(Into::into)
    }

    fn check_login_identity(
        &self,
        capture: &crate::Capture,
        intended: Option<ProfileId>,
    ) -> Result<(), OperationError> {
        if let Some(profile) = intended {
            let matched = self
                .storage
                .current_reference(&capture.identity)
                .is_ok_and(|reference| reference.profile == profile.to_bytes());
            if !matched {
                return Err(OperationError("E_IDENTITY_MISMATCH"));
            }
        }
        Ok(())
    }
}

fn capture_optional(target: &Target) -> Result<Option<crate::Capture>, OperationError> {
    target
        .read(0)?
        .map(|mut bytes| {
            auth_capture::capture(std::mem::take(&mut *bytes))
                .map_err(|_| OperationError("E_AUTH_FORMAT_UNSUPPORTED"))
        })
        .transpose()
}
fn cancelled(cancel: &LoginCancellation) -> Result<(), OperationError> {
    if cancel.cancelled() {
        Err(OperationError("E_CANCELLED"))
    } else {
        Ok(())
    }
}
fn protocol_error(error: PipeSessionError) -> OperationError {
    OperationError(match error.primary {
        SessionError::Cancelled | SessionError::CancellationTimeout => "E_CANCELLED",
        SessionError::LoginTimeout | SessionError::TotalTimeout => "E_LOGIN_TIMEOUT",
        _ => "E_LOGIN_FAILED",
    })
}
fn next(session: &mut PipeSession) -> Result<PipeEvent, OperationError> {
    loop {
        match session.next_event().map_err(protocol_error)? {
            PipeEvent::IgnoredNotification
            | PipeEvent::AccountUpdated
            | PipeEvent::RateLimitsUpdated(_)
            | PipeEvent::Written(_) => {}
            event => return Ok(event),
        }
    }
}
fn protocol(
    session: &mut PipeSession,
    path: &Path,
    cancel: &LoginCancellation,
) -> Result<(), OperationError> {
    cancelled(cancel)?;
    session.initialize().map_err(protocol_error)?;
    match next(session)? {
        PipeEvent::Initialized {
            home,
            platform_family,
            platform_os,
            ..
        } if super::same_path(Path::new(home.as_str()), path)
            && platform_family.as_str() == "windows"
            && platform_os.as_str() == "windows" => {}
        _ => return Err(OperationError("E_COMPAT_UNKNOWN")),
    }
    session.initialized().map_err(protocol_error)?;
    loop {
        match session.next_event().map_err(protocol_error)? {
            PipeEvent::Written(_) => break,
            PipeEvent::IgnoredNotification
            | PipeEvent::AccountUpdated
            | PipeEvent::RateLimitsUpdated(_) => {}
            _ => return Err(OperationError("E_COMPAT_UNKNOWN")),
        }
    }
    session.read_config_requirements().map_err(protocol_error)?;
    let policy = match next(session)? {
        PipeEvent::Requirements(value) => value.requirements,
        _ => return Err(OperationError("E_POLICY_DENIED")),
    };
    session.read_config().map_err(protocol_error)?;
    let config = match next(session)? {
        PipeEvent::Config(value) => value.config,
        _ => return Err(OperationError("E_POLICY_DENIED")),
    };
    runtime_context::compatible(config, policy)?;
    cancelled(cancel)?;
    session.start_login().map_err(protocol_error)?;
    let mut opened = false;
    loop {
        match next(session)? {
            PipeEvent::LoginStarted { auth_url } if !opened => {
                cancelled(cancel)?;
                open_browser(auth_url.as_str())?;
                opened = true;
                cancel.stage(LoginStage::WaitingForBrowser);
            }
            PipeEvent::LoginCompleted(LoginCompletion::Success) => return cancelled(cancel),
            PipeEvent::LoginCompleted(LoginCompletion::Failure) => {
                return Err(OperationError("E_LOGIN_FAILED"))
            }
            PipeEvent::LoginCompletionBuffered | PipeEvent::CancellationProgress => {}
            PipeEvent::CancellationAcknowledged => return Err(OperationError("E_CANCELLED")),
            _ => return Err(OperationError("E_LOGIN_FAILED")),
        }
    }
}
fn approved_url(url: &str) -> bool {
    // Official codex-rs/login/src/server.rs DEFAULT_ISSUER and authorize path.
    // No shell command line or caller-supplied authority is constructed.
    url.len() <= 16384
        && url.starts_with("https://auth.openai.com/oauth/authorize?")
        && url.len() > "https://auth.openai.com/oauth/authorize?".len()
        && url
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'\\' | b'"' | b'\'' | b'#'))
}
fn open_browser(url: &str) -> Result<(), OperationError> {
    if !approved_url(url) {
        return Err(OperationError("E_LOGIN_URL_REFUSED"));
    }
    let url = Zeroizing::new(url.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    // SAFETY: bounded NUL-terminated validated HTTPS URL, fixed verb, no args.
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            url.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        Err(OperationError("E_BROWSER_OPEN_FAILED"))
    } else {
        Ok(())
    }
}

struct Namespace {
    path: PathBuf,
    lock: HomeLock,
    security: Security,
    _marker: File,
}
impl Namespace {
    fn open(path: &Path, create: bool) -> Result<Option<Self>, StorageError> {
        // Startup has nothing to clean when the fixed namespace is absent. Do
        // not turn the broad LocalAppData parent into a home-lock requirement
        // for users who have never used Add Account. This is a metadata-only
        // probe; an existing child still passes every retained-path/ACL check.
        if !create {
            match open_file(
                path,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                OPEN_EXISTING,
                None,
            ) {
                Err(StorageError::Missing) => return Ok(None),
                Err(error) => return Err(error),
                Ok(_) => {}
            }
        }
        let security = inherited_security()?;
        let mut parents = Vec::new();
        let created = match open_file(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => {
                check_object(&file, true)?;
                security.check_credential(&file)?;
                false
            }
            Err(StorageError::Missing) if !create => return Ok(None),
            Err(StorageError::Missing) => {
                parents = creation_parents(path, &security)?;
                create_directory(path, &security)?;
                true
            }
            Err(error) => return Err(error),
        };
        let lock = HomeLock::acquire_for_transaction(path)?;
        let marker = path.join(MARKER);
        if created {
            let mut file = open_file(
                &marker,
                GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
                0,
                CREATE_NEW,
                Some(&Security::new()?),
            )?;
            file.write_all(MARKER_BYTES).map_err(|_| StorageError::Io)?;
            file.sync_all().map_err(|_| StorageError::Io)?;
        }
        let mut marker = open_file(
            &marker,
            GENERIC_READ | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        Security::new()?.check(&marker, true)?;
        if check_object(&marker, false)?.size != MARKER_BYTES.len() as u64 {
            return Err(StorageError::UnsafePath);
        }
        if read_bytes(&mut marker)? != MARKER_BYTES {
            return Err(StorageError::UnsafePath);
        }
        drop(parents);
        Ok(Some(Self {
            path: path.to_owned(),
            lock,
            security,
            _marker: marker,
        }))
    }
    fn children(&self) -> Result<Vec<PathBuf>, StorageError> {
        self.lock.revalidate()?;
        let mut children = Vec::new();
        for entry in std::fs::read_dir(&self.path).map_err(|_| StorageError::Io)? {
            let entry = entry.map_err(|_| StorageError::Io)?;
            if entry.file_name() == MARKER || record::is_record_name(&entry.file_name()) {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_str().ok_or(StorageError::UnsafePath)?;
            if !generated_name(name) || children.len() >= 8 {
                return Err(StorageError::UnsafePath);
            }
            children.push(entry.path());
        }
        Ok(children)
    }
    fn cleanup_all(&self, root: &RootKey) -> Result<(), StorageError> {
        let children = self.children()?;
        let record = record::Onboarding::read(self, root)?;
        let Some(record) = record else {
            if !children.is_empty() {
                return Err(StorageError::TemporaryCleanupRequired);
            }
            return record::discard_unpublished(self);
        };
        if record::has_unpublished(self)? || children.iter().any(|path| *path != record.home(self))
        {
            return Err(StorageError::TemporaryCleanupRequired);
        }
        if Inventory::capture_codex()
            .map_err(|_| StorageError::Busy)?
            .has_live_clients()
            .map_err(|_| StorageError::Busy)?
        {
            return Err(StorageError::Busy);
        }
        self.cleanup_registered(&record)
    }
    fn create_home(&self, path: &Path) -> Result<(), StorageError> {
        self.lock.revalidate()?;
        create_directory(path, &self.security)?;
        // The official helper must persist only to this file backend. Managed
        // requirements are still read and can refuse this explicit setting.
        let mut config = open_file(
            &path.join("config.toml"),
            GENERIC_WRITE | READ_CONTROL,
            0,
            CREATE_NEW,
            Some(&Security::new()?),
        )?;
        config
            .write_all(b"cli_auth_credentials_store = \"file\"\n")
            .map_err(|_| StorageError::Io)?;
        config.sync_all().map_err(|_| StorageError::Io)?;
        Ok(())
    }
    fn cleanup_registered(&self, record: &record::Onboarding) -> Result<(), StorageError> {
        let path = record.home(self);
        match open_file(
            &path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => {
                check_object(&file, true)?;
                drop(file);
                self.cleanup_home(&path)?;
            }
            Err(StorageError::Missing) => {}
            Err(error) => return Err(error),
        }
        record.complete(self)
    }
    fn cleanup_home(&self, path: &Path) -> Result<(), StorageError> {
        self.lock.revalidate()?;
        if path.parent() != Some(self.path.as_path())
            || !path
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(generated_name)
        {
            return Err(StorageError::UnsafePath);
        }
        let mut nodes = Vec::new();
        pin_tree(path, &self.security, 0, &mut nodes)?;
        // Every entry was pinned and checked before any deletion. No path based
        // recursive removal, reparse traversal or ACL repair is used.
        for file in nodes {
            self.security.check_credential(&file)?;
            let info = FILE_DISPOSITION_INFO { DeleteFile: true };
            // SAFETY: exact retained DELETE handle, initialized SDK structure.
            if unsafe {
                SetFileInformationByHandle(
                    file.as_raw_handle(),
                    FileDispositionInfo,
                    (&info as *const FILE_DISPOSITION_INFO).cast(),
                    std::mem::size_of_val(&info) as u32,
                )
            } == 0
            {
                return Err(last());
            }
            drop(file);
        }
        self.lock.revalidate()
    }
}
fn creation_parents(path: &Path, security: &Security) -> Result<Vec<File>, StorageError> {
    validate_path(path)?;
    let mut paths: Vec<_> = path.ancestors().skip(1).collect();
    if !(2..=64).contains(&paths.len()) {
        return Err(StorageError::UnsafePath);
    }
    paths.reverse();
    let mut parents = Vec::new();
    for parent in paths {
        // LocalAppData is an ancestor, not a credential home: permitted Windows
        // owners/ACLs apply here. Read access with no delete sharing pins it.
        let file = open_file(
            parent,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        check_object(&file, true)?;
        security.check(&file, false)?;
        parents.push(file);
    }
    check_volume_and_sync(parents.last().ok_or(StorageError::UnsafePath)?, &parents)?;
    Ok(parents)
}
fn generated_name(name: &str) -> bool {
    name.len() == 38
        && name.starts_with("login-")
        && name.as_bytes()[6..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}
fn pin_tree(
    path: &Path,
    security: &Security,
    depth: usize,
    nodes: &mut Vec<File>,
) -> Result<(), StorageError> {
    if depth > 16 || nodes.len() >= MAX_ENTRIES {
        return Err(StorageError::InputLimit);
    }
    // This path is the registered scratch root or a child returned by read_dir
    // beneath retained parent handles. Runtime plugins can exceed MAX_PATH.
    // Extended syntax preserves the exact enumerated name; it is never accepted
    // from callers or used for vault/credential locations outside this tree.
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(StorageError::UnsafePath);
    }
    let name = wide_bounded(path, 32760)?;
    let mut extended: Vec<u16> = r"\\?\".encode_utf16().collect();
    extended.extend(name.into_iter().map(|c| {
        if c == u16::from(b'/') {
            u16::from(b'\\')
        } else {
            c
        }
    }));
    let file = open_file_encoded(
        &extended,
        FILE_READ_ATTRIBUTES | READ_CONTROL | DELETE | FILE_LIST_DIRECTORY,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )?;
    let directory = stamp(&file)?.attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    check_object(&file, directory)?;
    security.check_credential(&file)?;
    if directory {
        for entry in std::fs::read_dir(path).map_err(|_| StorageError::Io)? {
            pin_tree(
                &entry.map_err(|_| StorageError::Io)?.path(),
                security,
                depth + 1,
                nodes,
            )?;
        }
    }
    nodes.push(file);
    Ok(())
}
fn create_directory(path: &Path, security: &Security) -> Result<(), StorageError> {
    let name = wide(path)?;
    let attrs = security.attrs();
    // SAFETY: absolute verified parent and generated/fixed leaf, private DACL.
    if unsafe { CreateDirectoryW(name.as_ptr(), &attrs) } == 0 {
        return Err(last());
    }
    let file = open_file(
        path,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
        None,
    )?;
    check_object(&file, true)?;
    security.check_credential(&file)
}
fn inherited_security() -> Result<Security, StorageError> {
    let mut security = Security::new()?;
    let mut text = null_mut();
    // SAFETY: validated copied token SID, LocalFree-owned output.
    if unsafe { ConvertSidToStringSidW(security.user.as_ptr().cast_mut().cast(), &mut text) } == 0 {
        return Err(last());
    }
    let text = Allocation(text.cast());
    let mut len = 0;
    while len < 256 && unsafe { *text.0.cast::<u16>().add(len) } != 0 {
        len += 1;
    }
    if len == 256 {
        return Err(StorageError::UnsafePath);
    }
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text.0.cast::<u16>(), len) })
        .map_err(|_| StorageError::UnsafePath)?;
    let sddl: Vec<u16> = format!("O:{sid}D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)\0")
        .encode_utf16()
        .collect();
    let mut descriptor = null_mut();
    // SAFETY: generated current-user/SYSTEM descriptor; used only at creation.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(last());
    }
    security.descriptor = Allocation(descriptor);
    Ok(security)
}

/// Called once while opening the vault. Existing generated scratch homes are
/// removed only after process quiescence and retained-object validation.
pub(in crate::native) fn cleanup_abandoned(root: &RootKey) -> Result<(), StorageError> {
    let result = (|| {
        if let Some(namespace) = Namespace::open(
            &known_folder(&FOLDERID_LocalAppData)?.join(NAMESPACE),
            false,
        )? {
            namespace.cleanup_all(root)?;
        }
        Ok(())
    })();
    result.map_err(|_: StorageError| StorageError::TemporaryCleanupRequired)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Elevated Windows runners can default new objects to Administrators ownership.
    // Model an ordinary user runtime explicitly, while leaving the DACL absent
    // so these files still exercise real inheritance from the private home.
    pub(super) fn runtime_fixture(path: &Path, bytes: Option<&[u8]>, security: &Security) {
        let mut descriptor = SECURITY_DESCRIPTOR::default();
        let descriptor_ptr = (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast();
        let mut name: Vec<u16> = r"\\?\".encode_utf16().collect();
        assert!(path.is_absolute());
        name.extend(path.as_os_str().encode_wide());
        name.push(0);
        // SAFETY: stack-owned absolute descriptor and live current-user SID;
        // initialization leaves SE_DACL_PRESENT clear, never a present NULL DACL.
        unsafe {
            assert_ne!(InitializeSecurityDescriptor(descriptor_ptr, 1), 0);
            assert_ne!(
                SetSecurityDescriptorOwner(
                    descriptor_ptr,
                    security.user.as_ptr().cast_mut().cast(),
                    0,
                ),
                0
            );
        }
        let attrs = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor_ptr,
            bInheritHandle: 0,
        };
        if let Some(bytes) = bytes {
            // SAFETY: fresh fixture path, live descriptor, non-inheritable handle.
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_WRITE,
                    0,
                    &attrs,
                    CREATE_NEW,
                    FILE_ATTRIBUTE_NORMAL,
                    null_mut(),
                )
            };
            assert_ne!(handle, INVALID_HANDLE_VALUE);
            // SAFETY: transfer the single newly created handle into File.
            let mut file = unsafe { File::from_raw_handle(handle) };
            file.write_all(bytes).unwrap();
        } else {
            // SAFETY: exact new fixture directory with live owner descriptor.
            assert_ne!(unsafe { CreateDirectoryW(name.as_ptr(), &attrs) }, 0);
        }
    }
    #[test]
    fn reauthentication_mismatch_preserves_selected_generation_and_current_profile() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let mut disk = Disk::at(&sandbox.0, true).unwrap();
        let root = bootstrap(&mut disk, true).unwrap();
        let storage = Storage::create(disk, &root).unwrap();
        let mut vault = Vault { root, storage };
        let capture = |subject: &str, value: &[u8]| {
            let slot = codex_accounts_vault::ResourceId::new(0).unwrap();
            crate::Capture::new(
                codex_accounts_vault::Identity::new(
                    "SYNTHETIC".into(),
                    subject.into(),
                    "SYNTHETIC".into(),
                )
                .unwrap(),
                1,
                vec![(slot, codex_accounts_vault::ResourceShape::JsonObject, true)],
                vec![codex_accounts_vault::Resource::present(slot, value.to_vec()).unwrap()],
            )
            .unwrap()
        };
        let label = || ProfileText::new("Saved A".into()).unwrap();
        let selected = vault
            .storage
            .save_current(&vault.root, capture("A", b"{}"), Some(label()))
            .unwrap();
        let before = vault.storage.latest(selected).unwrap();
        let result = vault.save_login_capture(
            capture("B", b"{\"SYNTHETIC_B\":true}"),
            label(),
            Some(selected),
        );
        assert_eq!(result.unwrap_err().code(), "E_IDENTITY_MISMATCH");
        assert_eq!(vault.storage.latest(selected).unwrap(), before);
        assert_eq!(vault.storage.profiles().unwrap().len(), 1);
        assert_eq!(
            vault.storage.remove(&vault.root, selected),
            Err(StorageError::ActiveProfile)
        );
        assert_eq!(
            vault
                .save_login_capture(
                    capture("A", b"{\"REFRESHED_SYNTHETIC_A\":true}"),
                    label(),
                    Some(selected)
                )
                .unwrap(),
            selected
        );
        assert_ne!(vault.storage.latest(selected).unwrap(), before);
        assert_eq!(vault.storage.profiles().unwrap()[0].label, "Saved A");
    }
    #[test]
    fn absent_login_namespace_does_not_lock_its_parent() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let _parent = HomeLock::acquire_for_transaction(sandbox.0.parent().unwrap()).unwrap();
        assert!(Namespace::open(&sandbox.0, false).unwrap().is_none());
        // Existing namespaces still require all protection checks; the absence
        // shortcut cannot adopt an unmarked pre-existing directory.
        create_directory(&sandbox.0, &inherited_security().unwrap()).unwrap();
        assert!(Namespace::open(&sandbox.0, false).is_err());
    }
    #[test]
    fn browser_authority_is_fixed_and_never_accepts_shell_or_redirect_schemes() {
        assert!(approved_url("https://auth.openai.com/oauth/authorize?state=SYNTHETIC&redirect_uri=http%3A%2F%2Flocalhost%3A1455"));
        for url in [
            "http://auth.openai.com/oauth/authorize?x=1",
            "https://auth.openai.com.evil.invalid/oauth/authorize?x=1",
            "https://auth.openai.com@evil.invalid/oauth/authorize?x=1",
            "file:///C:/example",
            "https://auth.openai.com/oauth/authorize?x=1\n",
            "https://auth.openai.com/oauth/authorize?x=1#fragment",
        ] {
            assert!(!approved_url(url));
        }
        assert!(generated_name("login-0123456789abcdef0123456789abcdef"));
        assert!(!generated_name("login-../auth.json"));
    }
    #[test]
    #[ignore = "explicit official runtime login-start/cancel and scratch cleanup; no browser or real account"]
    fn official_login_home_can_be_cleaned_after_helper_exit() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let namespace = Namespace::open(&sandbox.0, true).unwrap().unwrap();
        let root = RootKey::generate().unwrap();
        let record = record::Onboarding::begin(&namespace, &root, None).unwrap();
        let path = record.home(&namespace);
        namespace.create_home(&path).unwrap();
        let installation = codex_accounts_discovery::OfficialInstallation::discover().unwrap();
        let registered_core = installation.contained_core() && installation.provisioned_core();
        let mut connection =
            RuntimeConnection::open(&installation, &path, true, registered_core).unwrap();
        let session = connection.session.as_mut().unwrap();
        session.initialize().unwrap();
        assert!(matches!(
            next(session).unwrap(),
            PipeEvent::Initialized { .. }
        ));
        session.initialized().unwrap();
        assert!(matches!(
            session.next_event().unwrap(),
            PipeEvent::Written(_)
        ));
        session.read_config_requirements().unwrap();
        let PipeEvent::Requirements(policy) = next(session).unwrap() else {
            panic!("requirements missing")
        };
        session.read_config().unwrap();
        let PipeEvent::Config(config) = next(session).unwrap() else {
            panic!("configuration missing")
        };
        runtime_context::compatible(config.config, policy.requirements).unwrap();
        session.start_login().unwrap();
        assert!(matches!(
            next(session).unwrap(),
            PipeEvent::LoginStarted { .. }
        ));
        connection.cancellation.request();
        loop {
            match next(session).unwrap() {
                PipeEvent::CancellationAcknowledged => break,
                PipeEvent::CancellationProgress | PipeEvent::LoginCompletionBuffered => {}
                _ => panic!("unexpected cancellation event"),
            }
        }
        connection.close_and_wait().unwrap();
        assert!(!path.join("auth.json").exists());
        namespace
            .cleanup_registered(&record)
            .expect("official helper scratch must be removable after exit");
        drop(record);
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_supports_long_runtime_plugin_paths() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let namespace = Namespace::open(&sandbox.0, true).unwrap().unwrap();
        let root = RootKey::generate().unwrap();
        let record = record::Onboarding::begin(&namespace, &root, None).unwrap();
        let path = record.home(&namespace);
        namespace.create_home(&path).unwrap();
        let plugins = path.join("synthetic-plugin-name-".repeat(5));
        let assets = plugins.join("synthetic-plugin-assets-".repeat(4));
        runtime_fixture(&plugins, None, &namespace.security);
        runtime_fixture(&assets, None, &namespace.security);
        let asset = assets.join("synthetic-asset.txt");
        assert!(asset.as_os_str().encode_wide().count() > 260);
        runtime_fixture(&asset, Some(b"SYNTHETIC"), &namespace.security);
        namespace
            .cleanup_registered(&record)
            .expect("long runtime plugin paths must be cleaned");
        drop(record);
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_supports_a_populated_runtime_home() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let namespace = Namespace::open(&sandbox.0, true).unwrap().unwrap();
        let root = RootKey::generate().unwrap();
        let record = record::Onboarding::begin(&namespace, &root, None).unwrap();
        let path = record.home(&namespace);
        namespace.create_home(&path).unwrap();
        // A real completed browser flow left 7,873 ordinary entries. Empty-home
        // cancellation finishes too early to represent that populated layout.
        let assets = path.join("synthetic-assets");
        create_directory(&assets, &namespace.security).unwrap();
        for index in 0..8192 {
            runtime_fixture(
                &assets.join(format!("synthetic-{index}")),
                Some(b"SYNTHETIC"),
                &namespace.security,
            );
        }
        namespace
            .cleanup_registered(&record)
            .expect("populated runtime home must be cleaned");
        drop(record);
        assert!(!path.exists());
        assert!(record::Onboarding::read(&namespace, &root)
            .unwrap()
            .is_none());
    }

    #[test]
    fn synthetic_login_home_is_private_and_cleanup_refuses_live_writers() {
        let sandbox = super::super::super::tests::Sandbox::new();
        let namespace = Namespace::open(&sandbox.0, true).unwrap().unwrap();
        let root = RootKey::generate().unwrap();
        let record = record::Onboarding::begin(&namespace, &root, None).unwrap();
        let path = record.home(&namespace);
        namespace.create_home(&path).unwrap();
        let auth = path.join("auth.json");
        runtime_fixture(
            &auth,
            Some(b"{\"SYNTHETIC_ONLY\":true}"),
            &namespace.security,
        );
        let target = Target::open_file_backend(&path).unwrap();
        assert_eq!(
            &**target.read(0).unwrap().unwrap(),
            b"{\"SYNTHETIC_ONLY\":true}"
        );
        drop(target);
        let writer = open_file(
            &auth,
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            None,
        )
        .unwrap();
        assert!(namespace.cleanup_registered(&record).is_err());
        assert!(auth.exists());
        assert!(path.join("config.toml").exists());
        drop(writer);
        drop(record);
        // The authenticated record survives the failure and can be reopened.
        let record = record::Onboarding::read(&namespace, &root)
            .unwrap()
            .unwrap();
        namespace.cleanup_registered(&record).unwrap();
        drop(record);
        assert!(!path.exists());
        let other = sandbox.0.parent().unwrap().join("synthetic-outside.txt");
        std::fs::write(&other, b"SYNTHETIC_OUTSIDE_UNCHANGED").unwrap();
        let record = record::Onboarding::begin(&namespace, &root, None).unwrap();
        let path = record.home(&namespace);
        namespace.create_home(&path).unwrap();
        let linked = path.join("synthetic-link");
        std::fs::hard_link(&other, &linked).unwrap();
        assert!(namespace.cleanup_registered(&record).is_err());
        assert!(path.join("config.toml").exists());
        assert_eq!(
            std::fs::read(&other).unwrap(),
            b"SYNTHETIC_OUTSIDE_UNCHANGED"
        );
        std::fs::remove_file(linked).unwrap();
        namespace.cleanup_registered(&record).unwrap();
        drop(record);
        drop(namespace);
        let reopened = Namespace::open(&sandbox.0, false).unwrap().unwrap();
        assert!(reopened.children().unwrap().is_empty());
    }
}
