//! Native creation contract with fixed, non-sensitive failure stages.
use super::super::*;

#[test]
fn native_creation_checks_parent_volume_descriptor_and_lock_separately() {
    let fixture = super::super::tests::Sandbox::new();
    validate_path(&fixture.0).expect("path syntax");
    let security = Security::new().expect("creation security descriptor");
    let mut paths: Vec<_> = fixture.0.ancestors().skip(1).collect();
    paths.reverse();
    let mut dirs = Vec::new();
    for path in paths {
        let f = open_file(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )
        .expect("ancestor metadata handle");
        check_object(&f, true).expect("ancestor object shape");
        security.check(&f, false).expect("ancestor access policy");
        dirs.push(f);
    }
    let parent = dirs.last().unwrap();
    let mut owner = null_mut();
    let mut descriptor = null_mut();
    assert_eq!(
        unsafe {
            GetSecurityInfo(
                parent.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                null_mut(),
                null_mut(),
                &mut descriptor,
            )
        },
        0,
        "direct-parent security query"
    );
    let _descriptor = Allocation(descriptor);
    assert!(
        sid_copy(owner).unwrap() == security.user,
        "direct parent must be user-owned"
    );
    cloud_check(parent, &dirs).expect("parent cloud and registered-sync-root observation");
    let mut fs = [0u16; 32];
    let mut flags = 0;
    assert_ne!(
        unsafe {
            GetVolumeInformationByHandleW(
                parent.as_raw_handle(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                &mut flags,
                fs.as_mut_ptr(),
                fs.len() as u32,
            )
        },
        0,
        "volume information query"
    );
    let end = fs.iter().position(|v| *v == 0).unwrap();
    assert!(
        String::from_utf16(&fs[..end]).unwrap() == "NTFS",
        "NTFS required"
    );
    assert_ne!(
        unsafe { CreateDirectoryW(wide(&fixture.0).unwrap().as_ptr(), &security.attrs()) },
        0,
        "protected synthetic directory creation"
    );
    let directory = open_file(
        &fixture.0,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    )
    .expect("new root metadata handle");
    check_object(&directory, true).expect("new root object shape");
    security
        .check(&directory, true)
        .expect("new root exact owner and protected DACL");
    dirs.push(directory);
    cloud_check(dirs.last().unwrap(), &dirs)
        .expect("new root cloud and registered-sync-root observation");
    let lock = open_file(
        &fixture.0.join("lock"),
        GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
        0,
        OPEN_ALWAYS,
        Some(&security),
    )
    .expect("exclusive lifetime lock creation");
    assert_eq!(
        check_object(&lock, false).expect("lock object shape").size,
        0
    );
    security
        .check(&lock, true)
        .expect("lock exact owner and protected DACL");
    lock.sync_all().expect("lock flush");
    drop(lock);
    drop(dirs);
    let disk = Disk::at(&fixture.0, false).expect("reopen full native storage contract");
    drop(disk);
}

#[test]
fn native_torn_control_preserves_encrypted_evidence_and_current_generation() {
    use crate::{Capture, ProfileText, Recovery};
    use codex_accounts_vault::{Identity, Resource, ResourceId, ResourceShape};
    let fixture = super::super::tests::Sandbox::new();
    let mut disk = Disk::at(&fixture.0, true).unwrap();
    let root = bootstrap(&mut disk, true).unwrap();
    let mut store = Storage::create(disk, &root).unwrap();
    let rid = ResourceId::new(0).unwrap();
    let bytes = b"{ \"SYNTHETIC\": \"NOT_REAL\" }\r\n";
    let capture = Capture::new(
        Identity::new(
            "SYNTHETIC_ISSUER".into(),
            "SYNTHETIC_SUBJECT".into(),
            "SYNTHETIC_WORKSPACE".into(),
        )
        .unwrap(),
        1,
        vec![(rid, ResourceShape::JsonObject, true)],
        vec![Resource::present(rid, bytes.to_vec()).unwrap()],
    )
    .unwrap();
    let text = || ProfileText::new("SYNTHETIC".into()).unwrap();
    let (profile, generation) = store.add(&root, text(), text(), capture).unwrap();
    drop(store);
    let mut disk = Disk::at(&fixture.0, false).unwrap();
    disk.stage("state.bin", b"SYNTHETIC_TORN_STAGE").unwrap();
    drop(disk);
    let disk = Disk::at(&fixture.0, false).unwrap();
    let mut store = Storage::open(disk, &root).unwrap();
    assert_eq!(store.recovery(), Recovery::ControlRepairRequired);
    store.recover_control(&root).unwrap();
    assert_eq!(store.latest(profile).unwrap(), generation);
    assert!(
        store
            .read_latest(&root, profile)
            .unwrap()
            .resource(rid)
            .unwrap()
            .as_bytes()
            == Some(bytes.as_slice())
    );
    assert!(store
        .disk
        .list()
        .unwrap()
        .iter()
        .any(|name| name.starts_with("recovery-")));
    assert!(store.disk.read("state.bin.stage").unwrap().is_none());
    drop(store);
    let disk = Disk::at(&fixture.0, false).unwrap();
    assert_eq!(
        Storage::open(disk, &root).unwrap().recovery(),
        Recovery::Clean
    );
}

#[path = "hazard_tests.rs"]
mod hazards;
