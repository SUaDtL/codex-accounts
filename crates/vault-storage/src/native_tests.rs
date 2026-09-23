use super::*;
use crate::{Capture, ProfileText};
use codex_accounts_vault::{Identity, Resource, ResourceId, ResourceShape};
use std::process::{Command, Stdio};

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let parent = local_root()
            .expect("known folder")
            .parent()
            .unwrap()
            .to_owned();
        let suffix: String = random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Self(parent.join(format!("ca03c-test-{suffix}")))
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).expect("remove only owned synthetic test directory");
        }
    }
}
fn cap() -> Capture {
    let id = ResourceId::new(0).unwrap();
    Capture::new(
        Identity::new(
            "SYNTHETIC".into(),
            "SYNTHETIC_SUBJECT".into(),
            "SYNTHETIC_WORKSPACE".into(),
        )
        .unwrap(),
        1,
        vec![(id, ResourceShape::JsonObject, true)],
        vec![Resource::present(
            id,
            b"{ \"SYNTHETIC\": \"NOT_A_REAL_CREDENTIAL\" }\r\n".to_vec(),
        )
        .unwrap()],
    )
    .unwrap()
}
fn text() -> ProfileText {
    ProfileText::new("SYNTHETIC_LABEL".into()).unwrap()
}

#[test]
fn native_create_bootstrap_reopen_and_ciphertext_only() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).expect("safe synthetic native root");
    let root = bootstrap(&mut d, true).unwrap();
    let mut s = Storage::create(d, &root).unwrap();
    let (p, g) = s.add(&root, text(), text(), cap()).unwrap();
    let expected = s.read_latest(&root, p).unwrap();
    drop(s);
    drop(root);
    let mut d = Disk::at(&dir.0, false).unwrap();
    let root = bootstrap(&mut d, false).unwrap();
    let s = Storage::open(d, &root).unwrap();
    assert_eq!(s.latest(p).unwrap(), g);
    assert_eq!(
        s.read_latest(&root, p)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        expected
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes()
    );
    for n in s.disk.list().unwrap() {
        if n == "lock" {
            continue;
        }
        let b = s.disk.read(&n).unwrap().unwrap();
        assert!(!b.windows(9).any(|v| v == b"SYNTHETIC"));
    }
}
#[test]
fn native_kernel_lock_and_parent_replacement_refuse() {
    let dir = Sandbox::new();
    let d = Disk::at(&dir.0, true).unwrap();
    assert!(matches!(Disk::at(&dir.0, false), Err(StorageError::Busy)));
    assert!(std::fs::rename(&dir.0, dir.0.with_extension("moved")).is_err());
    drop(d);
    Disk::at(&dir.0, false).unwrap();
}
#[test]
fn native_hardlinks_and_sharing_conflicts_refuse() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    d.stage("synthetic.bin", b"SYNTHETIC_ONLY").unwrap();
    d.publish("synthetic.bin", None, b"SYNTHETIC_ONLY").unwrap();
    std::fs::hard_link(dir.0.join("synthetic.bin"), dir.0.join("linked.bin")).unwrap();
    assert!(matches!(
        d.read("synthetic.bin"),
        Err(StorageError::UnsafePath)
    ));
    std::fs::remove_file(dir.0.join("linked.bin")).unwrap();
    let held = d
        .leaf("synthetic.bin", GENERIC_READ | GENERIC_WRITE)
        .unwrap();
    assert!(matches!(d.read("synthetic.bin"), Err(StorageError::Busy)));
    drop(held);
}
#[test]
fn native_same_directory_replace_compare_and_delete() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    d.stage("synthetic.bin", b"SYNTHETIC_ONE").unwrap();
    d.publish("synthetic.bin", None, b"SYNTHETIC_ONE").unwrap();
    d.stage("synthetic.bin", b"SYNTHETIC_TWO").unwrap();
    assert_eq!(
        d.publish("synthetic.bin", Some(b"WRONG"), b"SYNTHETIC_TWO"),
        Err(StorageError::ExternalChange)
    );
    assert_eq!(d.read("synthetic.bin").unwrap().unwrap(), b"SYNTHETIC_ONE");
    d.publish("synthetic.bin", Some(b"SYNTHETIC_ONE"), b"SYNTHETIC_TWO")
        .unwrap();
    assert_eq!(
        d.erase("synthetic.bin", b"WRONG"),
        Err(StorageError::ExternalChange)
    );
    d.erase("synthetic.bin", b"SYNTHETIC_TWO").unwrap();
    assert!(d.read("synthetic.bin").unwrap().is_none());
}
#[test]
fn native_partial_bootstrap_only_recovers_the_original_protected_key() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    let key = RootKey::generate().unwrap();
    let id = key.identifier();
    let wrapped = key.protect().unwrap();
    d.stage("key.cakp", wrapped.as_bytes()).unwrap();
    drop(d);
    let mut d = Disk::at(&dir.0, false).unwrap();
    assert_eq!(bootstrap(&mut d, false).unwrap().identifier(), id);
    drop(d);
    std::fs::remove_file(dir.0.join("key.cakp")).unwrap();
    let mut d = Disk::at(&dir.0, false).unwrap();
    assert!(matches!(
        bootstrap(&mut d, false),
        Err(StorageError::KeyUnavailable)
    ));
    assert!(d.read("key.cakp").unwrap().is_none());
}
#[test]
fn native_corrupt_key_and_existing_root_never_regenerate() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    let _ = bootstrap(&mut d, true).unwrap();
    drop(d);
    assert!(matches!(
        Disk::at(&dir.0, true),
        Err(StorageError::AlreadyExists)
    ));
    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(dir.0.join("key.cakp"))
        .unwrap();
    drop(f);
    let mut d = Disk::at(&dir.0, false).unwrap();
    assert!(bootstrap(&mut d, false).is_err());
    assert!(d.read("key.cakp").unwrap().unwrap().is_empty());
}
#[test]
fn native_network_sync_traversal_and_device_paths_rejected() {
    for p in [
        r"\\host\share\vault",
        r"\\?\C:\vault",
        r"C:\OneDrive\vault",
        r"C:\Dropbox\vault",
        r"C:\base\..\vault",
        r"C:\base\CON",
        r"relative\vault",
        r"C:\name:stream",
    ] {
        assert_eq!(validate_path(Path::new(p)), Err(StorageError::UnsafePath));
    }
    let dir = Sandbox::new();
    let unicode = Sandbox(dir.0.with_file_name(format!(
        "{}-Ω",
        dir.0.file_name().unwrap().to_str().unwrap()
    )));
    let d = Disk::at(&unicode.0, true).unwrap();
    drop(d);
}
#[test]
fn native_broad_leaf_acl_is_refused_without_repair() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    d.stage("synthetic.bin", b"SYNTHETIC").unwrap();
    d.publish("synthetic.bin", None, b"SYNTHETIC").unwrap();
    let path = wide(&dir.0.join("synthetic.bin")).unwrap();
    let sddl: Vec<_> = "D:P(A;;FA;;;WD)\0".encode_utf16().collect();
    let mut sd = null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        },
        0
    );
    let sd = Allocation(sd);
    let mut present = 0;
    let mut acl = null_mut();
    let mut defaulted = 0;
    assert_ne!(
        unsafe { GetSecurityDescriptorDacl(sd.0, &mut present, &mut acl, &mut defaulted) },
        0
    );
    // Deliberately corrupt ONLY this created synthetic file's ACL. No production
    // path repairs or permission manipulation are exposed by the library.
    assert_eq!(
        unsafe {
            SetNamedSecurityInfoW(
                path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl,
                null_mut(),
            )
        },
        0
    );
    assert!(matches!(
        d.read("synthetic.bin"),
        Err(StorageError::UnsafePath)
    ));
}

// A child is the SAME test binary, never a shell, Codex, or arbitrary executable.
// Test-only environment is neither compiled into production nor an authority flag.
#[test]
fn native_restart_child() {
    let Ok(path) = std::env::var("CA03C_SYNTHETIC_CHILD_ROOT") else {
        return;
    };
    let p = PathBuf::from(path);
    assert!(p
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("ca03c-test-"));
    let mut d = Disk::at(&p, false).unwrap();
    let root = bootstrap(&mut d, false).unwrap();
    let mut s = Storage::open(d, &root).unwrap();
    s.add(&root, text(), text(), cap()).unwrap();
    // Exit without destructors proves kernel lock release and disk-only recovery.
    std::process::exit(73);
}
#[test]
fn native_process_restart_reopens_persisted_state_and_releases_lock() {
    let dir = Sandbox::new();
    let mut d = Disk::at(&dir.0, true).unwrap();
    let root = bootstrap(&mut d, true).unwrap();
    let s = Storage::create(d, &root).unwrap();
    drop(s);
    drop(root);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native::tests::native_restart_child",
            "--nocapture",
        ])
        .env("CA03C_SYNTHETIC_CHILD_ROOT", &dir.0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned synthetic child timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(73));
    let mut d = Disk::at(&dir.0, false).unwrap();
    let root = bootstrap(&mut d, false).unwrap();
    let s = Storage::open(d, &root).unwrap();
    assert_eq!(s.recovery(), crate::Recovery::Clean);
}

struct CrashDisk {
    disk: Disk,
    at: usize,
    count: usize,
}
impl CrashDisk {
    fn edge(&mut self) {
        self.count += 1;
        if self.count == self.at {
            std::process::exit(73);
        }
    }
}
impl Files for CrashDisk {
    fn read(&self, n: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.disk.read(n)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.disk.list()
    }
    fn stage(&mut self, n: &str, b: &[u8]) -> Result<(), StorageError> {
        self.edge();
        self.disk.stage(n, b)?;
        self.edge();
        Ok(())
    }
    fn publish(&mut self, n: &str, e: Option<&[u8]>, b: &[u8]) -> Result<(), StorageError> {
        self.edge();
        self.disk.publish(n, e, b)?;
        self.edge();
        Ok(())
    }
    fn erase(&mut self, n: &str, e: &[u8]) -> Result<(), StorageError> {
        self.edge();
        self.disk.erase(n, e)?;
        self.edge();
        Ok(())
    }
}
#[test]
fn native_fault_child() {
    let Ok(path) = std::env::var("CA03C_SYNTHETIC_FAULT_ROOT") else {
        return;
    };
    let path = PathBuf::from(path);
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("ca03c-test-"));
    let at: usize = std::env::var("CA03C_SYNTHETIC_FAULT_AT")
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=20).contains(&at));
    let disk = Disk::at(&path, false).unwrap();
    let mut disk = CrashDisk { disk, at, count: 0 };
    let root = bootstrap(&mut disk, false).unwrap();
    let mut s = Storage::open(disk, &root).unwrap();
    s.add(&root, text(), text(), cap()).unwrap();
    panic!("every selected native crash edge must execute");
}
#[test]
fn native_restart_at_each_durable_commit_boundary() {
    for at in 1..=20 {
        let dir = Sandbox::new();
        let mut disk = Disk::at(&dir.0, true).unwrap();
        let root = bootstrap(&mut disk, true).unwrap();
        drop(Storage::create(disk, &root).unwrap());
        drop(root);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "native::tests::native_fault_child"])
            .env("CA03C_SYNTHETIC_FAULT_ROOT", &dir.0)
            .env("CA03C_SYNTHETIC_FAULT_AT", at.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break s;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("owned synthetic fault child timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert_eq!(status.code(), Some(73), "native checkpoint {at}");
        let mut disk = Disk::at(&dir.0, false).unwrap();
        let root = bootstrap(&mut disk, false).unwrap();
        let mut s = Storage::open(disk, &root).unwrap();
        if s.reconcile(&root).is_err() {
            drop(s);
            let disk = Disk::at(&dir.0, false).unwrap();
            let mut reopened = Storage::open(disk, &root).unwrap();
            reopened.restore_previous(&root).unwrap();
            assert_eq!(reopened.recovery(), crate::Recovery::Clean);
        } else {
            assert_eq!(s.recovery(), crate::Recovery::Clean);
        }
    }
}
