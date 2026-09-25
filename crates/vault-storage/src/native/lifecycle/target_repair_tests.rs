//! Synthetic native archive/removal tests. No real account or Desktop input.
use super::*;

pub(super) fn interrupted() -> (Fixture, Id) {
    let mut f = Fixture::new(1, 2);
    let id = f.begin();
    f.until(id, SwitchPhase::SourceSaved);
    f.fx.target.boundary = 0;
    f.fx.target.fail_at = Some(2);
    assert!(f.store.advance_switch(&f.root, &mut f.fx, id).is_err());
    f.fx.target.fail_at = None;
    f.fx.target.boundary = 0;
    (f, id)
}
fn stages(home: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(home)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "stage"))
        .collect()
}
pub(super) fn torn() {
    let (f, id) = interrupted();
    let mut f = f.reopen();
    assert_eq!(stages(&f.home).len(), 1);
    let source = plain(&f.fx.target);
    assert!(f
        .store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Restore)
        .is_err());
    assert_eq!(f.phase(), SwitchPhase::Conflict);
    f.store
        .repair_registered_staging(&f.root, &mut f.fx, id)
        .unwrap();
    assert!(stages(&f.home).is_empty());
    assert_eq!(plain(&f.fx.target), source);
    assert_eq!(f.store.recovery(), Recovery::SwitchPending);
    assert_eq!(f.phase(), SwitchPhase::Recovery);
    let mut f = f.reopen();
    f.store
        .recover_switch(&f.root, &mut f.fx, id, Choice::Restore)
        .unwrap();
    assert_eq!(f.phase(), SwitchPhase::Cancelled);
    assert_eq!(plain(&f.fx.target), values(A1, 1));
    assert_eq!(
        f.store.switch_status().unwrap()[0].primary_error,
        Some("E_WRITE_FAILED")
    );
}
pub(super) fn refusals() {
    let (mut f, id) = interrupted();
    let stage = stages(&f.home).pop().unwrap();
    let source = plain(&f.fx.target);
    let held = open_file(&stage, GENERIC_READ | GENERIC_WRITE, 0, OPEN_EXISTING, None).unwrap();
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.fx, id)
        .is_err());
    assert!(stage.exists());
    drop(held);
    let alias = f.home.join("synthetic-hardlink");
    std::fs::hard_link(&stage, &alias).unwrap();
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.fx, id)
        .is_err());
    assert!(stage.exists());
    std::fs::remove_file(alias).unwrap();
    let foreign = f.home.join("SYNTHETIC_UNREGISTERED.stage");
    std::fs::write(&foreign, b"SYNTHETIC_FOREIGN").unwrap();
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.fx, id)
        .is_err());
    assert_eq!(std::fs::read(&foreign).unwrap(), b"SYNTHETIC_FOREIGN");
    assert!(stage.exists());
    assert_eq!(plain(&f.fx.target), source);
}
pub(super) fn archive_failure() {
    let dir = home();
    let mut target = Target::open(&dir.0, 1).unwrap();
    let op = [9; 16];
    let bytes = b"SYNTHETIC_TORN_BYTES_DO_NOT_SELECT";
    target.stage(op, 0, Some(bytes)).unwrap();
    let mut called = 0;
    let result = target.preserve_then_remove(op, 1, &mut |resources| {
        called += 1;
        assert_eq!(resources[0].as_bytes(), Some(bytes.as_slice()));
        // The original handle remains pinned while evidence is being committed.
        let path = stages(&dir.0).pop().unwrap();
        assert!(open_file(&path, GENERIC_WRITE, 7, OPEN_EXISTING, None).is_err());
        Err(StorageError::Io.into())
    });
    assert_eq!(result, Err(SwitchError::Storage(StorageError::Io)));
    assert_eq!(called, 1);
    assert_eq!(std::fs::read(stages(&dir.0).pop().unwrap()).unwrap(), bytes);
    assert!(target.read(0).unwrap().is_none());
}
