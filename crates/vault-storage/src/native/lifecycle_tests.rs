//! Native effects against fresh synthetic directories and exclusively owned children.
//! No installed client, real account, user creation or existing ACL change is used.
use super::super::*;
use super::{
    helper::OwnedFamily,
    home::HomeLock,
    process::{Inventory, ObservedProcess, ObservedTree},
};
use crate::lifecycle_model::Fault;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows_sys::Win32::System::{LibraryLoader::GetModuleHandleW, Threading::*};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CHILD_TEST: &str = "native::lifecycle::tests::native_lifecycle_child";
const MARKER: &[u8] = b"SYNTHETIC_CA04B_ONLY";
static REFUSE_CLOSE: AtomicBool = AtomicBool::new(false);
static CLOSE_RECEIVED: AtomicBool = AtomicBool::new(false);

fn home() -> super::super::tests::Sandbox {
    let mut fixture = super::super::tests::Sandbox::new();
    fixture.0.set_file_name("ca04b-home-測試");
    let security = Security::new().unwrap();
    assert_ne!(
        unsafe { CreateDirectoryW(wide(&fixture.0).unwrap().as_ptr(), &security.attrs()) },
        0
    );
    let mut marker = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fixture.0.join("marker"))
        .unwrap();
    marker.write_all(MARKER).unwrap();
    fixture
}
fn signal(root: &Path, name: &str) {
    std::fs::write(root.join(name), b"SYNTHETIC").unwrap();
}
fn wait_marker(root: &Path, name: &str) {
    let start = Instant::now();
    while !root.join(name).exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "synthetic child handshake timed out"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn child_wait(root: &Path, name: &str) {
    let start = Instant::now();
    while !root.join(name).exists() && start.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct TestChild {
    process: Token,
    thread: Token,
    pid: u32,
}
impl TestChild {
    fn suspended(root: &Path, mode: &str) -> Self {
        assert!([
            "lock",
            "wait",
            "window-close",
            "window-refuse",
            "family",
            "tree",
            "late"
        ]
        .contains(&mode));
        let executable = std::env::current_exe().unwrap();
        let executable_text = executable.to_str().unwrap();
        assert!(!executable_text.contains('"'));
        let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
        let mut command: Vec<u16> =
            format!("\"{executable_text}\" --exact {CHILD_TEST} --nocapture\0")
                .encode_utf16()
                .collect();
        let mut environment = BTreeMap::<String, OsString>::new();
        for name in [
            "SYSTEMROOT",
            "WINDIR",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "LOCALAPPDATA",
        ] {
            if let Some(value) = std::env::var_os(name) {
                environment.insert(name.to_owned(), value);
            }
        }
        assert!(environment.contains_key("SYSTEMROOT"));
        environment.insert("CA04B_TEST_MODE".to_owned(), mode.into());
        environment.insert("CA04B_TEST_ROOT".to_owned(), root.as_os_str().to_owned());
        environment.insert("RUST_TEST_THREADS".to_owned(), "1".into());
        let mut block = Vec::<u16>::new();
        for (name, value) in environment {
            block.extend(name.encode_utf16());
            block.push(u16::from(b'='));
            let value: Vec<_> = value.encode_wide().collect();
            assert!(!value.contains(&0));
            block.extend(value);
            block.push(0);
        }
        block.push(0);
        assert!(block.len() < 32767);
        let startup = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: this test creates only its own absolute test executable, suspended,
        // with fixed arguments, bounded private environment, no shell and no inheritance.
        assert_ne!(
            unsafe {
                CreateProcessW(
                    application.as_ptr(),
                    command.as_mut_ptr(),
                    null(),
                    null(),
                    0,
                    CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
                    block.as_ptr().cast(),
                    wide(root).unwrap().as_ptr(),
                    &startup,
                    &mut info,
                )
            },
            0,
            "create exclusively owned synthetic child"
        );
        Self {
            process: Token(info.hProcess),
            thread: Token(info.hThread),
            pid: info.dwProcessId,
        }
    }
    fn resume(&self) {
        assert_eq!(
            unsafe { ResumeThread(self.thread.0) },
            1,
            "resume the one owned suspension"
        );
    }
    fn wait(&self) {
        assert_eq!(
            unsafe { WaitForSingleObject(self.process.0, 5000) },
            WAIT_OBJECT_0,
            "owned test child must actually exit"
        );
        let mut code = 0;
        assert_ne!(unsafe { GetExitCodeProcess(self.process.0, &mut code) }, 0);
        assert_eq!(code, 0, "synthetic child failed");
    }
}
impl Drop for TestChild {
    fn drop(&mut self) {
        // Test cleanup ONLY: this is CreateProcessW's original handle, never a
        // discovered process/PID. None of this helper is compiled into the library.
        if unsafe { WaitForSingleObject(self.process.0, 0) } != WAIT_OBJECT_0 {
            unsafe { TerminateProcess(self.process.0, 1) };
            unsafe { WaitForSingleObject(self.process.0, 5000) };
        }
    }
}
struct TestFamily {
    child: TestChild,
    family: OwnedFamily,
}
impl TestFamily {
    fn start(root: &Path, mode: &str) -> Self {
        let child = TestChild::suspended(root, mode);
        let family = OwnedFamily::for_suspended_test_child(child.process.0, child.pid).unwrap();
        child.resume();
        Self { child, family }
    }
}
impl Drop for TestFamily {
    fn drop(&mut self) {
        // Bounded graceful/owned-job cleanup before the enclosing sandbox is removed.
        // Errors remain test failure evidence; destructor cleanup is not a pass.
        let _ = self.family.short_test_shutdown();
    }
}

unsafe extern "system" fn window_proc(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if message == WM_CLOSE {
        CLOSE_RECEIVED.store(true, Ordering::Relaxed);
    }
    match message {
        WM_CLOSE if REFUSE_CLOSE.load(Ordering::Relaxed) => 0,
        WM_CLOSE | WM_TIMER => {
            unsafe { DestroyWindow(window) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(window, message, w, l) },
    }
}
fn run_window(root: &Path, refuse: bool) {
    REFUSE_CLOSE.store(refuse, Ordering::Relaxed);
    let class: Vec<u16> = "CA04B_SYNTHETIC_WINDOW\0".encode_utf16().collect();
    let instance = unsafe { GetModuleHandleW(null()) };
    assert!(!instance.is_null());
    let definition = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        ..Default::default()
    };
    assert_ne!(unsafe { RegisterClassW(&definition) }, 0);
    let window = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            100,
            100,
            null_mut(),
            null_mut(),
            instance,
            null(),
        )
    };
    assert!(!window.is_null());
    // This hidden, test-owned window has a finite lifetime even if its parent dies.
    assert_ne!(unsafe { SetTimer(window, 1, 20_000, None) }, 0);
    signal(root, "ready-window");
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        assert!(result >= 0);
        if result == 0 {
            break;
        }
        unsafe { TranslateMessage(&message) };
        unsafe { DispatchMessageW(&message) };
        if CLOSE_RECEIVED.swap(false, Ordering::Relaxed) {
            signal(root, "close-received");
        }
    }
}

#[test]
fn native_lifecycle_child() {
    let Some(mode) = std::env::var_os("CA04B_TEST_MODE") else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("CA04B_TEST_ROOT").expect("synthetic root only"));
    validate_path(&root).unwrap();
    assert!(root
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("ca03c-test-"));
    assert_eq!(std::fs::read(root.join("marker")).unwrap(), MARKER);
    match mode.to_str().unwrap() {
        "lock" => {
            let _lock = HomeLock::acquire(&root).unwrap();
            signal(&root, "ready-lock");
            child_wait(&root, "finish-lock");
        }
        "wait" => {
            signal(&root, "ready-wait");
            child_wait(&root, "finish-wait");
        }
        "window-close" => run_window(&root, false),
        "window-refuse" => run_window(&root, true),
        "family" | "tree" => {
            let child = TestChild::suspended(&root, "late");
            child.resume();
            std::fs::write(root.join("descendant.pid"), child.pid.to_string()).unwrap();
            // The OS closes these handles on root exit. Do not kill the descendant
            // in this synthetic root: the parent must prove it remains in the job.
            std::mem::forget(child);
            wait_marker(&root, "ready-late");
            signal(&root, "ready-family");
            if mode == "tree" {
                child_wait(&root, "finish-parent");
            }
        }
        "late" => {
            signal(&root, "ready-late");
            child_wait(&root, "finish-late");
            std::fs::write(root.join("late-write"), b"SYNTHETIC_LATE_REFRESH").unwrap();
        }
        _ => panic!("unknown synthetic child mode"),
    }
}

#[test]
fn ca04b_home_mutex_unicode_alias_and_recursive_contention() {
    let fixture = home();
    let lock = HomeLock::acquire(&fixture.0).unwrap();
    lock.revalidate().unwrap();
    assert!(!lock.abandoned());
    assert!(matches!(
        HomeLock::acquire(&fixture.0),
        Err(StorageError::Busy)
    ));
    let alias = fixture.0.with_file_name("CA04B-HOME-測試");
    assert!(matches!(HomeLock::acquire(&alias), Err(StorageError::Busy)));
    assert_eq!(format!("{lock:?}"), "HomeLock([REDACTED])");
    let other = home();
    HomeLock::acquire(&other.0).unwrap();
    drop(lock);
    HomeLock::acquire(&fixture.0).unwrap();
}
#[test]
fn ca04b_home_mutex_process_contention_and_exit_release() {
    let fixture = home();
    let child = TestChild::suspended(&fixture.0, "lock");
    child.resume();
    wait_marker(&fixture.0, "ready-lock");
    assert!(matches!(
        HomeLock::acquire(&fixture.0),
        Err(StorageError::Busy)
    ));
    signal(&fixture.0, "finish-lock");
    child.wait();
    HomeLock::acquire(&fixture.0).unwrap().revalidate().unwrap();
    // An actual process interruption releases the lifetime lock too; it does not
    // say the transaction is clean or skip the later journal recovery requirement.
    std::fs::remove_file(fixture.0.join("ready-lock")).unwrap();
    std::fs::remove_file(fixture.0.join("finish-lock")).unwrap();
    let interrupted = TestChild::suspended(&fixture.0, "lock");
    interrupted.resume();
    wait_marker(&fixture.0, "ready-lock");
    drop(interrupted);
    HomeLock::acquire(&fixture.0).unwrap().revalidate().unwrap();
}
#[test]
fn ca04b_home_replacement_and_non_directory_paths_refuse() {
    let fixture = home();
    // Construct the hazard before taking the restrictive directory handles.
    // The lock must not be weakened to permit later fixture setup.
    std::fs::hard_link(fixture.0.join("marker"), fixture.0.join("linked")).unwrap();
    let lock = HomeLock::acquire(&fixture.0).unwrap();
    assert!(std::fs::rename(&fixture.0, fixture.0.with_extension("moved")).is_err());
    let parent = fixture.0.parent().unwrap();
    assert!(std::fs::rename(parent, parent.with_extension("moved")).is_err());
    lock.revalidate().unwrap();
    for path in [
        fixture.0.join("marker"),
        fixture.0.join(".."),
        PathBuf::from(r"\\server\share\SYNTHETIC"),
        PathBuf::from(r"C:\SYNTHETIC\..\home"),
    ] {
        assert!(HomeLock::acquire(&path).is_err());
    }
    assert!(HomeLock::acquire(&fixture.0.join("linked")).is_err());
}
#[test]
fn ca04b_system_owned_directory_is_not_a_current_user_home() {
    let root = PathBuf::from(std::env::var_os("SYSTEMROOT").expect("standard Windows runner"));
    let security = Security::new().unwrap();
    let file = open_file(
        &root,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let mut owner = null_mut();
    let mut descriptor = null_mut();
    assert_eq!(
        unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descriptor,
            )
        },
        0
    );
    let _descriptor = Allocation(descriptor);
    assert!(
        sid_copy(owner).unwrap() != security.user,
        "requires a runner identity distinct from the system owner"
    );
    assert!(matches!(
        HomeLock::acquire(&root),
        Err(StorageError::UnsafePath) | Err(StorageError::Busy)
    ));
}
#[test]
fn ca04b_home_junction_root_and_ancestor_refuse() {
    use windows_sys::Win32::System::{
        Ioctl::FSCTL_SET_REPARSE_POINT, SystemServices::IO_REPARSE_TAG_MOUNT_POINT,
        IO::DeviceIoControl,
    };
    let fixture = home();
    let junction_path = fixture.0.parent().unwrap().join("synthetic-junction");
    let security = Security::new().unwrap();
    assert_ne!(
        unsafe { CreateDirectoryW(wide(&junction_path).unwrap().as_ptr(), &security.attrs()) },
        0
    );
    let junction = open_file(&junction_path, GENERIC_WRITE, 0, OPEN_EXISTING, None).unwrap();
    let substitute: Vec<u16> = format!("\\??\\{}", fixture.0.display())
        .encode_utf16()
        .collect();
    let print: Vec<u16> = fixture.0.as_os_str().encode_wide().collect();
    let sub_len = (substitute.len() * 2) as u16;
    let print_len = (print.len() * 2) as u16;
    let data_len = 8 + sub_len + 2 + print_len + 2;
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
    buffer.extend_from_slice(&data_len.to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    for value in [0, sub_len, sub_len + 2, print_len] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in substitute.into_iter().chain([0]).chain(print).chain([0]) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let mut returned = 0;
    assert_ne!(
        unsafe {
            DeviceIoControl(
                junction.as_raw_handle(),
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                null_mut(),
                0,
                &mut returned,
                null_mut(),
            )
        },
        0
    );
    drop(junction);
    let root_result = HomeLock::acquire(&junction_path);
    let descendant_result = HomeLock::acquire(&junction_path.join("absent"));
    // Unlink only the synthetic junction before the sandbox's recursive cleanup.
    std::fs::remove_dir(&junction_path).unwrap();
    assert!(matches!(root_result, Err(StorageError::UnsafePath)));
    assert!(matches!(descendant_result, Err(StorageError::UnsafePath)));
    assert_eq!(std::fs::read(fixture.0.join("marker")).unwrap(), MARKER);
    assert!(!fixture.0.join("absent").exists());
}
#[test]
fn ca04b_process_start_identity_and_exit_are_handle_bound() {
    let fixture = home();
    let child = TestChild::suspended(&fixture.0, "wait");
    child.resume();
    wait_marker(&fixture.0, "ready-wait");
    let process = ObservedProcess::open(child.pid).unwrap();
    let key = process.key();
    let mut reused = key;
    reused.created += 1;
    assert_eq!(process.ensure_live(reused), Err(Fault::Changed));
    assert_eq!(process.request_normal_quit(reused), Err(Fault::Changed));
    assert_eq!(
        process.wait_exit(Duration::from_millis(5)),
        Err(Fault::Timeout)
    );
    assert_eq!(
        process.wait_exit(Duration::from_secs(31)),
        Err(Fault::Bound)
    );
    assert!(process.life(0).unwrap().exited.is_none());
    assert_eq!(format!("{process:?}"), "ObservedProcess([REDACTED])");
    signal(&fixture.0, "finish-wait");
    child.wait();
    assert!(process.signalled().unwrap());
    assert!(process.life(0).unwrap().exited.is_some());
    assert_eq!(process.ensure_live(key), Err(Fault::Disappeared));
}
#[test]
fn ca04b_normal_quit_requires_actual_process_exit() {
    let fixture = home();
    let child = TestChild::suspended(&fixture.0, "window-close");
    child.resume();
    wait_marker(&fixture.0, "ready-window");
    let observed = ObservedProcess::open(child.pid).unwrap();
    let delivered = observed.request_normal_quit(observed.key()).unwrap();
    // Windows can create auxiliary top-level windows in the same process. The
    // actual fixture's WM_CLOSE receipt, not an assumed HWND count, is the oracle.
    assert!((1..=crate::lifecycle_model::MAX_WINDOWS).contains(&delivered));
    wait_marker(&fixture.0, "close-received");
    observed.wait_exit(Duration::from_secs(5)).unwrap();
    assert!(observed.signalled().unwrap());
    child.wait();
}
#[test]
fn ca04b_refused_normal_quit_never_terminates_observed_process() {
    let fixture = home();
    let child = TestChild::suspended(&fixture.0, "window-refuse");
    child.resume();
    wait_marker(&fixture.0, "ready-window");
    let observed = ObservedProcess::open(child.pid).unwrap();
    let delivered = observed.request_normal_quit(observed.key()).unwrap();
    // Windows can create auxiliary top-level windows in the same process. The
    // actual fixture's WM_CLOSE receipt, not an assumed HWND count, is the oracle.
    assert!((1..=crate::lifecycle_model::MAX_WINDOWS).contains(&delivered));
    wait_marker(&fixture.0, "close-received");
    assert_eq!(
        observed.wait_exit(Duration::from_millis(60)),
        Err(Fault::Timeout)
    );
    assert!(!observed.signalled().unwrap());
    // Only TestChild's original creator handle cleans this test-owned process up.
}
#[test]
fn ca04b_owned_job_waits_for_late_descendant_and_final_write() {
    let fixture = home();
    let mut owner = TestFamily::start(&fixture.0, "family");
    wait_marker(&fixture.0, "ready-family");
    owner.child.wait();
    assert!(
        !owner.family.poll_exit().unwrap(),
        "parent exit is not family exit"
    );
    assert!(!fixture.0.join("late-write").exists());
    signal(&fixture.0, "finish-late");
    let result = owner.family.shutdown_after_input_closed().unwrap();
    assert!(!result.forced);
    assert!(result.observed_members >= 1);
    assert!(owner.family.poll_exit().unwrap());
    assert_eq!(
        std::fs::read(fixture.0.join("late-write")).unwrap(),
        b"SYNTHETIC_LATE_REFRESH"
    );
}
#[test]
fn ca04b_owned_timeout_waits_after_termination_and_protects_unrelated_child() {
    let fixture = home();
    let unrelated = TestChild::suspended(&fixture.0, "wait");
    unrelated.resume();
    wait_marker(&fixture.0, "ready-wait");
    let unrelated_observation = ObservedProcess::open(unrelated.pid).unwrap();
    let mut owner = TestFamily::start(&fixture.0, "family");
    wait_marker(&fixture.0, "ready-family");
    owner.child.wait();
    let start = Instant::now();
    let result = owner.family.shutdown_after_input_closed().unwrap();
    assert!(result.forced);
    assert!(start.elapsed() >= Duration::from_secs(5));
    assert!(owner.family.poll_exit().unwrap());
    assert!(!unrelated_observation.signalled().unwrap());
    signal(&fixture.0, "finish-wait");
    unrelated.wait();
}
#[test]
fn ca04b_snapshot_and_known_descendants_never_become_home_authority() {
    let fixture = home();
    let mut owner = TestFamily::start(&fixture.0, "tree");
    wait_marker(&fixture.0, "ready-family");
    let pid: u32 = std::fs::read_to_string(fixture.0.join("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let child = ObservedProcess::open(pid).unwrap();
    let key = child.key();
    let root = ObservedProcess::open(owner.child.pid).unwrap();
    let mut tree = ObservedTree::new(root);
    let inventory = Inventory::capture().unwrap();
    assert!(inventory.shared_home_readiness().is_err());
    tree.refresh(inventory).unwrap();
    assert!(tree.contains(key));
    assert!(tree.shared_home_readiness().is_err());
    signal(&fixture.0, "finish-parent");
    owner.child.wait();
    assert!(!child.signalled().unwrap());
    assert!(tree.observed_members_exited().is_err() || !tree.observed_members_exited().unwrap());
    signal(&fixture.0, "finish-late");
    owner.family.shutdown_after_input_closed().unwrap();
    assert!(child.signalled().unwrap());
    // Even complete observed exits do not fill in missing broker/CLI/home rules.
    assert!(tree.shared_home_readiness().is_err());
}
