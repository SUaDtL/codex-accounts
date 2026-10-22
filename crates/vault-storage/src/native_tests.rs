use super::*;
use crate::{Capture, ProfileText};
use codex_accounts_vault::{Identity, Resource, ResourceId, ResourceShape};
use std::process::{Command, Stdio};

// Resolve Profile read-only and create only a fresh protected synthetic parent.
// Existing Profile/ancestor permissions and both application vault slots stay intact.
pub(super) struct Sandbox(pub(super) PathBuf, PathBuf);
impl Sandbox {
    pub(super) fn new() -> Self {
        Self::try_new().expect("safe synthetic fixture parent")
    }
    fn try_new() -> Result<Self, StorageError> {
        Self::try_under(&known_folder(
            &windows_sys::Win32::UI::Shell::FOLDERID_Profile,
        )?)
    }
    fn try_under(base: &Path) -> Result<Self, StorageError> {
        let suffix: String = random_id()?.iter().map(|b| format!("{b:02x}")).collect();
        let parent = base.join(format!("ca03c-test-{suffix}"));
        Self::create_parent(&parent)
    }
    fn create_parent(parent: &Path) -> Result<Self, StorageError> {
        Self::create_parent_with_final_check(parent, cloud_check)
    }
    fn create_parent_with_final_check(
        parent: &Path,
        final_check: impl FnOnce(&File, &[File]) -> Result<(), StorageError>,
    ) -> Result<Self, StorageError> {
        validate_path(parent)?;
        let security = Security::new()?;
        let mut ancestors: Vec<_> = parent.ancestors().skip(1).collect();
        ancestors.reverse();
        let mut held = Vec::new();
        for path in ancestors {
            let f = open_file(
                path,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            check_object(&f, true)?;
            security.check(&f, false)?;
            held.push(f);
        }
        check_volume_and_sync(held.last().ok_or(StorageError::UnsafePath)?, &held)?;
        if unsafe { CreateDirectoryW(wide(parent)?.as_ptr(), &security.attrs()) } == 0 {
            return Err(last());
        }
        let created = Self(parent.join("ca03c-test-vault"), parent.to_owned());
        let f = open_file(
            parent,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        check_object(&f, true)?;
        security.check(&f, true)?;
        held.push(f);
        // The test-only seam is exactly the final check, after the real fresh
        // parent handle has joined held. Normal fixtures always use cloud_check.
        final_check(held.last().ok_or(StorageError::UnsafePath)?, &held)?;
        Ok(created)
    }
}

#[test]
fn native_fixture_final_check_refusal_preserves_owned_ancestor() {
    // A regression/removal failure must still clean this exact exclusively
    // created synthetic root. This backstop never searches Profile by prefix.
    struct OwnedBackstop {
        fixture: Option<Sandbox>,
        root: PathBuf,
    }
    impl Drop for OwnedBackstop {
        fn drop(&mut self) {
            drop(self.fixture.take());
            if self.root.exists() {
                std::fs::remove_dir_all(&self.root).expect("remove exact owned regression root");
            }
        }
    }
    let owned = Sandbox::new();
    let backstop = OwnedBackstop {
        root: owned.1.clone(),
        fixture: Some(owned),
    };
    let fixture = backstop.fixture.as_ref().unwrap();
    let ancestor = open_file(
        &fixture.1,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    Security::new().unwrap().check(&ancestor, true).unwrap();
    let ancestor_security = security_bytes(&ancestor);
    let canary = fixture.1.join("synthetic-unrelated-canary");
    std::fs::write(&canary, b"SYNTHETIC_UNRELATED_ANCESTOR_UNCHANGED").unwrap();
    let parent = fixture.1.join("synthetic-final-check-refusal");
    assert!(!parent.exists(), "refusal parent starts absent");
    let mut callbacks = 0;
    let mut strict_fresh_parent_observed = false;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Sandbox::create_parent_with_final_check(&parent, |file, held| {
            callbacks += 1;
            assert!(
                held.last().unwrap().as_raw_handle() == file.as_raw_handle(),
                "the fresh parent handle is still the final held handle"
            );
            Security::new().unwrap().check(file, true).unwrap();
            let actual = check_object(file, true).unwrap();
            let expected = open_file(
                &parent,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )
            .unwrap();
            let expected = check_object(&expected, true).unwrap();
            assert!(
                (actual.volume, actual.id) == (expected.volume, expected.id),
                "callback receives this exact newly-created protected parent"
            );
            cloud_check(file, held).unwrap();
            strict_fresh_parent_observed = true;
            // Refuse only after actual native creation, metadata protection and
            // the real final sync query. This is a test-only injected error,
            // never an assumed inventory, bypass or qualification outcome.
            Err(StorageError::UnsafePath)
        })
    }));
    assert_eq!(
        callbacks, 1,
        "the final post-push callback must actually execute once"
    );
    assert!(
        strict_fresh_parent_observed,
        "strict live parent checks precede injected refusal"
    );
    assert!(
        result.is_ok(),
        "final-check refusal must return without cleanup panic"
    );
    assert!(
        matches!(result.unwrap(), Err(StorageError::UnsafePath)),
        "the original typed final-check refusal must be preserved"
    );
    assert!(
        !parent.exists(),
        "final-check refusal removes only its exclusively-created parent"
    );
    assert!(
        fixture.1.is_dir(),
        "the existing owned ancestor remains present"
    );
    assert!(
        std::fs::read(&canary).unwrap() == b"SYNTHETIC_UNRELATED_ANCESTOR_UNCHANGED",
        "final-check refusal preserves unrelated ancestor canary bytes"
    );
    assert!(
        security_bytes(&ancestor) == ancestor_security,
        "final-check refusal preserves existing ancestor protection"
    );
    drop(ancestor);
}

// Independent read-only oracle: no create/redirect flags, no environment path.
fn observed_profile_folder() -> PathBuf {
    let mut p = null_mut();
    let status = unsafe {
        SHGetKnownFolderPath(
            &windows_sys::Win32::UI::Shell::FOLDERID_Profile,
            0,
            null_mut(),
            &mut p,
        )
    };
    let p = TaskString(p);
    assert!(
        status >= 0 && !p.0.is_null(),
        "read-only Profile known-folder query"
    );
    let mut len = 0;
    while len < 240 && unsafe { *p.0.add(len) } != 0 {
        len += 1;
    }
    assert!(len > 0 && len < 240, "bounded Profile known-folder result");
    PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(p.0, len)
    }))
}

fn security_bytes(file: &File) -> Vec<u8> {
    let mut descriptor = null_mut();
    assert_eq!(
        unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descriptor,
            )
        },
        0,
        "read-only synthetic descriptor snapshot"
    );
    let descriptor = Allocation(descriptor);
    let len = unsafe { GetSecurityDescriptorLength(descriptor.0) } as usize;
    assert!((1..=65536).contains(&len), "bounded descriptor snapshot");
    unsafe { std::slice::from_raw_parts(descriptor.0.cast::<u8>(), len) }.to_vec()
}

#[test]
fn native_profile_fixture_parent_is_protected_and_unsafe_parents_refuse() {
    let profile = observed_profile_folder();
    let profile_handle = open_file(
        &profile,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let original_profile_security = security_bytes(&profile_handle);
    let created = Sandbox::try_new();
    assert!(
        created.is_ok(),
        "Profile fixture must create safely without AppData ancestry"
    );
    let fixture = created.unwrap();
    assert!(
        fixture.1.parent() == Some(profile.as_path()),
        "fixture must be directly under Profile"
    );
    let name = fixture.1.file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("ca03c-test-") && name.len() == 43,
        "unique synthetic fixture name, never an application vault slot"
    );
    let second = Sandbox::try_new().unwrap();
    assert!(
        second.1 != fixture.1,
        "separate fixtures have distinct parents"
    );
    let security = Security::new().unwrap();
    let parent_handle = open_file(
        &fixture.1,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    check_object(&parent_handle, true).unwrap();
    security.check(&parent_handle, true).unwrap();
    let before = security_bytes(&parent_handle);
    assert!(
        matches!(
            Sandbox::create_parent(&fixture.1),
            Err(StorageError::AlreadyExists)
        ),
        "an existing synthetic parent is never adopted or overwritten"
    );
    assert!(
        security_bytes(&parent_handle) == before,
        "collision leaves the protected descriptor unchanged"
    );
    drop(parent_handle);
    // Native storage creation/reopen exercises the unchanged owner, NTFS,
    // ancestry, sync, protected root/leaf and exclusive kernel-lock checks.
    drop(Disk::at(&fixture.0, true).unwrap());
    drop(Disk::at(&fixture.0, false).unwrap());
    let file = fixture.1.join("synthetic-file-parent");
    std::fs::write(&file, b"SYNTHETIC_PARENT_CANARY").unwrap();
    assert!(
        matches!(Sandbox::try_under(&file), Err(StorageError::UnsafePath)),
        "a file cannot be a fixture ancestor"
    );
    assert!(
        std::fs::read(&file).unwrap() == b"SYNTHETIC_PARENT_CANARY",
        "refusal preserves synthetic file bytes"
    );
    for base in [
        fixture.1.join("OneDrive"),
        fixture.1.join(".."),
        fixture.1.join("CON"),
    ] {
        assert!(
            matches!(Sandbox::try_under(&base), Err(StorageError::UnsafePath)),
            "unsafe sync, traversal and device parents remain refused"
        );
    }
    let bad = fixture.1.join("synthetic-broad-parent");
    let sddl: Vec<u16> = "D:P(A;;FA;;;WD)\0".encode_utf16().collect();
    let mut descriptor = null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        },
        0
    );
    let descriptor = Allocation(descriptor);
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // This deliberately unsafe descriptor belongs only to this newly-created
    // synthetic directory. No existing ancestor's permissions are changed.
    assert_ne!(
        unsafe { CreateDirectoryW(wide(&bad).unwrap().as_ptr(), &attrs) },
        0
    );
    let bad_handle = open_file(
        &bad,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let bad_before = security_bytes(&bad_handle);
    assert!(
        matches!(Sandbox::try_under(&bad), Err(StorageError::UnsafePath)),
        "broad writable synthetic ancestors refuse without a fallback"
    );
    assert!(
        security_bytes(&bad_handle) == bad_before,
        "unsafe parent descriptor is never repaired"
    );
    assert_eq!(
        std::fs::read_dir(&bad).unwrap().count(),
        0,
        "refusal creates no fixture below an unsafe parent"
    );
    drop(bad_handle);
    assert!(
        security_bytes(&profile_handle) == original_profile_security,
        "existing Profile permissions remain unchanged"
    );
    drop(profile_handle);
    drop(second);
    let owned = fixture.1.clone();
    drop(fixture);
    assert!(
        !owned.exists(),
        "cleanup removes only the owned synthetic tree"
    );
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        // Every object here was created under this random, exclusively-created
        // synthetic parent. This is test cleanup, never a product cleanup path.
        std::fs::remove_dir_all(&self.1).expect("remove only owned synthetic test directory");
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

// TASK-05 exercises the same private constructor used by the public fixed-folder
// entry. Every path belongs to this exclusively created synthetic fixture.
fn layout_paths(fixture: &Sandbox) -> [PathBuf; 2] {
    [
        fixture.1.join("CodexAccounts-Vault-v1"),
        fixture
            .1
            .join("synthetic-profile")
            .join("CodexAccounts-Storage")
            .join("CodexAccounts-Vault-v1"),
    ]
}
fn layout_fixture() -> (Sandbox, [PathBuf; 2]) {
    let fixture = location_fixture();
    let paths = layout_paths(&fixture);
    protected_directory(paths[1].parent().unwrap().parent().unwrap());
    (fixture, paths)
}
fn protected_directory(path: &Path) {
    let security = Security::new().unwrap();
    assert_ne!(
        unsafe { CreateDirectoryW(wide(path).unwrap().as_ptr(), &security.attrs()) },
        0
    );
}
fn layout_seed(
    paths: &[PathBuf; 2],
    index: usize,
) -> (RootKey, crate::ProfileId, crate::GenerationId) {
    if index == 1 {
        protected_directory(paths[1].parent().unwrap());
    }
    let mut disk = Disk::at(&paths[index], true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let mut store = Storage::create(disk, &root).unwrap();
    let (profile, generation) = store.add(&root, text(), text(), cap()).unwrap();
    drop(store);
    (root, profile, generation)
}
fn layout_open(paths: &[PathBuf; 2], create: bool) -> Result<Vault, StorageError> {
    open_vault_dispatch(create, &known_location_namespace(paths), || {
        Ok(paths.clone())
    })
}
fn layout_open_with_passphrase(
    paths: &[PathBuf; 2],
    passphrase: Option<&[u8]>,
) -> Result<Vault, StorageError> {
    open_vault_dispatch_with_passphrase(false, passphrase, &known_location_namespace(paths), || {
        Ok(paths.clone())
    })
}

#[test]
fn native_layout_namespace_precedes_known_folder_resolution() {
    use windows_sys::Win32::System::Threading::{CreateMutexExW, OpenMutexW};
    let fixture = Sandbox::new();
    let namespace = location_namespace(&fixture);
    let security = Security::new().unwrap();
    let calls = std::cell::Cell::new(0);
    let denied_resolution = || {
        calls.set(calls.get() + 1);
        Err(StorageError::AccessDenied)
    };
    let guard = LocationLock::acquire(&security, &namespace).unwrap();
    assert!(
        matches!(
            open_vault_dispatch(false, &namespace, denied_resolution),
            Err(StorageError::Busy)
        ),
        "constructor namespace Busy precedes even failed known-folder resolution"
    );
    assert_eq!(
        calls.get(),
        0,
        "Busy never invokes the actual resolver dispatch"
    );
    drop(guard);
    for expected in 1..=2 {
        assert!(matches!(
            open_vault_dispatch(false, &namespace, denied_resolution),
            Err(StorageError::AccessDenied)
        ));
        assert_eq!(
            calls.get(),
            expected,
            "resolution failure releases actual namespace ownership"
        );
    }

    let mut child = LocationChild::start(&fixture, "abandon");
    child.ready(&fixture);
    let observer = Token(unsafe {
        OpenMutexW(
            LOCATION_RIGHTS,
            0,
            location_mutex_name(&security, &namespace).as_ptr(),
        )
    });
    assert!(!observer.0.is_null());
    child.finish(&fixture, 73);
    assert!(
        matches!(
            open_vault_dispatch(false, &namespace, denied_resolution),
            Err(StorageError::RecoveryRequired)
        ),
        "constructor abandonment precedes failed known-folder resolution"
    );
    assert_eq!(
        calls.get(),
        2,
        "abandoned ownership never invokes resolution"
    );
    assert!(matches!(
        open_vault_dispatch(false, &namespace, denied_resolution),
        Err(StorageError::AccessDenied)
    ));
    assert_eq!(
        calls.get(),
        3,
        "abandoned refusal releases ownership before retry"
    );
    drop(observer);

    let broad_namespace = [namespace.as_slice(), b"-broad-dispatch"].concat();
    let sddl: Vec<u16> = "D:P(A;;GA;;;WD)\0".encode_utf16().collect();
    let mut descriptor = null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        },
        0
    );
    let descriptor = Allocation(descriptor);
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let broad = Token(unsafe {
        CreateMutexExW(
            &attrs,
            location_mutex_name(&security, &broad_namespace).as_ptr(),
            0,
            LOCATION_RIGHTS,
        )
    });
    assert!(!broad.0.is_null());
    assert!(
        matches!(
            open_vault_dispatch(false, &broad_namespace, denied_resolution),
            Err(StorageError::UnsafePath)
        ),
        "constructor namespace protection refusal precedes failed known-folder resolution"
    );
    assert_eq!(
        calls.get(),
        3,
        "namespace refusal never invokes resolution or repairs protection"
    );
}
fn layout_snapshot(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            {
                let name = entry.file_name().into_string().unwrap();
                let bytes = if name == "lock" {
                    assert_eq!(entry.metadata().unwrap().len(), 0);
                    Vec::new()
                } else {
                    std::fs::read(entry.path()).unwrap()
                };
                (name, bytes)
            }
        })
        .collect()
}
fn synthetic_descriptor(extra: &str) -> Allocation {
    let security = Security::new().unwrap();
    let mut text = null_mut();
    assert_ne!(
        unsafe { ConvertSidToStringSidW(security.user.as_ptr().cast_mut().cast(), &mut text) },
        0
    );
    let text = Allocation(text.cast());
    let mut len = 0;
    while len < 256 && unsafe { *text.0.cast::<u16>().add(len) } != 0 {
        len += 1;
    }
    assert!(len < 256);
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text.0.cast::<u16>(), len) })
        .unwrap();
    let sddl: Vec<u16> = format!("O:{sid}D:P{extra}(A;;FA;;;{sid})(A;;FA;;;SY)\0")
        .encode_utf16()
        .collect();
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
    Allocation(sd)
}
fn synthetic_dacl(path: &Path, extra: &str) {
    let sd = synthetic_descriptor(extra);
    let (mut present, mut acl, mut defaulted) = (0, null_mut(), 0);
    assert_ne!(
        unsafe { GetSecurityDescriptorDacl(sd.0, &mut present, &mut acl, &mut defaulted) },
        0
    );
    assert_eq!(
        unsafe {
            SetNamedSecurityInfoW(
                wide(path).unwrap().as_ptr(),
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
}

#[test]
fn native_layout_legacy_metadata_unsafe_grants_never_authorize_data_or_denial_fallback() {
    let (fixture, mut paths) = layout_fixture();
    let legacy_parent = fixture.1.join("synthetic-legacy-ancestor");
    protected_directory(&legacy_parent);
    paths[0] = legacy_parent.join("CodexAccounts-Vault-v1");
    synthetic_dacl(&legacy_parent, "(A;;0x00000002;;;WD)");
    let parent = open_file(
        &legacy_parent,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let original_descriptor = security_bytes(&parent);
    assert!(
        matches!(
            Security::new().unwrap().check(&parent, false),
            Err(StorageError::UnsafePath)
        ),
        "strict data checks still refuse the positively supported dangerous grant"
    );
    drop(parent);
    let vault = layout_open(&paths, true).expect("checked metadata absence beneath parsed unsafe legacy grant permits only new protected Profile creation");
    drop(vault);
    let parent = open_file(
        &legacy_parent,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    assert_eq!(
        security_bytes(&parent),
        original_descriptor,
        "legacy ancestor permissions are never repaired"
    );
    drop(parent);
    std::fs::remove_dir_all(&paths[1]).unwrap();
    std::fs::remove_dir(paths[1].parent().unwrap()).unwrap();
    protected_directory(&paths[0]);
    assert!(
        matches!(layout_open(&paths, true), Err(StorageError::AlreadyExists)),
        "occupied unsafe legacy slot is observed before any data access"
    );
    assert!(
        matches!(layout_open(&paths, false), Err(StorageError::UnsafePath)),
        "selected legacy data access refuses with no Profile fallback"
    );
    assert!(!paths[0].join("lock").exists());
    std::fs::remove_dir(&paths[0]).unwrap();
    synthetic_dacl(
        &legacy_parent,
        "(OA;;FA;00000000-0000-0000-0000-000000000001;;WD)",
    );
    assert!(
        matches!(layout_open(&paths, true), Err(StorageError::UnsafePath)),
        "unsupported ACE is not a metadata unsafe-grant tag"
    );
    synthetic_dacl(&legacy_parent, "(D;;RC;;;OW)");
    assert!(
        matches!(layout_open(&paths, true), Err(StorageError::AccessDenied)),
        "denied observation never becomes Missing"
    );
    assert!(
        !paths[1].parent().unwrap().exists(),
        "all metadata refusals leave container absent"
    );
}

#[test]
fn native_layout_fresh_nested_round_trip_and_container_precedence() {
    let (_fixture, paths) = layout_fixture();
    assert!(
        matches!(
            Disk::at_known_location(&paths, false),
            Err(StorageError::Missing)
        ),
        "genuinely absent container means Missing, never unknown intermediate ancestry"
    );
    let mut vault = layout_open(&paths, true).expect("fresh protected container and root");
    assert!(
        vault.storage.disk.root == paths[1],
        "fresh constructor selects only nested Profile root"
    );
    let (p, g) = vault
        .storage
        .add(&vault.root, text(), text(), cap())
        .unwrap();
    let original = layout_snapshot(&paths[1]);
    drop(vault);
    let vault = layout_open(&paths, false).unwrap();
    assert_eq!(vault.storage.latest(p).unwrap(), g);
    assert_eq!(
        layout_snapshot(&paths[1]),
        original,
        "clean reopen preserves key, control and registry bytes"
    );
    drop(vault);
    assert!(matches!(
        layout_open(&paths, true),
        Err(StorageError::AlreadyExists)
    ));
    std::fs::remove_dir_all(&paths[1]).unwrap();
    assert!(
        matches!(
            layout_open(&paths, true),
            Err(StorageError::RecoveryRequired)
        ),
        "empty existing container is never adopted"
    );
    assert!(matches!(
        layout_open(&paths, false),
        Err(StorageError::RecoveryRequired)
    ));
}

#[test]
fn native_layout_reserved_collision_and_empty_container_beside_legacy_preserve_bytes() {
    let (_fixture, paths) = layout_fixture();
    let (_, p, g) = layout_seed(&paths, 0);
    let original = layout_snapshot(&paths[0]);
    protected_directory(paths[1].parent().unwrap());
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "empty container blocks sole legacy selection"
    );
    assert!(matches!(
        layout_open(&paths, true),
        Err(StorageError::AlreadyExists)
    ));
    assert_eq!(layout_snapshot(&paths[0]), original);
    std::fs::remove_dir(paths[1].parent().unwrap()).unwrap();
    let vault = layout_open(&paths, false).unwrap();
    assert_eq!(vault.storage.latest(p).unwrap(), g);
    drop(vault);
    let reserved = paths[1]
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("CodexAccounts-Vault-v1");
    protected_directory(&reserved);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::ActiveStateUnknown)
        ),
        "reserved slot is collision-only"
    );
    assert!(matches!(
        layout_open(&paths, true),
        Err(StorageError::AlreadyExists)
    ));
    assert_eq!(layout_snapshot(&paths[0]), original);
    std::fs::remove_dir(&reserved).unwrap();
    layout_seed(&paths, 1);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::ActiveStateUnknown)
        ),
        "two active roots refuse before data reads"
    );
    assert_eq!(layout_snapshot(&paths[0]), original);

    // COV-05-02 reserved-only constructor oracle: this is deliberately a
    // separate fresh fixture, so the preceding reserved-plus-legacy evidence
    // remains intact and cannot mask a conditional reserved-slot regression.
    {
        let (_fixture, paths) = layout_fixture();
        let profile = paths[1].parent().unwrap().parent().unwrap();
        let reserved = profile.join("CodexAccounts-Vault-v1");
        protected_directory(&reserved);
        let canary = reserved.join("synthetic-reserved-only-canary");
        std::fs::write(&canary, b"SYNTHETIC_RESERVED_ONLY_CANARY").unwrap();
        let reserved_before = layout_snapshot(&reserved);
        let profile_before = std::fs::read_dir(profile)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            profile_before,
            ["CodexAccounts-Vault-v1".to_owned()].into_iter().collect(),
            "the fresh reserved-only fixture has no active root or nested container"
        );
        assert!(
            !paths[0].exists() && !paths[1].exists() && !paths[1].parent().unwrap().exists(),
            "both active roots and the fixed nested container start absent"
        );
        if test_gaps_adverse() == Some(TestGapsAdverse::ReservedAbsent) {
            std::fs::remove_file(&canary).unwrap();
            std::fs::remove_dir(&reserved).unwrap();
            assert!(
                !reserved.exists(),
                "reserved-absent adverse control removed only the owned reserved fixture"
            );
        }
        assert!(
            matches!(
                layout_open(&paths, false),
                Err(StorageError::ActiveStateUnknown)
            ),
            "reserved-only occupancy refuses open before either active slot is created"
        );
        assert!(
            matches!(layout_open(&paths, true), Err(StorageError::AlreadyExists)),
            "reserved-only occupancy refuses create before key or control access"
        );
        assert_eq!(
            layout_snapshot(&reserved),
            reserved_before,
            "both reserved-only refusals preserve exact owned canary inventory"
        );
        let profile_after = std::fs::read_dir(profile)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            profile_after, profile_before,
            "both reserved-only refusals preserve the exact Profile child inventory"
        );
        assert!(
            !paths[0].exists() && !paths[1].exists() && !paths[1].parent().unwrap().exists(),
            "reserved-only refusal never creates either active root or nested container"
        );
        for root in [&paths[0], &paths[1], &reserved] {
            for name in ["key.cakp", "state.bin", "lock"] {
                assert!(
                    !root.join(name).exists(),
                    "reserved-only refusal never creates protected storage files"
                );
            }
        }
    }
}

#[test]
fn native_layout_existing_lock_is_read_only_and_lock_first() {
    let (_fixture, paths) = layout_fixture();
    let _ = layout_seed(&paths, 1);
    std::fs::remove_file(paths[1].join("lock")).unwrap();
    std::fs::rename(paths[1].join("key.cakp"), paths[1].join("key.cakp.stage")).unwrap();
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "missing existing lock wins over staged-key plus payload refusal"
    );
    assert_eq!(
        layout_snapshot(&paths[1]),
        before,
        "recognition never recreates lock or adopts key"
    );
    let lock = open_file(
        &paths[1].join("lock"),
        GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
        0,
        CREATE_NEW,
        Some(&Security::new().unwrap()),
    )
    .unwrap();
    drop(lock);
    assert!(matches!(
        layout_open(&paths, false),
        Err(StorageError::KeyUnavailable)
    ));
    assert_eq!(
        layout_snapshot(&paths[1]).get("key.cakp.stage"),
        before.get("key.cakp.stage")
    );
    std::fs::write(paths[1].join("lock"), b"SYNTHETIC_INVALID_LOCK").unwrap();
    assert!(
        matches!(layout_open(&paths, false), Err(StorageError::Corrupt)),
        "invalid lock also wins over key classification"
    );
}

#[test]
fn native_layout_unrecognized_initialization_and_key_matrix_are_preserved() {
    let (_fixture, paths) = layout_fixture();
    protected_directory(paths[1].parent().unwrap());
    let mut disk = Disk::at(&paths[1], true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    drop(disk);
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "committed key alone is not an authenticated store"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    std::fs::rename(paths[1].join("key.cakp"), paths[1].join("key.cakp.stage")).unwrap();
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "staged-only bootstrap is never adopted"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    std::fs::write(paths[1].join("payload.bin"), b"SYNTHETIC_PAYLOAD").unwrap();
    assert!(matches!(
        layout_open(&paths, false),
        Err(StorageError::KeyUnavailable)
    ));
    std::fs::remove_file(paths[1].join("payload.bin")).unwrap();
    let mut committed = open_file(
        &paths[1].join("key.cakp"),
        GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
        0,
        CREATE_NEW,
        Some(&Security::new().unwrap()),
    )
    .unwrap();
    committed
        .write_all(root.protect().unwrap().as_bytes())
        .unwrap();
    committed.sync_all().unwrap();
    drop(committed);
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "matching committed and staged keys do not create an unauthenticated store"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    std::fs::remove_file(paths[1].join("key.cakp.stage")).unwrap();
    std::fs::write(paths[1].join("key.cakp"), b"SYNTHETIC_MALFORMED_KEY").unwrap();
    assert!(matches!(
        layout_open(&paths, false),
        Err(StorageError::KeyUnavailable)
    ));
}

#[test]
fn native_layout_container_inventory_and_lifetime_changes_refuse_before_effects() {
    let (_fixture, paths) = layout_fixture();
    let (_, p, _) = layout_seed(&paths, 1);
    let canary = paths[1].parent().unwrap().join("synthetic-extra-child");
    std::fs::write(&canary, b"SYNTHETIC_CONTAINER_CANARY").unwrap();
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(layout_open(&paths, false), Err(StorageError::UnsafePath)),
        "initial container inventory is exactly one fixed directory child"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    std::fs::remove_file(&canary).unwrap();
    let mut vault = layout_open(&paths, false).unwrap();
    std::fs::write(&canary, b"SYNTHETIC_LATE_CHILD").unwrap();
    assert!(matches!(
        vault.storage.read_latest(&vault.root, p),
        Err(StorageError::ExternalChange)
    ));
    assert!(matches!(
        vault.storage.add(&vault.root, text(), text(), cap()),
        Err(StorageError::ExternalChange)
    ));
    assert_eq!(
        layout_snapshot(&paths[1]),
        before,
        "late ambiguity precedes every protected effect"
    );
    drop(vault);
    std::fs::remove_file(&canary).unwrap();
    let vault = layout_open(&paths, false).unwrap();
    protected_directory(&paths[0]);
    assert!(matches!(
        vault.storage.read_latest(&vault.root, p),
        Err(StorageError::ExternalChange)
    ));
    assert_eq!(layout_snapshot(&paths[1]), before);
}

#[test]
fn native_layout_authenticated_stages_and_torn_recovery_keep_real_methods_reachable() {
    let (_fixture, paths) = layout_fixture();
    protected_directory(paths[1].parent().unwrap());
    let mut disk = Disk::at(&paths[1], true).unwrap();
    let _root = bootstrap(&mut disk, true).unwrap();
    disk.stage("state.bin", b"SYNTHETIC_TORN_WITHOUT_ANCHOR")
        .unwrap();
    drop(disk);
    let before = layout_snapshot(&paths[1]);
    assert!(
        matches!(
            layout_open(&paths, false),
            Err(StorageError::RecoveryRequired)
        ),
        "torn stage without current authenticated anchor is incomplete initialization"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    std::fs::remove_dir_all(&paths[1]).unwrap();
    std::fs::remove_dir(paths[1].parent().unwrap()).unwrap();
    let (root, p, g) = layout_seed(&paths, 1);
    let mut disk = Disk::at(&paths[1], false).unwrap();
    disk.stage("state.bin", b"SYNTHETIC_TORN_WITH_VALID_ANCHOR")
        .unwrap();
    drop(disk);
    let before = layout_snapshot(&paths[1]);
    let mut vault = layout_open(&paths, false).unwrap();
    assert_eq!(vault.recovery(), Recovery::ControlRepairRequired);
    assert!(matches!(
        vault.storage.read_latest(&vault.root, p),
        Err(StorageError::RecoveryRequired)
    ));
    assert_eq!(
        layout_snapshot(&paths[1]),
        before,
        "recognition and torn opening are nonmutating"
    );
    vault.recover_control().unwrap();
    assert_eq!(vault.recovery(), Recovery::Clean);
    assert_eq!(vault.storage.latest(p).unwrap(), g);
    drop(vault);
    let mut disk = Disk::at(&paths[1], false).unwrap();
    let current = disk.read("state.bin").unwrap().unwrap();
    let context = codex_accounts_vault_crypto::EnvelopeContext::new(
        codex_accounts_vault_crypto::Purpose::Journal,
        1,
        root.identifier(),
        root.identifier(),
        0,
    )
    .unwrap();
    let plaintext = root
        .open(
            &context,
            &codex_accounts_vault_crypto::Envelope::from_bytes(&current).unwrap(),
        )
        .unwrap();
    let mut next = crate::codec::read_state(plaintext.expose()).unwrap();
    next.sequence += 1;
    next.parent = Some(digest(&current));
    let sealed = root
        .seal(&context, &crate::codec::state(&next).unwrap())
        .unwrap();
    disk.stage("state.bin", sealed.as_bytes()).unwrap();
    drop(disk);
    let disk = Disk::at_known_location(&paths, false).unwrap();
    assert_eq!(
        std::fs::read(paths[1].join("state.bin")).unwrap(),
        current,
        "private recognition never publishes an authenticated stage"
    );
    assert!(paths[1].join("state.bin.stage").exists());
    drop(disk);
    let vault = layout_open(&paths, false).unwrap();
    assert_eq!(vault.recovery(), Recovery::Clean);
    assert_eq!(
        std::fs::read(paths[1].join("state.bin")).unwrap(),
        sealed.as_bytes(),
        "only admitted existing Storage::open publishes the valid stage"
    );
}

thread_local! { static CREATION_FAULT: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) }; }
pub(super) fn creation_checkpoint(point: u8) -> Result<(), StorageError> {
    CREATION_FAULT.with(|fault| {
        if fault.get() == Some(point) {
            fault.set(None);
            Err(StorageError::Io)
        } else {
            Ok(())
        }
    })
}
struct CreationFault;
impl CreationFault {
    fn arm(point: u8) -> Self {
        CREATION_FAULT.with(|f| f.set(Some(point)));
        Self
    }
}
impl Drop for CreationFault {
    fn drop(&mut self) {
        CREATION_FAULT.with(|f| f.set(None));
    }
}

struct FailSecondControlStage {
    disk: Disk,
    controls: usize,
}
impl Files for FailSecondControlStage {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.disk.read(name)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.disk.list()
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.disk.stage(name, bytes)?;
        if name == "state.bin" {
            self.controls += 1;
            if self.controls == 2 {
                return Err(StorageError::Io);
            }
        }
        Ok(())
    }
    fn publish(
        &mut self,
        name: &str,
        old: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<(), StorageError> {
        self.disk.publish(name, old, bytes)
    }
    fn erase(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.disk.erase(name, bytes)
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestGapsAdverse {
    ReservedAbsent,
    CleanupPrecompleted,
    CleanupSkipped,
    RestorePrecompleted,
    RestoreSkipped,
}

#[cfg(test)]
fn test_gaps_adverse() -> Option<TestGapsAdverse> {
    let Ok(value) = std::env::var("CA05_TEST_GAPS_ADVERSE") else {
        return None;
    };
    Some(match value.as_str() {
        "reserved-absent" => TestGapsAdverse::ReservedAbsent,
        "cleanup-precompleted" => TestGapsAdverse::CleanupPrecompleted,
        "cleanup-skipped" => TestGapsAdverse::CleanupSkipped,
        "restore-precompleted" => TestGapsAdverse::RestorePrecompleted,
        "restore-skipped" => TestGapsAdverse::RestoreSkipped,
        _ => panic!("unknown CA05_TEST_GAPS_ADVERSE selector"),
    })
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingEffect {
    CommitIntentPublish,
    GarbageErase,
}

// Delegates every real Disk operation. The selected operation has already
// completed on the test-owned synthetic fixture when it reports Io, modelling
// uncertain durable outcome without altering production behavior or Disk.
#[cfg(test)]
struct FailPendingEffect {
    disk: Disk,
    effect: PendingEffect,
    injected: bool,
}

#[cfg(test)]
impl Files for FailPendingEffect {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.disk.read(name)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.disk.list()
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.disk.stage(name, bytes)
    }
    fn publish(
        &mut self,
        name: &str,
        old: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<(), StorageError> {
        self.disk.publish(name, old, bytes)?;
        if self.effect == PendingEffect::CommitIntentPublish
            && !self.injected
            && name == "state.bin"
        {
            self.injected = true;
            return Err(StorageError::Io);
        }
        Ok(())
    }
    fn erase(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.disk.erase(name, bytes)?;
        if self.effect == PendingEffect::GarbageErase && !self.injected {
            self.injected = true;
            return Err(StorageError::Io);
        }
        Ok(())
    }
}

#[test]
fn native_layout_creation_failures_and_declared_pending_registry_recovery() {
    for point in 1..=5 {
        let (_fixture, paths) = layout_fixture();
        let fault = CreationFault::arm(point);
        assert!(
            matches!(creation_checkpoint(99), Ok(())),
            "unselected probe cannot inject a failure"
        );
        let result = layout_open(&paths, true);
        drop(fault);
        assert!(matches!(result,Err(StorageError::Io)),"the exact actual post-creation/key/control effect must reach the injected typed failure");
        drop(
            LocationLock::acquire(&Security::new().unwrap(), &known_location_namespace(&paths))
                .unwrap(),
        );
        assert!(
            paths[1].parent().unwrap().is_dir(),
            "uncertain created container is preserved"
        );
        let before = if paths[1].exists() {
            layout_snapshot(&paths[1])
        } else {
            std::collections::BTreeMap::new()
        };
        let retry = layout_open(&paths, true);
        assert!(
            matches!(
                retry,
                Err(StorageError::AlreadyExists | StorageError::RecoveryRequired)
            ),
            "create never completes preserved partial initialization"
        );
        if point < 5 {
            assert!(matches!(
                layout_open(&paths, false),
                Err(StorageError::RecoveryRequired)
            ));
        } else {
            let mut vault = layout_open(&paths, false).unwrap();
            assert_eq!(vault.recovery(),Recovery::CommitPending,"authenticated published first intent is recognized and returns an actual guarded recovery object");
            assert!(matches!(
                vault.storage.add(&vault.root, text(), text(), cap()),
                Err(StorageError::RecoveryRequired)
            ));
            assert_eq!(
                layout_snapshot(&paths[1]),
                before,
                "recognized opening does not complete pending initialization"
            );
            let key = before.get("key.cakp").unwrap().clone();
            vault.reconcile().unwrap();
            assert_eq!(vault.recovery(), Recovery::Clean);
            assert_eq!(std::fs::read(paths[1].join("key.cakp")).unwrap(), key);
            continue;
        }
        if paths[1].exists() {
            assert_eq!(
                layout_snapshot(&paths[1]),
                before,
                "both retries preserve every resulting synthetic byte"
            );
        }
    }
    let (_fixture, paths) = layout_fixture();
    let (root, p, g) = layout_seed(&paths, 1);
    let disk = Disk::at_known_location(&paths, false).unwrap();
    let mut store = Storage::open(FailSecondControlStage { disk, controls: 0 }, &root).unwrap();
    assert!(matches!(
        store.append(&root, p, g, cap()),
        Err(StorageError::Io)
    ));
    assert_eq!(
        store.disk.controls, 2,
        "fault follows the real pending intent and all new registry/object publications"
    );
    drop(store);
    let before = layout_snapshot(&paths[1]);
    let disk = Disk::at_known_location(&paths, false).unwrap();
    assert_eq!(
        layout_snapshot(&paths[1]),
        before,
        "private recognition admits declared new registry but never publishes its control"
    );
    let mut store = Storage::open(disk, &root).unwrap();
    assert_eq!(store.recovery(), Recovery::Clean);
    assert_ne!(
        store.latest(p).unwrap(),
        g,
        "existing opening publishes the genuine new-registry control after recognition"
    );
    let (id, mut effects) =
        crate::engine::coordinator::tests::native_prepare_interrupted(&mut store, &root);
    drop(store);
    let mut vault = layout_open(&paths, false).unwrap();
    assert_eq!(vault.recovery(), Recovery::SwitchPending);
    assert!(matches!(
        vault.storage.add(&vault.root, text(), text(), cap()),
        Err(StorageError::SwitchPending)
    ));
    crate::engine::coordinator::tests::native_finish_interrupted(
        &mut vault.storage,
        &vault.root,
        id,
        &mut effects,
    );

    // COV-05-01 CleanupPending: append first so prune has an actual obsolete
    // generation and authenticated garbage. A later Storage::open establishes
    // the required later-startup condition before the real garbage erase fault.
    {
        let (_fixture, paths) = layout_fixture();
        let (root, profile, first) = layout_seed(&paths, 1);
        let key_before = std::fs::read(paths[1].join("key.cakp")).unwrap();
        let disk = Disk::at_known_location(&paths, false).unwrap();
        let mut store = Storage::open(disk, &root).unwrap();
        let current = store.append(&root, profile, first, cap()).unwrap();
        assert_eq!(store.latest(profile).unwrap(), current);
        drop(store);
        let disk = Disk::at_known_location(&paths, false).unwrap();
        let mut store = Storage::open(
            FailPendingEffect {
                disk,
                effect: PendingEffect::GarbageErase,
                injected: false,
            },
            &root,
        )
        .unwrap();
        assert!(
            matches!(store.prune(&root), Err(StorageError::Io)),
            "actual authenticated garbage erasure reports the injected I/O outcome"
        );
        assert!(
            store.disk.injected,
            "the cleanup fixture reached a real Disk garbage-erase effect"
        );
        drop(store);
        let cleanup_pending = layout_snapshot(&paths[1]);
        let disk = Disk::at_known_location(&paths, false).unwrap();
        crate::engine::recognize_existing(&disk, &root).unwrap();
        assert_eq!(
            layout_snapshot(&paths[1]),
            cleanup_pending,
            "authenticated read-only recognition leaves CleanupPending bytes intact"
        );
        drop(disk);
        if test_gaps_adverse() == Some(TestGapsAdverse::CleanupPrecompleted) {
            let mut completed = layout_open(&paths, false).unwrap();
            completed.reconcile().unwrap();
            assert_eq!(
                completed.recovery(),
                Recovery::Clean,
                "cleanup-precompleted adverse control completed the owned pending fixture"
            );
            drop(completed);
        }
        let mut vault = layout_open(&paths, false).unwrap();
        assert_eq!(
            vault.recovery(),
            Recovery::CleanupPending,
            "layout_open returns the actual guarded CleanupPending Vault"
        );
        assert_eq!(
            layout_snapshot(&paths[1]),
            cleanup_pending,
            "constructor admission does not mutate the authenticated cleanup inventory"
        );
        assert!(
            matches!(
                vault.add(text(), text(), cap()),
                Err(StorageError::RecoveryRequired)
            ),
            "ordinary mutation remains refused while cleanup is pending"
        );
        assert!(matches!(
            vault.latest(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(
            vault.read_latest(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(
            vault.append(profile, current, cap()),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(vault.prune(), Err(StorageError::RecoveryRequired)));
        assert!(matches!(
            vault.remove_inactive(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert_eq!(
            layout_snapshot(&paths[1]),
            cleanup_pending,
            "all ordinary-operation refusals preserve the authenticated pending inventory"
        );
        if test_gaps_adverse() == Some(TestGapsAdverse::CleanupSkipped) {
            assert_eq!(
                vault.recovery(),
                Recovery::CleanupPending,
                "cleanup-skipped adverse control left the owned pending fixture unresolved"
            );
        } else {
            vault.reconcile().unwrap();
        }
        assert_eq!(
            vault.recovery(),
            Recovery::Clean,
            "explicit reconciliation is required to reach clean state"
        );
        assert_eq!(vault.latest(profile).unwrap(), current);
        assert_eq!(
            std::fs::read(paths[1].join("key.cakp")).unwrap(),
            key_before,
            "reconciliation preserves exact wrapped-key bytes"
        );
        let cleaned = layout_snapshot(&paths[1]);
        assert!(
            cleaned.len() < cleanup_pending.len()
                && cleaned.iter().all(|(name, bytes)| {
                    name == "state.bin" || cleanup_pending.get(name) == Some(bytes)
                }),
            "reconciliation preserves authenticated retained inventory and erases only pending garbage"
        );
    }

    // COV-05-01 CommitPending: the real control intent publishes first, then
    // reports Io. The old authenticated registry remains current and the
    // constructor must return a guarded Vault with restore_previous reachable.
    {
        let (_fixture, paths) = layout_fixture();
        let (root, profile, old) = layout_seed(&paths, 1);
        let old_inventory = layout_snapshot(&paths[1]);
        let key_before = std::fs::read(paths[1].join("key.cakp")).unwrap();
        let disk = Disk::at_known_location(&paths, false).unwrap();
        let mut store = Storage::open(
            FailPendingEffect {
                disk,
                effect: PendingEffect::CommitIntentPublish,
                injected: false,
            },
            &root,
        )
        .unwrap();
        assert!(
            matches!(
                store.append(&root, profile, old, cap()),
                Err(StorageError::Io)
            ),
            "the actual pending control intent reports the injected I/O outcome"
        );
        assert!(
            store.disk.injected,
            "the commit fixture reached a real Disk control-intent publication"
        );
        drop(store);
        let commit_pending = layout_snapshot(&paths[1]);
        let disk = Disk::at_known_location(&paths, false).unwrap();
        crate::engine::recognize_existing(&disk, &root).unwrap();
        assert_eq!(
            layout_snapshot(&paths[1]),
            commit_pending,
            "authenticated read-only recognition leaves CommitPending bytes intact"
        );
        drop(disk);
        if test_gaps_adverse() == Some(TestGapsAdverse::RestorePrecompleted) {
            let mut completed = layout_open(&paths, false).unwrap();
            completed.restore_previous().unwrap();
            assert_eq!(
                completed.recovery(),
                Recovery::Clean,
                "restore-precompleted adverse control completed the owned pending fixture"
            );
            drop(completed);
        }
        let mut vault = layout_open(&paths, false).unwrap();
        assert_eq!(
            vault.recovery(),
            Recovery::CommitPending,
            "layout_open returns the actual guarded CommitPending Vault"
        );
        assert_eq!(
            layout_snapshot(&paths[1]),
            commit_pending,
            "constructor admission does not publish or rewrite the pending control"
        );
        assert!(
            matches!(
                vault.add(text(), text(), cap()),
                Err(StorageError::RecoveryRequired)
            ),
            "ordinary mutation remains refused while commit restoration is pending"
        );
        assert!(matches!(
            vault.latest(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(
            vault.read_latest(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(
            vault.append(profile, old, cap()),
            Err(StorageError::RecoveryRequired)
        ));
        assert!(matches!(vault.prune(), Err(StorageError::RecoveryRequired)));
        assert!(matches!(
            vault.remove_inactive(profile),
            Err(StorageError::RecoveryRequired)
        ));
        assert_eq!(
            layout_snapshot(&paths[1]),
            commit_pending,
            "all ordinary-operation refusals preserve the authenticated pending inventory"
        );
        if test_gaps_adverse() == Some(TestGapsAdverse::RestoreSkipped) {
            assert_eq!(
                vault.recovery(),
                Recovery::CommitPending,
                "restore-skipped adverse control left the owned pending fixture unresolved"
            );
        } else {
            vault.restore_previous().unwrap();
        }
        assert_eq!(
            vault.recovery(),
            Recovery::Clean,
            "explicit restore_previous is required to reach clean state"
        );
        assert_eq!(vault.latest(profile).unwrap(), old);
        assert_eq!(
            std::fs::read(paths[1].join("key.cakp")).unwrap(),
            key_before,
            "restore_previous preserves exact wrapped-key bytes"
        );
        let restored = layout_snapshot(&paths[1]);
        for (name, bytes) in &old_inventory {
            if name != "state.bin" {
                assert_eq!(
                    restored.get(name),
                    Some(bytes),
                    "restore_previous preserves the old authenticated inventory"
                );
            }
        }
        assert!(
            restored
                .keys()
                .all(|name| name == "state.bin" || old_inventory.contains_key(name)),
            "restore_previous leaves no pending object outside the old inventory"
        );
    }
}

#[test]
fn native_layout_descriptor_drift_and_legacy_container_absence_are_lifetime_guards() {
    let (fixture, mut paths) = layout_fixture();
    let legacy_parent = fixture.1.join("synthetic-observed-legacy-parent");
    protected_directory(&legacy_parent);
    paths[0] = legacy_parent.join(VAULT_CHILD);
    synthetic_dacl(&legacy_parent, "(A;;0x00000002;;;WD)");
    let mut vault = layout_open(&paths, true).unwrap();
    assert!(
        matches!(layout_open(&paths, false), Err(StorageError::Busy)),
        "actual constructor acquires namespace before metadata/selection"
    );
    assert!(
        matches!(Disk::at(&paths[1], false), Err(StorageError::Busy)),
        "actual recognized namespace retains independent per-root ownership"
    );
    let before = layout_snapshot(&paths[1]);
    // Both descriptors have the same supported unsafe-grant classification.
    // Only the observed byte comparison detects this test-owned DACL change.
    synthetic_dacl(&legacy_parent, "(A;;0x00000006;;;WD)");
    assert!(
        matches!(
            vault.storage.disk.read("key.cakp"),
            Err(StorageError::ExternalChange)
        ),
        "same-tag descriptor drift refuses protected read"
    );
    assert!(
        matches!(
            vault.storage.add(&vault.root, text(), text(), cap()),
            Err(StorageError::ExternalChange)
        ),
        "same-tag descriptor drift refuses before a protected effect"
    );
    assert_eq!(layout_snapshot(&paths[1]), before);
    drop(vault);
    let (_fixture, paths) = layout_fixture();
    let (root, p, _) = layout_seed(&paths, 0);
    let vault = layout_open(&paths, false).unwrap();
    let before = layout_snapshot(&paths[0]);
    protected_directory(paths[1].parent().unwrap());
    assert!(
        matches!(
            vault.storage.read_latest(&vault.root, p),
            Err(StorageError::ExternalChange)
        ),
        "sole legacy lifetime retains actual checked container absence"
    );
    assert_eq!(layout_snapshot(&paths[0]), before);
    drop(vault);
    std::fs::remove_dir(paths[1].parent().unwrap()).unwrap();
    let mut disk = Disk::at(&paths[0], false).unwrap();
    disk.stage("state.bin", b"SYNTHETIC_RECOGNITION_TORN_STAGE")
        .unwrap();
    let before = layout_snapshot(&paths[0]);
    crate::engine::recognize_existing(&disk, &root).unwrap();
    assert_eq!(
        layout_snapshot(&paths[0]),
        before,
        "direct private engine recognition performs no creation/publication/erase"
    );
    drop(disk);
}

struct StopFirstControlStage(Disk);
impl Files for StopFirstControlStage {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.0.read(name)
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.0.list()
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.0.stage(name, bytes)?;
        if name == "state.bin" {
            Err(StorageError::Io)
        } else {
            Ok(())
        }
    }
    fn publish(
        &mut self,
        name: &str,
        old: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<(), StorageError> {
        self.0.publish(name, old, bytes)
    }
    fn erase(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        self.0.erase(name, bytes)
    }
}
#[test]
fn native_layout_valid_staged_initial_anchor_reconciles_without_key_regeneration() {
    let (_fixture, paths) = layout_fixture();
    protected_directory(paths[1].parent().unwrap());
    let mut disk = Disk::at(&paths[1], true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    assert!(
        matches!(
            Storage::create(StopFirstControlStage(disk), &root),
            Err(StorageError::Io)
        ),
        "stop only after real authenticated first control staging"
    );
    let before = layout_snapshot(&paths[1]);
    assert!(!before.contains_key("state.bin"));
    assert!(before.contains_key("state.bin.stage"));
    let disk = Disk::at_known_location(&paths, false)
        .expect("valid staged initial authenticated anchor must recognize read-only");
    crate::engine::recognize_existing(&disk, &root).unwrap();
    assert_eq!(
        layout_snapshot(&paths[1]),
        before,
        "stage-only recognition never publishes control or key"
    );
    let mut vault = finish_vault(disk, false).unwrap();
    assert_eq!(vault.recovery(), Recovery::CommitPending);
    assert!(matches!(
        vault.storage.add(&vault.root, text(), text(), cap()),
        Err(StorageError::RecoveryRequired)
    ));
    vault.reconcile().unwrap();
    assert_eq!(vault.recovery(), Recovery::Clean);
    assert_eq!(
        std::fs::read(paths[1].join("key.cakp")).unwrap(),
        *before.get("key.cakp").unwrap(),
        "explicit recognized-store recovery keeps the exact committed wrapped key"
    );
}

// TASK-04 uses separate names and only the exact newly-owned fixture. The name
// includes current-user identity internally, but neither names nor SIDs are logged.
fn location_fixture() -> Sandbox {
    let fixture = Sandbox::new();
    if let Some(journal) = std::env::var_os("CA04_LOCATION_SYNTHETIC_JOURNAL") {
        // Optional private author capture, never a behavior/authority bypass.
        // Retain each exact exclusively-owned root before any adverse operation.
        let encoded: String = fixture
            .1
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal)
            .unwrap();
        writeln!(
            file,
            "{{\"root_utf16_hex\":\"{encoded}\",\"pid\":{}}}",
            std::process::id()
        )
        .unwrap();
        file.sync_all().unwrap();
    }
    fixture
}
fn location_namespace(fixture: &Sandbox) -> Vec<u8> {
    fixture
        .1
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
fn location_slots(fixture: &Sandbox) -> [PathBuf; 2] {
    [
        fixture.1.join("synthetic-legacy-slot"),
        fixture.1.join("synthetic-profile-slot"),
    ]
}
fn location_mutex_name(security: &Security, namespace: &[u8]) -> Vec<u16> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"CodexAccounts/VaultLocation/v1\0");
    hash.update((security.user.len() as u32).to_le_bytes());
    hash.update(&security.user);
    hash.update(namespace);
    let hex: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    format!("Global\\CodexAccounts.VaultLocation.v1.{hex}\0")
        .encode_utf16()
        .collect()
}
fn protected_location_mutex(handle: HANDLE, security: &Security) -> bool {
    let (mut owner, mut dacl, mut sd) = (null_mut(), null_mut(), null_mut());
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut sd,
        )
    };
    let sd = Allocation(sd);
    if status != 0 || sid_copy(owner).ok().as_ref() != Some(&security.user) || dacl.is_null() {
        return false;
    }
    let (mut control, mut revision) = (0, 0);
    if unsafe { GetSecurityDescriptorControl(sd.0, &mut control, &mut revision) } == 0
        || control & SE_DACL_PROTECTED == 0
        || unsafe { IsValidAcl(dacl) } == 0
        || unsafe { (*dacl).AceCount } != 2
    {
        return false;
    }
    let mut found = [false; 2];
    for index in 0..2 {
        let mut ace = null_mut();
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 || ace.is_null() {
            return false;
        }
        let header = unsafe { &*ace.cast::<ACE_HEADER>() };
        if u32::from(header.AceType) != ACCESS_ALLOWED_ACE_TYPE
            || header.AceFlags != 0
            || usize::from(header.AceSize) < std::mem::size_of::<ACCESS_ALLOWED_ACE>()
        {
            return false;
        }
        let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
        let sid = (&allowed.SidStart as *const u32).cast::<u8>();
        let size = 8 + 4 * usize::from(unsafe { *sid.add(1) });
        if size > 68
            || std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart) + size
                > usize::from(header.AceSize)
            || allowed.Mask != READ_CONTROL | SYNCHRONIZE | 1
        {
            return false;
        }
        let Ok(sid) = sid_copy(sid.cast_mut().cast()) else {
            return false;
        };
        if sid == security.user && !found[0] {
            found[0] = true;
        } else if sid == security.system && !found[1] {
            found[1] = true;
        } else {
            return false;
        }
    }
    found == [true, true]
}

#[test]
fn native_location_mutex_recursive_and_protected_object() {
    use windows_sys::Win32::System::Threading::CreateMutexExW;
    let fixture = Sandbox::new();
    let namespace = location_namespace(&fixture);
    let security = Security::new().unwrap();
    let guard = LocationLock::acquire(&security, &namespace).unwrap();
    assert!(
        matches!(
            LocationLock::acquire(&security, &namespace),
            Err(StorageError::Busy)
        ),
        "recursive location acquisition must refuse rather than increment kernel ownership"
    );
    assert!(
        protected_location_mutex(guard.0 .0, &security),
        "location object is strictly current-user/SYSTEM protected"
    );
    drop(guard);
    let second = LocationLock::acquire(&security, &namespace).unwrap();
    drop(second);
    // Deliberately broad protection at creation of a fresh test-only kernel object.
    let broad_name = location_mutex_name(&security, &[namespace.as_slice(), b"-broad"].concat());
    let broad_sddl: Vec<u16> = "D:P(A;;GA;;;WD)\0".encode_utf16().collect();
    let mut broad_sd = null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                broad_sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut broad_sd,
                null_mut(),
            )
        },
        0
    );
    let broad_sd = Allocation(broad_sd);
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: broad_sd.0,
        bInheritHandle: 0,
    };
    let broad = Token(unsafe {
        CreateMutexExW(
            &attrs,
            broad_name.as_ptr(),
            0,
            READ_CONTROL | SYNCHRONIZE | 1,
        )
    });
    assert!(!broad.0.is_null(), "fresh broad synthetic mutex exists");
    assert!(
        matches!(
            LocationLock::acquire(&security, &[namespace.as_slice(), b"-broad"].concat()),
            Err(StorageError::UnsafePath)
        ),
        "existing broad kernel protection must refuse without repair"
    );
}

struct LocationChild(std::process::Child);
impl LocationChild {
    fn start(fixture: &Sandbox, mode: &str) -> Self {
        std::fs::write(
            fixture.1.join("location-marker"),
            b"SYNTHETIC_LOCATION_ONLY",
        )
        .unwrap();
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "native::tests::native_location_child"])
                .env("CA04_LOCATION_SYNTHETIC_ROOT", &fixture.1)
                .env("CA04_LOCATION_SYNTHETIC_MODE", mode)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    fn ready(&mut self, fixture: &Sandbox) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !fixture.1.join("location-ready").exists() {
            assert!(
                self.0.try_wait().unwrap().is_none(),
                "owned location child remains live until readiness"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "owned location child readiness bound"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    fn finish(&mut self, fixture: &Sandbox, expected: i32) {
        std::fs::write(fixture.1.join("location-finish"), b"SYNTHETIC_FINISH").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let status = loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owned location child exit bound"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(expected));
    }
}
impl Drop for LocationChild {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().unwrap();
        }
        self.0.wait().unwrap();
    }
}
#[test]
fn native_location_child() {
    let Some(path) = std::env::var_os("CA04_LOCATION_SYNTHETIC_ROOT") else {
        return;
    };
    let root = PathBuf::from(path);
    validate_path(&root).unwrap();
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("ca03c-test-"));
    assert_eq!(
        std::fs::read(root.join("location-marker")).unwrap(),
        b"SYNTHETIC_LOCATION_ONLY"
    );
    let namespace: Vec<u8> = root
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect();
    let guard = LocationLock::acquire(&Security::new().unwrap(), &namespace).unwrap();
    std::fs::write(root.join("location-ready"), b"SYNTHETIC_READY").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !root.join("location-finish").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "finite synthetic child lifetime"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if std::env::var("CA04_LOCATION_SYNTHETIC_MODE").unwrap() == "abandon" {
        std::process::exit(73);
    }
    drop(guard);
}
#[test]
fn native_location_mutex_process_contention_release_and_abandonment() {
    use windows_sys::Win32::System::Threading::OpenMutexW;
    let fixture = Sandbox::new();
    let security = Security::new().unwrap();
    let namespace = location_namespace(&fixture);
    let mut child = LocationChild::start(&fixture, "normal");
    child.ready(&fixture);
    assert!(
        matches!(
            LocationLock::acquire(&security, &namespace),
            Err(StorageError::Busy)
        ),
        "independent process ownership blocks both-slot namespace"
    );
    child.finish(&fixture, 0);
    drop(LocationLock::acquire(&security, &namespace).unwrap());
    std::fs::remove_file(fixture.1.join("location-ready")).unwrap();
    std::fs::remove_file(fixture.1.join("location-finish")).unwrap();
    let mut child = LocationChild::start(&fixture, "abandon");
    child.ready(&fixture);
    // Retain an observer handle so the real abandoned kernel object survives
    // the exiting owner's last handle; it is neither ownership nor authority.
    let observer = Token(unsafe {
        OpenMutexW(
            READ_CONTROL | SYNCHRONIZE | 1,
            0,
            location_mutex_name(&security, &namespace).as_ptr(),
        )
    });
    assert!(
        !observer.0.is_null(),
        "named location observer opens before abrupt owner exit"
    );
    child.finish(&fixture, 73);
    assert!(
        matches!(
            LocationLock::acquire(&security, &namespace),
            Err(StorageError::RecoveryRequired)
        ),
        "actual abandoned ownership refuses and requires recovery"
    );
    drop(LocationLock::acquire(&security, &namespace).unwrap());
    drop(observer);
}

#[test]
fn native_location_read_only_probe_refuses_denial_and_wrong_type() {
    struct OwnedDeniedDirectory(File);
    impl Drop for OwnedDeniedDirectory {
        fn drop(&mut self) {
            let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
            // DELETE was granted at creation. Remove this exact owned empty
            // directory through that handle, without querying denied attributes
            // or repairing protection, including on an assertion unwind.
            assert_ne!(
                unsafe {
                    SetFileInformationByHandle(
                        self.0.as_raw_handle(),
                        FileDispositionInfo,
                        (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                        std::mem::size_of_val(&disposition) as u32,
                    )
                },
                0
            );
        }
    }
    let fixture = location_fixture();
    let owned_root = fixture.1.clone();
    let ancestor = open_file(
        &fixture.1,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
        None,
    )
    .unwrap();
    let ancestor_security = security_bytes(&ancestor);
    let canary = fixture.1.join("synthetic-denial-ancestor-canary");
    std::fs::write(&canary, b"SYNTHETIC_DENIAL_ANCESTOR_UNCHANGED").unwrap();
    let slots = location_slots(&fixture);
    std::fs::write(&slots[0], b"SYNTHETIC_WRONG_OBJECT").unwrap();
    let before = std::fs::read(&slots[0]).unwrap();
    assert!(
        matches!(probe_slots(&slots), Err(StorageError::UnsafePath)),
        "file is never a directory/absent slot"
    );
    assert!(
        std::fs::read(&slots[0]).unwrap() == before,
        "probe leaves wrong-type bytes unchanged"
    );
    std::fs::remove_file(&slots[0]).unwrap();
    let mut denied_sd = null_mut();
    let sddl: Vec<u16> = "D:P(D;;0x00000001;;;WD)(A;;0x00110080;;;WD)(A;;GA;;;SY)\0"
        .encode_utf16()
        .collect();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut denied_sd,
                null_mut(),
            )
        },
        0
    );
    let sd = Allocation(denied_sd);
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    assert_ne!(
        unsafe { CreateDirectoryW(wide(&slots[0]).unwrap().as_ptr(), &attrs) },
        0
    );
    let denied = OwnedDeniedDirectory(
        open_file(
            &slots[0],
            DELETE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            None,
        )
        .unwrap(),
    );
    assert!(
        matches!(probe_slots(&slots), Err(StorageError::AccessDenied)),
        "metadata denial is never inferred absence"
    );
    drop(denied);
    assert!(
        probe_slots(&slots).unwrap() == [None, None],
        "only verified absent leaves yield absence"
    );
    let missing_parent = [
        fixture.1.join("missing-parent").join("slot"),
        slots[1].clone(),
    ];
    assert!(
        matches!(
            probe_slots(&missing_parent),
            Err(StorageError::ActiveStateUnknown)
        ),
        "missing unchecked ancestry is indeterminate, never absent"
    );
    assert!(
        std::fs::read(&canary).unwrap() == b"SYNTHETIC_DENIAL_ANCESTOR_UNCHANGED",
        "denied slot cleanup preserves ancestor canary bytes"
    );
    assert!(
        security_bytes(&ancestor) == ancestor_security,
        "denied slot cleanup preserves ancestor security"
    );
    drop(ancestor);
    drop(fixture);
    assert!(
        !owned_root.exists(),
        "exact journaled synthetic denial root is removed"
    );
}

#[test]
fn native_location_collision_and_replacement_refuse_before_access() {
    let fixture = Sandbox::new();
    let slots = location_slots(&fixture);
    let namespace = location_namespace(&fixture);
    let mut disk = Disk::at_location(&slots, true, &namespace).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let mut storage = Storage::create(disk, &root).unwrap();
    let disk = &mut storage.disk;
    let original_key = disk.read("key.cakp").unwrap().unwrap();
    let original_control = disk.read("state.bin").unwrap().unwrap();
    disk.stage("synthetic.bin", b"SYNTHETIC_ORIGINAL").unwrap();
    disk.publish("synthetic.bin", None, b"SYNTHETIC_ORIGINAL")
        .unwrap();
    std::fs::create_dir(&slots[0]).unwrap();
    assert!(
        matches!(
            disk.read("synthetic.bin"),
            Err(StorageError::ExternalChange)
        ),
        "new competing root blocks read before selected bytes are returned"
    );
    assert!(
        matches!(
            disk.stage("other.bin", b"SYNTHETIC_NEW"),
            Err(StorageError::ExternalChange)
        ),
        "new competing root blocks write before staging"
    );
    assert!(
        !slots[1].join("other.bin.stage").exists(),
        "collision refuses without new protected payload"
    );
    assert!(
        std::fs::read(slots[1].join("synthetic.bin")).unwrap() == b"SYNTHETIC_ORIGINAL",
        "collision preserves selected bytes"
    );
    assert!(
        std::fs::read(slots[1].join("key.cakp")).unwrap() == original_key,
        "collision preserves the actual synthetic DPAPI-protected root key bytes"
    );
    assert!(
        std::fs::read(slots[1].join("state.bin")).unwrap() == original_control,
        "collision preserves the actual encrypted synthetic control bytes"
    );
    drop(storage);
    let observed = probe_slots(&slots).unwrap();
    let security = Security::new().unwrap();
    let mut watch = SlotWatch::new(&slots[0], &security).unwrap();
    assert!(
        std::fs::rename(&slots[0], fixture.1.join("synthetic-old-root")).is_err(),
        "retained native ancestor sharing prevents external directory replacement"
    );
    watch.revalidate(&security).unwrap();
    // Test-only comparison subcase: explicitly release our owned ancestor pins
    // so the actual replacement can occur, retaining the prior leaf identity.
    // Production SlotWatch always retains them; no production sharing is relaxed.
    watch.parents.clear();
    std::fs::rename(&slots[0], fixture.1.join("synthetic-old-root")).unwrap();
    std::fs::create_dir(&slots[0]).unwrap();
    assert!(
        probe_slots(&slots).unwrap() != observed,
        "replacement changes actual native object identity"
    );
    assert!(
        matches!(
            watch.revalidate(&security),
            Err(StorageError::ExternalChange)
        ),
        "a replaced existing observed slot invalidates its retained identity guard"
    );
    assert!(
        matches!(
            Disk::at_location(&slots, false, &namespace),
            Err(StorageError::ActiveStateUnknown)
        ),
        "both roots never select a fallback"
    );
    drop(watch);
}
#[test]
fn native_location_failure_releases_mutex_and_retains_root_lock() {
    let fixture = Sandbox::new();
    let slots = location_slots(&fixture);
    let namespace = location_namespace(&fixture);
    let disk = Disk::at_location(&slots, true, &namespace).unwrap();
    let security = Security::new().unwrap();
    assert!(
        matches!(
            LocationLock::acquire(&security, &namespace),
            Err(StorageError::Busy)
        ),
        "Disk retains the location guard for its complete lifetime"
    );
    assert!(
        matches!(Disk::at(&slots[1], false), Err(StorageError::Busy)),
        "legacy per-root lock remains held too"
    );
    drop(disk);
    drop(LocationLock::acquire(&security, &namespace).unwrap());
    assert!(
        matches!(
            Disk::at_location(&slots, true, &namespace),
            Err(StorageError::AlreadyExists)
        ),
        "failed create refuses existing slot"
    );
    drop(LocationLock::acquire(&security, &namespace).unwrap());
    std::fs::remove_dir_all(&slots[1]).unwrap();
    std::fs::write(&slots[0], b"SYNTHETIC_COLLISION_FILE").unwrap();
    assert!(
        matches!(
            Disk::at_location(&slots, false, &namespace),
            Err(StorageError::UnsafePath)
        ),
        "failed probe refuses without losing reservation ownership"
    );
    drop(LocationLock::acquire(&security, &namespace).unwrap());
    std::fs::remove_file(&slots[0]).unwrap();
    std::fs::create_dir(&slots[1]).unwrap();
    assert!(
        matches!(
            Disk::at_location(&slots, false, &namespace),
            Err(StorageError::UnsafePath)
        ),
        "strict selected-root failure releases the already-acquired location owner"
    );
    drop(LocationLock::acquire(&security, &namespace).unwrap());
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
fn native_hardened_root_requires_its_passphrase_and_windows_user() {
    let dir = Sandbox::new();
    let mut disk = Disk::at(&dir.0, true).unwrap();
    let (root, protection) =
        bootstrap_protected(&mut disk, true, Some(b"correct horse battery staple")).unwrap();
    let identifier = root.identifier();
    assert_eq!(protection, crate::VaultProtection::Passphrase);
    drop(root);
    drop(disk);

    let mut disk = Disk::at(&dir.0, false).unwrap();
    assert!(matches!(
        bootstrap_protected(&mut disk, false, None),
        Err(StorageError::PassphraseRequired)
    ));
    assert!(matches!(
        bootstrap_protected(&mut disk, false, Some(b"wrong passphrase but long enough")),
        Err(StorageError::InvalidPassphrase)
    ));
    let (root, protection) =
        bootstrap_protected(&mut disk, false, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(root.identifier(), identifier);
    assert_eq!(protection, crate::VaultProtection::Passphrase);
}
#[test]
fn native_known_location_reopens_hardened_root_and_finishes_both_rewrap_directions() {
    let (_fixture, paths) = layout_fixture();
    let mut vault = layout_open(&paths, true).unwrap();
    let identifier = vault.root.identifier();
    let hardened = vault
        .root
        .protect_with_passphrase(b"correct horse battery staple")
        .unwrap();
    vault
        .storage
        .disk
        .stage("key.cakp", hardened.as_bytes())
        .unwrap();
    drop(vault);

    assert!(matches!(
        layout_open_with_passphrase(&paths, None),
        Err(StorageError::PassphraseRequired)
    ));
    let mut vault =
        layout_open_with_passphrase(&paths, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(vault.root.identifier(), identifier);
    assert_eq!(vault.protection, crate::VaultProtection::Passphrase);
    assert!(vault.storage.disk.read("key.cakp.stage").unwrap().is_none());
    let ordinary = vault.root.protect().unwrap();
    vault
        .storage
        .disk
        .stage("key.cakp", ordinary.as_bytes())
        .unwrap();
    drop(vault);

    let vault = layout_open_with_passphrase(&paths, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(vault.root.identifier(), identifier);
    assert_eq!(vault.protection, crate::VaultProtection::WindowsUser);
    assert!(vault.storage.disk.read("key.cakp.stage").unwrap().is_none());
}

#[test]
fn native_key_rewrap_successfully_enables_and_removes_passphrase_protection() {
    let (_fixture, paths) = layout_fixture();
    let mut vault = layout_open(&paths, true).unwrap();
    let identifier = vault.root.identifier();
    replace_vault_protection(
        &mut vault,
        Some(b"correct horse battery staple"),
        crate::VaultProtection::Passphrase,
    )
    .unwrap();
    assert_eq!(vault.protection, crate::VaultProtection::Passphrase);
    drop(vault);

    let mut vault =
        layout_open_with_passphrase(&paths, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(vault.root.identifier(), identifier);
    replace_vault_protection(&mut vault, None, crate::VaultProtection::WindowsUser).unwrap();
    assert_eq!(vault.protection, crate::VaultProtection::WindowsUser);
    assert!(vault.storage.disk.read("key.cakp.stage").unwrap().is_none());
    drop(vault);

    let vault = layout_open_with_passphrase(&paths, None).unwrap();
    assert_eq!(vault.root.identifier(), identifier);
    assert_eq!(vault.protection, crate::VaultProtection::WindowsUser);
}

#[test]
fn native_key_rewrap_cleans_a_failed_pre_rename_stage() {
    let (_fixture, paths) = layout_fixture();
    let mut vault = layout_open(&paths, true).unwrap();
    let fault = CreationFault::arm(7);
    let result = replace_vault_protection(
        &mut vault,
        Some(b"correct horse battery staple"),
        crate::VaultProtection::Passphrase,
    );
    drop(fault);
    assert_eq!(result, Err(StorageError::Io));
    assert_eq!(vault.protection, crate::VaultProtection::WindowsUser);
    assert!(vault.storage.disk.read("key.cakp.stage").unwrap().is_none());
    drop(vault);
    assert!(layout_open_with_passphrase(&paths, None).is_ok());
}

#[test]
fn native_key_rewrap_reports_a_post_rename_failure_as_uncertain() {
    let (_fixture, paths) = layout_fixture();
    let mut vault = layout_open(&paths, true).unwrap();
    let fault = CreationFault::arm(6);
    let result = replace_vault_protection(
        &mut vault,
        Some(b"correct horse battery staple"),
        crate::VaultProtection::Passphrase,
    );
    drop(fault);
    assert_eq!(result, Err(StorageError::ProtectionChangeUncertain));
    assert_eq!(vault.protection, crate::VaultProtection::Passphrase);
    assert!(vault.storage.disk.read("key.cakp.stage").unwrap().is_none());
    drop(vault);
    assert!(layout_open_with_passphrase(&paths, Some(b"correct horse battery staple")).is_ok());
}
#[test]
fn native_torn_hardening_rewrap_finishes_only_with_the_same_root_and_passphrase() {
    let dir = Sandbox::new();
    let mut disk = Disk::at(&dir.0, true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let identifier = root.identifier();
    let hardened = root
        .protect_with_passphrase(b"correct horse battery staple")
        .unwrap();
    disk.stage("key.cakp", hardened.as_bytes()).unwrap();
    drop(disk);

    let mut disk = Disk::at(&dir.0, false).unwrap();
    assert!(matches!(
        bootstrap_protected(&mut disk, false, None),
        Err(StorageError::PassphraseRequired)
    ));
    let (root, protection) =
        bootstrap_protected(&mut disk, false, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(root.identifier(), identifier);
    assert_eq!(protection, crate::VaultProtection::Passphrase);
    assert!(disk.read("key.cakp.stage").unwrap().is_none());

    let ordinary = root.protect().unwrap();
    disk.stage("key.cakp", ordinary.as_bytes()).unwrap();
    drop(disk);
    let mut disk = Disk::at(&dir.0, false).unwrap();
    let (root, protection) =
        bootstrap_protected(&mut disk, false, Some(b"correct horse battery staple")).unwrap();
    assert_eq!(root.identifier(), identifier);
    assert_eq!(protection, crate::VaultProtection::WindowsUser);
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
    let mut unicode = Sandbox::new();
    unicode.0 = unicode.0.with_file_name("ca03c-test-vault-Ω");
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

#[test]
fn native_preflight_reports_only_sanitized_security_conditions() {
    let dir = Sandbox::new();
    validate_path(&dir.0).expect("path syntax");
    let security = Security::new().expect("current token and creation descriptor");
    let mut paths: Vec<_> = dir.0.ancestors().skip(1).collect();
    paths.reverse();
    let mut held = vec![];
    for (index, path) in paths.iter().enumerate() {
        let f = open_file(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )
        .unwrap_or_else(|e| panic!("ancestor-open {index}: {e:?}"));
        check_object(&f, true).unwrap_or_else(|e| panic!("ancestor-shape {index}: {e:?}"));
        if let Err(e) = security.check(&f, false) {
            let mut owner = null_mut();
            let mut acl = null_mut();
            let mut descriptor = null_mut();
            assert_eq!(
                unsafe {
                    GetSecurityInfo(
                        f.as_raw_handle(),
                        SE_FILE_OBJECT,
                        OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                        &mut owner,
                        null_mut(),
                        &mut acl,
                        null_mut(),
                        &mut descriptor,
                    )
                },
                0
            );
            let _descriptor = Allocation(descriptor);
            let classify = |s: &[u8]| {
                if s == security.user {
                    "current"
                } else if s == security.system {
                    "system"
                } else if s == security.admins {
                    "administrators"
                } else if s == security.installer {
                    "windows_servicing"
                } else {
                    "other"
                }
            };
            let owner = sid_copy(owner).unwrap();
            let mut facts = vec![];
            if !acl.is_null() && unsafe { IsValidAcl(acl) } != 0 {
                for i in 0..unsafe { (*acl).AceCount }.min(32) {
                    let mut ace = null_mut();
                    assert_ne!(unsafe { GetAce(acl, u32::from(i), &mut ace) }, 0);
                    assert!(!ace.is_null());
                    let header = unsafe { &*ace.cast::<ACE_HEADER>() };
                    if u32::from(header.AceType) == ACCESS_ALLOWED_ACE_TYPE
                        && usize::from(header.AceSize) >= std::mem::size_of::<ACCESS_ALLOWED_ACE>()
                    {
                        let a = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
                        let offset = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
                        assert!(usize::from(header.AceSize) >= offset + 8);
                        let ptr = (&a.SidStart as *const u32).cast::<u8>();
                        let size = 8 + 4 * usize::from(unsafe { *ptr.add(1) });
                        assert!(size <= 68 && offset + size <= usize::from(header.AceSize));
                        let sid = sid_copy(ptr.cast_mut().cast()).unwrap();
                        facts.push((classify(&sid), header.AceFlags, a.Mask));
                    }
                }
            }
            panic!("ancestor-security {index}: {e:?}; owner_class={}; access_classes_flags_masks={facts:?}", classify(&owner));
        }
        held.push(f);
    }
    let parent = held.last().unwrap();
    sync::registered_sync_check(&held).expect("complete registered sync-root inventory");
    if let Err(e) = cloud_check(parent, &held) {
        let mut buffer = [0u64; 256];
        let mut size = 0;
        let code = unsafe {
            CfGetSyncRootInfoByHandle(
                parent.as_raw_handle(),
                CF_SYNC_ROOT_INFO_BASIC,
                buffer.as_mut_ptr().cast(),
                std::mem::size_of_val(&buffer) as u32,
                &mut size,
            )
        };
        panic!("cloud-query: {e:?}; hresult={code:08x}; required_bytes={size}");
    }
    drop(held);
    let d = Disk::at(&dir.0, true).expect("root creation after passing ancestor preflight");
    drop(d);
}

#[test]
fn native_ancestor_owner_allowlist_is_exact_not_any_service_or_user() {
    let s = Security::new().unwrap();
    for sid in [&s.user, &s.system, &s.admins, &s.installer] {
        assert!(s.ancestor_owner_allowed(sid));
    }
    for kind in [
        WinWorldSid,
        WinAuthenticatedUserSid,
        WinBuiltinUsersSid,
        WinLocalServiceSid,
        WinNetworkServiceSid,
    ] {
        assert!(!s.ancestor_owner_allowed(&known_sid(kind).unwrap()));
    }
    let mut unrelated = s.installer.clone();
    let last = unrelated.len() - 1;
    unrelated[last] ^= 1;
    assert!(!s.ancestor_owner_allowed(&unrelated));
}

#[test]
fn native_switch_journals_persist_reopen_and_prune_without_live_effects() {
    let dir = Sandbox::new();
    let mut disk = Disk::at(&dir.0, true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let mut store = Storage::create(disk, &root).unwrap();
    crate::engine::coordinator::tests::native_roundtrip(&mut store, &root);
    let expected = store.switch_status().unwrap();
    assert_eq!(expected.len(), 2);
    drop(store);
    drop(root);
    let mut disk = Disk::at(&dir.0, false).unwrap();
    let root = bootstrap(&mut disk, false).unwrap();
    let mut store = Storage::open(disk, &root).unwrap();
    assert_eq!(store.switch_status().unwrap(), expected);
    store.prune(&root).unwrap();
    assert!(store.switch_status().unwrap().is_empty());
}

#[test]
fn native_interrupted_journal_blocks_dispatch_and_preserves_synthetic_helper_refresh() {
    let dir = Sandbox::new();
    let mut disk = Disk::at(&dir.0, true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let mut store = Storage::create(disk, &root).unwrap();
    let (id, mut effects) =
        crate::engine::coordinator::tests::native_prepare_interrupted(&mut store, &root);
    drop(store);
    drop(root);
    let mut disk = Disk::at(&dir.0, false).unwrap();
    let root = bootstrap(&mut disk, false).unwrap();
    let mut store = Storage::open(disk, &root).unwrap();
    crate::engine::coordinator::tests::native_finish_interrupted(
        &mut store,
        &root,
        id,
        &mut effects,
    );
    let expected = store.switch_status().unwrap();
    drop(store);
    let disk = Disk::at(&dir.0, false).unwrap();
    let store = Storage::open(disk, &root).unwrap();
    assert_eq!(store.switch_status().unwrap(), expected);
}
