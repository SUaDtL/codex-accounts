//! Faults in recovery itself must preserve the original or a verified encrypted copy.
use super::*;

#[test]
fn torn_control_requires_explicit_action_and_never_selects_unknown_bytes() {
    let (root, disk, mut s) = initial();
    let (p, g) = s
        .add(&root, text(), text(), capture("SYNTHETIC", A, false))
        .unwrap();
    let original = disk.read("state.bin").unwrap().unwrap();
    drop(s);
    let damaged = b"SYNTHETIC_UNTRUSTED_CONTROL_BYTES";
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), damaged.to_vec());
    let before = disk.0.borrow().files.clone();
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    assert_eq!(s.recovery(), Recovery::ControlRepairRequired);
    assert_eq!(disk.0.borrow().files, before);
    assert_eq!(s.latest(p), Err(StorageError::RecoveryRequired));
    assert_eq!(s.reconcile(&root), Err(StorageError::RecoveryRequired));
    s.recover_control(&root).unwrap();
    assert_eq!(disk.read("state.bin").unwrap().unwrap(), original);
    assert_eq!(s.latest(p).unwrap(), g);
    assert_eq!(bytes(&s, &root, p), A);
    assert!(disk.read("state.bin.stage").unwrap().is_none());
    assert!(disk
        .list()
        .unwrap()
        .iter()
        .any(|n| n.starts_with("recovery-")));
    for b in disk.0.borrow().files.values() {
        assert!(!b.windows(damaged.len()).any(|v| v == damaged));
    }
    assert_eq!(s.prune(&root), Err(StorageError::LaterStartupRequired));
    drop(s);
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    let evidence: Vec<_> = disk
        .list()
        .unwrap()
        .into_iter()
        .filter(|n| n.starts_with("recovery-"))
        .collect();
    s.prune(&root).unwrap();
    for name in evidence {
        assert!(disk.read(&name).unwrap().is_some());
    }
}

#[test]
fn every_recovery_archive_write_failure_preserves_source_and_can_resume() {
    let (root, disk, mut s) = initial();
    let (p, _) = s
        .add(&root, text(), text(), capture("SYNTHETIC", A, false))
        .unwrap();
    drop(s);
    let damaged = b"SYNTHETIC_TORN_CONTROL";
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), damaged.to_vec());
    let d = disk.snapshot();
    let mut s = Storage::open(d.clone(), &root).unwrap();
    s.recover_control(&root).unwrap();
    let steps = d.0.borrow().step;
    assert!(steps >= 11);
    for failure in 1..=steps {
        let d = disk.snapshot();
        let mut s = Storage::open(d.clone(), &root).unwrap();
        d.fail(failure, StorageError::Io);
        assert!(s.recover_control(&root).is_err(), "archive edge {failure}");
        drop(s);
        d.heal();
        let mut s = Storage::open(d.clone(), &root).unwrap();
        if s.recovery() == Recovery::ControlRepairRequired {
            assert_eq!(d.read("state.bin.stage").unwrap().unwrap(), damaged);
            s.recover_control(&root).unwrap();
        }
        assert_eq!(s.recovery(), Recovery::Clean);
        assert_eq!(bytes(&s, &root, p), A);
    }
}

#[test]
fn control_repair_checks_snapshot_inventory_capacity_and_committed_integrity() {
    let (root, disk, mut s) = initial();
    s.add(&root, text(), text(), capture("SYNTHETIC", A, false))
        .unwrap();
    drop(s);
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), vec![]);
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), vec![1]);
    assert_eq!(s.recover_control(&root), Err(StorageError::ExternalChange));
    drop(s);
    disk.0
        .borrow_mut()
        .files
        .insert("foreign.bin".into(), vec![]);
    assert!(matches!(
        Storage::open(disk.clone(), &root),
        Err(StorageError::ExternalChange)
    ));
    disk.0.borrow_mut().files.remove("foreign.bin");
    for i in 0..32 {
        disk.0
            .borrow_mut()
            .files
            .insert(format!("recovery-{i:032x}.bin.stage"), vec![]);
    }
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    let before = disk.0.borrow().files.clone();
    assert_eq!(s.recover_control(&root), Err(StorageError::InputLimit));
    assert_eq!(disk.0.borrow().files, before);
    drop(s);
    disk.0.borrow_mut().files.get_mut("state.bin").unwrap()[0] ^= 1;
    assert!(matches!(
        Storage::open(disk, &root),
        Err(StorageError::Corrupt)
    ));
}

#[test]
fn interrupted_first_control_uses_original_root_and_never_hides_unknown_payloads() {
    let root = RootKey::generate().unwrap();
    let id = root.identifier();
    let d = Memory::default();
    d.0.borrow_mut()
        .files
        .insert("state.bin.stage".into(), b"SYNTHETIC_TORN_INITIAL".to_vec());
    let mut s = Storage::open(d.clone(), &root).unwrap();
    assert_eq!(s.recovery(), Recovery::ControlRepairRequired);
    s.recover_control(&root).unwrap();
    assert_eq!(root.identifier(), id);
    assert_eq!(s.recovery(), Recovery::Clean);
    assert!(s.registry.profiles.is_empty());
    drop(s);
    let s = Storage::open(d.clone(), &root).unwrap();
    assert_eq!(s.recovery(), Recovery::Clean);
    d.0.borrow_mut().files.remove("state.bin");
    assert!(matches!(
        Storage::open(d, &root),
        Err(StorageError::ExternalChange)
    ));
}

#[test]
fn authentic_control_stage_does_not_ignore_foreign_inventory_before_publication() {
    let (root, disk, s) = initial();
    let mut next = s.state.clone();
    next.sequence += 1;
    next.parent = s.state_bytes.as_deref().map(digest);
    let staged = root
        .seal(
            &control_context(&root).unwrap(),
            &codec::state(&next).unwrap(),
        )
        .unwrap();
    drop(s);
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), staged.as_bytes().to_vec());
    disk.0
        .borrow_mut()
        .files
        .insert("foreign.bin".into(), b"SYNTHETIC_EXTERNAL".to_vec());
    let before = disk.0.borrow().files.clone();
    assert!(matches!(
        Storage::open(disk.clone(), &root),
        Err(StorageError::ExternalChange)
    ));
    assert_eq!(disk.0.borrow().files, before);
}
