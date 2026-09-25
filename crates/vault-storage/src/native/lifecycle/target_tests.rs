//! Real Windows file effects; only newly created synthetic homes/vaults.
use super::super::*;
use super::target::Target;
use crate::engine::coordinator::{Check, Choice, Effects, Request, Snapshot, SwitchError};
use crate::journal::{Failure, GenerationRef, SwitchPhase};
use crate::{Capture, ProfileId, ProfileText};
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};
use codex_accounts_vault::{CredentialSet, Identity, Resource, ResourceId, ResourceShape};

const A0: &[u8] = b"{ \"SYNTHETIC\":\"NOT_REAL_A0\" }\r\n";
const A1: &[u8] = b"{\"SYNTHETIC\":\"NOT_REAL_A1\",\"unknown\":[true,null]}\r\n";
const B0: &[u8] = b"{\"SYNTHETIC\":\"NOT_REAL_B0\"}";
const B1: &[u8] = b"{\"SYNTHETIC\":\"NOT_REAL_B1\"}\r\n";
const BINDING: [u8; 32] = [0x43; 32];
const INITIALIZE: Id = [0x49; 16];
fn identity(subject: &str) -> Identity {
    Identity::new(
        "SYNTHETIC_ISSUER".into(),
        subject.into(),
        "SYNTHETIC_WORKSPACE".into(),
    )
    .unwrap()
}
fn text() -> ProfileText {
    ProfileText::new("SYNTHETIC_ONLY".into()).unwrap()
}
fn values(main: &[u8], mask: u8) -> Vec<Option<Vec<u8>>> {
    vec![
        Some(main.to_vec()),
        (mask & 1 != 0).then(|| b"SYNTHETIC\r\n\0\xff".to_vec()),
        (mask & 2 != 0).then(Vec::new),
    ]
}
fn resources(values: &[Option<Vec<u8>>]) -> Vec<Resource> {
    values
        .iter()
        .enumerate()
        .map(|(i, b)| match b {
            Some(b) => Resource::present(ResourceId::new(i as u8).unwrap(), b.clone()).unwrap(),
            None => Resource::absent(ResourceId::new(i as u8).unwrap()),
        })
        .collect()
}
fn capture(who: &str, v: &[Option<Vec<u8>>]) -> Capture {
    Capture::new(
        identity(who),
        1,
        vec![
            (ResourceId::new(0).unwrap(), ResourceShape::JsonObject, true),
            (ResourceId::new(1).unwrap(), ResourceShape::Opaque, false),
            (ResourceId::new(2).unwrap(), ResourceShape::Opaque, false),
        ],
        resources(v),
    )
    .unwrap()
}
fn mkdir(path: &Path) {
    let security = Security::new().unwrap();
    assert_ne!(
        unsafe { CreateDirectoryW(wide(path).unwrap().as_ptr(), &security.attrs()) },
        0,
        "create only a fresh protected synthetic directory"
    );
}
fn home() -> super::super::tests::Sandbox {
    let mut dir = super::super::tests::Sandbox::new();
    dir.0.set_file_name("ca04c-synthetic-home");
    mkdir(&dir.0);
    dir
}
fn plain(t: &Target) -> Vec<Option<Vec<u8>>> {
    t.snapshot()
        .unwrap()
        .iter()
        .map(|r| r.as_bytes().map(<[u8]>::to_vec))
        .collect()
}
fn failure(e: StorageError) -> Failure {
    match e {
        StorageError::ExternalChange | StorageError::UnsafePath => Failure::ExternalChange,
        StorageError::InvalidData | StorageError::InputLimit => Failure::InvalidData,
        StorageError::Busy => Failure::Busy,
        _ => Failure::Write,
    }
}
struct NativeEffects {
    // Field destruction order keeps the home lock until the owned family exits.
    helper: Option<(Id, SyntheticHelper)>,
    target: Target,
    home: PathBuf,
}
impl Effects for NativeEffects {
    fn confirm(&mut self, request: &Request) -> Result<(), Failure> {
        // Test-only confirmation is not native consent or Desktop qualification.
        self.guard(Check::Confirm, &request.binding)
    }
    fn guard(&mut self, check: Check, binding: &[u8; 32]) -> Result<(), Failure> {
        if binding != &BINDING {
            return Err(Failure::Binding);
        }
        self.target.validate().map_err(failure)?;
        if self.helper.is_some() && matches!(check, Check::Write | Check::Commit | Check::Launch) {
            return Err(Failure::Writers);
        }
        if check == Check::Launch {
            return Err(Failure::Launch);
        }
        // No user processes are discovered or claimed quiescent. The test owns
        // every writer of this fresh fixture; real writer association stays gated.
        Ok(())
    }
    fn snapshot(&mut self) -> Result<Snapshot, Failure> {
        let resources = self.target.snapshot().map_err(failure)?;
        let main = resources[0].as_bytes();
        let who = if main == Some(A0) || main == Some(A1) {
            Some(identity("SYNTHETIC_A"))
        } else if main == Some(B0) || main == Some(B1) {
            Some(identity("SYNTHETIC_B"))
        } else {
            None
        };
        Ok(Snapshot {
            identity: who,
            schema: 1,
            resources,
        })
    }
    fn stage(&mut self, op: Id, slot: u8, value: Option<&[u8]>) -> Result<(), Failure> {
        self.target.stage(op, slot, value).map_err(failure)
    }
    fn replace(
        &mut self,
        op: Id,
        slot: u8,
        expected: Option<&[u8]>,
        value: Option<&[u8]>,
    ) -> Result<(), Failure> {
        self.target
            .replace(op, slot, expected, value)
            .map_err(failure)
    }
    fn cleanup(
        &mut self,
        op: Id,
        registered: u16,
        allowed: &[CredentialSet],
    ) -> Result<(), Failure> {
        self.target
            .cleanup(op, registered, allowed)
            .map_err(|e| match e {
                StorageError::ExternalChange | StorageError::UnsafePath => Failure::ExternalChange,
                _ => Failure::Cleanup,
            })
    }
    fn start_helper(&mut self, op: Id) -> Result<(), Failure> {
        if self.helper.is_some() {
            return Err(Failure::HelperStuck);
        }
        self.helper = Some((op, SyntheticHelper::start(&self.home)));
        Ok(())
    }
    fn observe(&mut self, op: Id) -> Result<CredentialAcceptance, Failure> {
        if self.helper.as_ref().is_none_or(|(id, _)| *id != op) {
            return Err(Failure::HelperStuck);
        }
        // No provider is called and no credentials are accepted by this fixture.
        Ok(CredentialAcceptance::ObservationUnavailable)
    }
    fn reap_helper(&mut self, op: Id) -> Result<(), Failure> {
        if self.helper.as_ref().is_none_or(|(id, _)| *id != op) {
            return Err(Failure::HelperStuck);
        }
        let (_, mut helper) = self.helper.take().ok_or(Failure::HelperStuck)?;
        helper.reap()
    }
    fn launch(&mut self, _: Id) -> Result<DesktopLaunch, Failure> {
        Err(Failure::Launch)
    }
}
struct Fixture {
    store: Storage<Disk>,
    fx: NativeEffects,
    root: RootKey,
    a: ProfileId,
    b: ProfileId,
    home: PathBuf,
    dir: super::super::tests::Sandbox,
}
impl Fixture {
    fn new(source: u8, target: u8) -> Self {
        let dir = super::super::tests::Sandbox::new();
        let home = dir.0.with_file_name("ca04c-synthetic-home");
        mkdir(&home);
        let mut disk = Disk::at(&dir.0, true).unwrap();
        let root = bootstrap(&mut disk, true).unwrap();
        let mut store = Storage::create(disk, &root).unwrap();
        let (a, _) = store
            .add(
                &root,
                text(),
                text(),
                capture("SYNTHETIC_A", &values(A0, source)),
            )
            .unwrap();
        let (b, _) = store
            .add(
                &root,
                text(),
                text(),
                capture("SYNTHETIC_B", &values(B0, target)),
            )
            .unwrap();
        let mut target_home = Target::open(&home, 7).unwrap();
        for (i, bytes) in values(A1, source).iter().enumerate() {
            target_home
                .replace(INITIALIZE, i as u8, None, bytes.as_deref())
                .unwrap();
        }
        target_home.boundary = 0;
        Self {
            store,
            fx: NativeEffects {
                target: target_home,
                home: home.clone(),
                helper: None,
            },
            root,
            a,
            b,
            home,
            dir,
        }
    }
    fn request(&self) -> Request {
        Request {
            source: GenerationRef {
                profile: self.a.to_bytes(),
                generation: self.store.latest(self.a).unwrap().to_bytes(),
            },
            target: GenerationRef {
                profile: self.b.to_bytes(),
                generation: self.store.latest(self.b).unwrap().to_bytes(),
            },
            binding: BINDING,
            online: false,
        }
    }
    fn begin(&mut self) -> Id {
        let request = self.request();
        self.store
            .begin_switch(&self.root, &mut self.fx, request)
            .unwrap()
    }
    fn phase(&self) -> SwitchPhase {
        self.store.switch_status().unwrap().last().unwrap().phase
    }
    fn until(&mut self, id: Id, phase: SwitchPhase) {
        for _ in 0..100 {
            if self.phase() == phase {
                return;
            }
            self.store
                .advance_switch(&self.root, &mut self.fx, id)
                .unwrap();
        }
        panic!("bounded native coordinator transition not reached");
    }
    fn reopen(self) -> Self {
        let Self {
            store,
            fx,
            root,
            a,
            b,
            home,
            dir,
        } = self;
        drop(store);
        drop(fx);
        drop(root);
        let mut disk = Disk::at(&dir.0, false).unwrap();
        let root = bootstrap(&mut disk, false).unwrap();
        let mut store = Storage::open(disk, &root).unwrap();
        if matches!(
            store.recovery(),
            Recovery::CommitPending | Recovery::CleanupPending
        ) {
            store.reconcile(&root).unwrap();
        }
        Self {
            store,
            fx: NativeEffects {
                target: Target::open(&home, 7).unwrap(),
                home: home.clone(),
                helper: None,
            },
            root,
            a,
            b,
            home,
            dir,
        }
    }
}
#[test]
fn ca04c_exact_bytes_absence_and_empty_are_distinct() {
    let options = [None, Some(&b""[..]), Some(&b"SYNTHETIC\0\xff\r\n"[..])];
    for a in options {
        for b in options {
            let dir = home();
            let mut target = Target::open(&dir.0, 1).unwrap();
            target.replace(INITIALIZE, 0, None, a).unwrap();
            target.stage([1; 16], 0, b).unwrap();
            target.replace([1; 16], 0, a, b).unwrap();
            assert_eq!(target.read(0).unwrap().as_deref().map(Vec::as_slice), b);
            let resource = match b {
                Some(b) => Resource::present(ResourceId::new(0).unwrap(), b.to_vec()).unwrap(),
                None => Resource::absent(ResourceId::new(0).unwrap()),
            };
            let set = CredentialSet::new(
                &[(ResourceId::new(0).unwrap(), ResourceShape::Opaque, false)],
                vec![resource],
            )
            .unwrap();
            target.cleanup([1; 16], 1, &[set]).unwrap();
            assert_eq!(
                std::fs::read_dir(&dir.0).unwrap().count(),
                usize::from(b.is_some())
            );
        }
    }
}
#[test]
fn ca04c_expected_old_foreign_stage_and_invalid_slot_refuse() {
    let dir = home();
    let mut target = Target::open(&dir.0, 1).unwrap();
    target.replace(INITIALIZE, 0, None, Some(A0)).unwrap();
    target.stage([1; 16], 0, Some(B0)).unwrap();
    assert_eq!(
        target.replace([1; 16], 0, Some(A1), Some(B0)),
        Err(StorageError::ExternalChange)
    );
    assert_eq!(
        target.stage([1; 16], 0, Some(B1)),
        Err(StorageError::ExternalChange)
    );
    assert_eq!(
        target.stage([1; 16], 16, Some(B0)),
        Err(StorageError::InvalidData)
    );
    assert_eq!(
        target.stage([0; 16], 0, Some(B0)),
        Err(StorageError::InvalidData)
    );
    assert_eq!(target.read(0).unwrap().unwrap().as_slice(), A0);
    assert_eq!(format!("{target:?}"), "SyntheticTarget([REDACTED])");
}
#[test]
fn ca04c_sharing_hardlink_and_namespace_changes_refuse() {
    let dir = home();
    let mut target = Target::open(&dir.0, 1).unwrap();
    target.replace(INITIALIZE, 0, None, Some(A0)).unwrap();
    let live = dir.0.join("synthetic-resource-00.bin");
    let writer = open_file(
        &live,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    assert!(matches!(
        target.replace([1; 16], 0, Some(A0), Some(B0)),
        Err(StorageError::Busy)
    ));
    drop(writer);
    let alias = dir.0.join("synthetic-hardlink");
    std::fs::hard_link(&live, &alias).unwrap();
    assert!(matches!(target.read(0), Err(StorageError::UnsafePath)));
    std::fs::remove_file(alias).unwrap();
    assert!(std::fs::rename(&dir.0, dir.0.with_extension("moved")).is_err());
    assert!(matches!(Target::open(&dir.0, 1), Err(StorageError::Busy)));
    std::fs::rename(&live, dir.0.join("synthetic-foreign.bin")).unwrap();
    assert_eq!(
        target.replace([1; 16], 0, Some(A0), Some(B0)),
        Err(StorageError::ExternalChange)
    );
}
#[test]
fn ca04c_cleanup_requires_registered_authenticated_bytes() {
    let dir = home();
    let mut target = Target::open(&dir.0, 1).unwrap();
    target.stage([1; 16], 0, Some(B0)).unwrap();
    let set = CredentialSet::new(
        &[(ResourceId::new(0).unwrap(), ResourceShape::Opaque, false)],
        vec![Resource::present(ResourceId::new(0).unwrap(), A0.to_vec()).unwrap()],
    )
    .unwrap();
    assert_eq!(
        target.cleanup([1; 16], 0, &[set]),
        Err(StorageError::ExternalChange)
    );
    assert_eq!(
        target.cleanup([1; 16], 1, &[]),
        Err(StorageError::ExternalChange)
    );
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    let set = CredentialSet::new(
        &[(ResourceId::new(0).unwrap(), ResourceShape::Opaque, false)],
        vec![Resource::present(ResourceId::new(0).unwrap(), B0.to_vec()).unwrap()],
    )
    .unwrap();
    target.cleanup([1; 16], 1, &[set]).unwrap();
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
}
#[test]
fn ca04c_journal_preserves_newest_source_and_committed_target() {
    let mut f = Fixture::new(1, 2);
    let old = f.store.latest(f.a).unwrap();
    let id = f.begin();
    f.until(id, SwitchPhase::TargetStaged);
    assert_ne!(f.store.latest(f.a).unwrap(), old);
    assert_eq!(
        f.store
            .read_latest(&f.root, f.a)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(A1)
    );
    f.until(id, SwitchPhase::Finished);
    assert_eq!(plain(&f.fx.target), values(B0, 2));
    let status = f.store.switch_status().unwrap().pop().unwrap();
    assert_eq!(status.installed_profile, Some(f.b));
    assert_eq!(status.primary_error, Some("E_LAUNCH_FAILED"));
    assert_eq!(
        f.store
            .recover_switch(&f.root, &mut f.fx, id, Choice::Restore),
        Err(SwitchError::InvalidTransition)
    );
    let f = f.reopen();
    assert_eq!(plain(&f.fx.target), values(B0, 2));
}
#[test]
fn ca04c_midpoint_reopen_restores_with_separate_failures() {
    let mut f = Fixture::new(1, 2);
    let id = f.begin();
    f.until(id, SwitchPhase::Applying);
    assert_eq!(f.fx.target.read(0).unwrap().unwrap().as_slice(), B0);
    f.store.cancel_switch(&f.root, &mut f.fx, id).unwrap();
    let mut f = f.reopen();
    f.store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Restore)
        .unwrap();
    f.fx.target.fail_at = Some(1);
    assert!(f.store.advance_switch(&f.root, &mut f.fx, id).is_err());
    let status = f.store.switch_status().unwrap().pop().unwrap();
    assert_eq!(status.primary_error, Some("E_CANCELLED"));
    assert_eq!(status.restoration_error, Some("E_WRITE_FAILED"));
    let mut f = f.reopen();
    f.store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Restore)
        .unwrap();
    f.until(id, SwitchPhase::Restored);
    assert_eq!(plain(&f.fx.target), values(A1, 1));
    assert_eq!(std::fs::read_dir(&f.home).unwrap().count(), 2);
}
#[test]
fn ca04c_external_change_and_stale_parent_never_overwrite() {
    let mut f = Fixture::new(1, 2);
    let stale = f.request();
    f.store
        .append(
            &f.root,
            f.b,
            f.store.latest(f.b).unwrap(),
            capture("SYNTHETIC_B", &values(B1, 2)),
        )
        .unwrap();
    assert_eq!(
        f.store.begin_switch(&f.root, &mut f.fx, stale),
        Err(SwitchError::Storage(StorageError::StaleParent))
    );
    let id = f.begin();
    f.until(id, SwitchPhase::TargetStaged);
    let live = f.home.join("synthetic-resource-00.bin");
    // Only this fixture is changed, in place by its explicitly controlled writer.
    std::fs::write(&live, b"SYNTHETIC_FOREIGN").unwrap();
    assert!(f.store.advance_switch(&f.root, &mut f.fx, id).is_err());
    assert_eq!(f.phase(), SwitchPhase::Conflict);
    assert_eq!(std::fs::read(live).unwrap(), b"SYNTHETIC_FOREIGN");
    assert_eq!(
        f.store
            .read_latest(&f.root, f.a)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(A1)
    );
}

#[test]
fn ca04c_cleanup_failure_blocks_until_verified_retry() {
    let mut f = Fixture::new(1, 1);
    let id = f.begin();
    f.until(id, SwitchPhase::Committed);
    let stage = std::fs::read_dir(&f.home)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "stage"))
        .unwrap();
    let held = open_file(&stage, GENERIC_READ, 0, OPEN_EXISTING, None).unwrap();
    assert!(f.store.advance_switch(&f.root, &mut f.fx, id).is_err());
    assert_eq!(f.phase(), SwitchPhase::Recovery);
    assert_eq!(
        f.store.switch_status().unwrap()[0].primary_error,
        Some("E_RECOVERY_REQUIRED")
    );
    assert!(stage.exists());
    drop(held);
    let mut f = f.reopen();
    f.store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Finish)
        .unwrap();
    assert_eq!(plain(&f.fx.target), values(B0, 1));
    assert_eq!(f.phase(), SwitchPhase::Finished);
    assert!(!stage.exists());
}
#[test]
fn ca04c_changed_acl_and_oversized_input_refuse() {
    let dir = home();
    let mut target = Target::open(&dir.0, 1).unwrap();
    assert_eq!(
        target.stage([1; 16], 0, Some(&vec![0; 1024 * 1024 + 1])),
        Err(StorageError::InputLimit)
    );
    target.replace(INITIALIZE, 0, None, Some(A0)).unwrap();
    let file = open_file(
        &dir.0.join("synthetic-resource-00.bin"),
        WRITE_DAC | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let security = Security::new().unwrap();
    let mut acl = null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    assert_ne!(
        unsafe {
            GetSecurityDescriptorDacl(
                security.descriptor.0,
                &mut present,
                &mut acl,
                &mut defaulted,
            )
        },
        0
    );
    assert_ne!(present, 0);
    // Remove DACL protection on ONLY this newly-created synthetic leaf. Never
    // repair existing permissions or request elevation. Original bytes survive.
    assert_eq!(
        unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl,
                null_mut(),
            )
        },
        0
    );
    drop(file);
    assert!(matches!(target.read(0), Err(StorageError::UnsafePath)));
    assert_eq!(
        std::fs::read(dir.0.join("synthetic-resource-00.bin")).unwrap(),
        A0
    );
}

// Same owned-job primitive as CA-04B. This child is the test binary itself, not
// an official runtime; its one write is explicitly synthetic and never a login.
struct Spawned {
    process: Token,
    thread: Token,
    pid: u32,
}
impl Drop for Spawned {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Threading::*;
        // Only CreateProcessW's original owned handle, never a discovered PID.
        if unsafe { WaitForSingleObject(self.process.0, 0) } != WAIT_OBJECT_0 {
            unsafe {
                TerminateProcess(self.process.0, 1);
                WaitForSingleObject(self.process.0, 5000);
            }
        }
    }
}
struct SyntheticHelper {
    child: Spawned,
    family: super::helper::OwnedFamily,
}
impl SyntheticHelper {
    fn start(home: &Path) -> Self {
        use windows_sys::Win32::System::Threading::*;
        let executable = std::env::current_exe().unwrap();
        let text = executable.to_str().unwrap();
        assert!(!text.contains('"'));
        let application: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
        let mut command: Vec<u16> = format!("\"{text}\" --exact native::lifecycle::target_tests::native_target_helper_child --nocapture\0")
            .encode_utf16().collect();
        let mut environment = std::collections::BTreeMap::<String, OsString>::new();
        for name in [
            "SYSTEMROOT",
            "SYSTEMDRIVE",
            "WINDIR",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "LOCALAPPDATA",
        ] {
            if let Some(value) = std::env::var_os(name) {
                environment.insert(name.into(), value);
            }
        }
        environment.insert("CA04C_HELPER_HOME".into(), home.as_os_str().to_owned());
        environment.insert("RUST_TEST_THREADS".into(), "1".into());
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
        // SAFETY: fixed test executable/arguments, private bounded environment,
        // no inherited handles or shell, suspended until assigned to our new job.
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
                    wide(home).unwrap().as_ptr(),
                    &startup,
                    &mut info,
                )
            },
            0
        );
        let child = Spawned {
            process: Token(info.hProcess),
            thread: Token(info.hThread),
            pid: info.dwProcessId,
        };
        let family =
            super::helper::OwnedFamily::for_suspended_test_child(child.process.0, child.pid)
                .unwrap();
        assert_eq!(unsafe { ResumeThread(child.thread.0) }, 1);
        Self { child, family }
    }
    fn reap(&mut self) -> Result<(), Failure> {
        use windows_sys::Win32::System::Threading::*;
        let outcome = self
            .family
            .shutdown_after_input_closed()
            .map_err(|_| Failure::HelperStuck)?;
        let mut code = 1;
        if outcome.forced
            || unsafe { GetExitCodeProcess(self.child.process.0, &mut code) } == 0
            || code != 0
        {
            return Err(Failure::HelperStuck);
        }
        Ok(())
    }
}
impl Drop for SyntheticHelper {
    fn drop(&mut self) {
        let _ = self.family.shutdown_after_input_closed();
    }
}
#[test]
fn native_target_helper_child() {
    let Some(home) = std::env::var_os("CA04C_HELPER_HOME") else {
        return;
    };
    let home = PathBuf::from(home);
    assert_eq!(home.file_name().unwrap(), "ca04c-synthetic-home");
    validate_path(&home).unwrap();
    let security = Security::new().unwrap();
    let mut file = open_file(
        &home.join("synthetic-resource-00.bin"),
        GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
        0,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    security.check(&file, true).unwrap();
    check_object(&file, false).unwrap();
    assert_eq!(read_bytes(&mut file).unwrap(), B0);
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(B1).unwrap();
    file.set_len(B1.len() as u64).unwrap();
    file.sync_all().unwrap();
}
#[test]
fn ca04c_owned_helper_generation_survives_source_restoration() {
    let mut f = Fixture::new(1, 2);
    let before = f.store.latest(f.b).unwrap();
    let mut request = f.request();
    request.online = true;
    let id = f.store.begin_switch(&f.root, &mut f.fx, request).unwrap();
    f.until(id, SwitchPhase::Observed);
    assert_eq!(
        f.store.switch_status().unwrap()[0]
            .observations
            .credential_acceptance(),
        CredentialAcceptance::ObservationUnavailable
    );
    assert_ne!(f.store.latest(f.b).unwrap(), before);
    assert_eq!(
        f.store
            .read_latest(&f.root, f.b)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(B1)
    );
    f.store.cancel_switch(&f.root, &mut f.fx, id).unwrap();
    let mut f = f.reopen();
    f.store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Restore)
        .unwrap();
    f.until(id, SwitchPhase::Restored);
    assert_eq!(plain(&f.fx.target), values(A1, 1));
    assert_eq!(
        f.store
            .read_latest(&f.root, f.b)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(B1)
    );
    assert_eq!(
        f.store.switch_status().unwrap()[0]
            .observations
            .credential_acceptance(),
        CredentialAcceptance::Unknown
    );
}

#[path = "target_restart_tests.rs"]
mod restart;
#[test]
fn ca04c_process_restart_forward_boundaries() {
    restart::exercise("forward");
}
#[test]
fn ca04c_process_restart_restoration_boundaries() {
    restart::exercise("restore");
}

pub(super) fn crash_at_native_boundary() -> ! {
    std::process::exit(86)
}
