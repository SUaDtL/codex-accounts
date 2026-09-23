use super::*;
use codex_accounts_vault::Identity;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

// Only tests can inject a storage implementation. This records persisted bytes
// independently of Storage, so dropping/reopening loses all coordinator memory.
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<Medium>>);
#[derive(Clone, Default)]
struct Medium {
    files: BTreeMap<String, Vec<u8>>,
    step: usize,
    fail: Option<usize>,
    code: Option<StorageError>,
    trace: Vec<&'static str>,
}
impl Memory {
    fn checkpoint(&self, stage: &'static str) -> Result<(), StorageError> {
        let mut m = self.0.borrow_mut();
        m.step += 1;
        m.trace.push(stage);
        if m.fail == Some(m.step) {
            Err(m.code.unwrap_or(StorageError::Io))
        } else {
            Ok(())
        }
    }
    fn snapshot(&self) -> Self {
        let mut m = self.0.borrow().clone();
        m.step = 0;
        m.fail = None;
        m.trace.clear();
        Self(Rc::new(RefCell::new(m)))
    }
    fn fail(&self, n: usize, code: StorageError) {
        let mut m = self.0.borrow_mut();
        m.step = 0;
        m.fail = Some(n);
        m.code = Some(code);
        m.trace.clear();
    }
    fn heal(&self) {
        self.0.borrow_mut().fail = None;
    }
}
impl Files for Memory {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.0.borrow().files.get(name).cloned())
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.0.borrow().files.keys().cloned().collect())
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let name = format!("{name}.stage");
        if let Some(old) = self.read(&name)? {
            return if old == bytes {
                Ok(())
            } else {
                Err(StorageError::ExternalChange)
            };
        }
        self.checkpoint("before-create")?;
        self.0.borrow_mut().files.insert(name.clone(), vec![]);
        self.checkpoint("after-create")?;
        self.0
            .borrow_mut()
            .files
            .insert(name.clone(), bytes[..bytes.len() / 2].to_vec());
        self.checkpoint("partial-write")?;
        self.0.borrow_mut().files.insert(name, bytes.to_vec());
        self.checkpoint("after-write")?;
        self.checkpoint("before-flush")?;
        self.checkpoint("after-flush")
    }
    fn publish(
        &mut self,
        name: &str,
        expected: Option<&[u8]>,
        staged: &[u8],
    ) -> Result<(), StorageError> {
        if self.read(name)?.as_deref() != expected
            || self.read(&format!("{name}.stage"))?.as_deref() != Some(staged)
        {
            return Err(StorageError::ExternalChange);
        }
        self.checkpoint("before-publish")?;
        let mut m = self.0.borrow_mut();
        m.files.remove(&format!("{name}.stage"));
        m.files.insert(name.to_owned(), staged.to_vec());
        drop(m);
        self.checkpoint("after-publish")?;
        self.checkpoint("after-metadata-flush")
    }
    fn erase(&mut self, name: &str, expected: &[u8]) -> Result<(), StorageError> {
        if self.read(name)?.as_deref() != Some(expected) {
            return Err(StorageError::ExternalChange);
        }
        self.checkpoint("before-delete")?;
        self.0.borrow_mut().files.remove(name);
        self.checkpoint("after-delete")
    }
}
fn capture(subject: &str, value: &[u8], presence: bool) -> Capture {
    let a = ResourceId::new(0).unwrap();
    let b = ResourceId::new(1).unwrap();
    Capture::new(
        Identity::new(
            "SYNTHETIC_ISSUER".into(),
            subject.into(),
            "SYNTHETIC_WORKSPACE".into(),
        )
        .unwrap(),
        1,
        vec![
            (a, ResourceShape::JsonObject, true),
            (b, ResourceShape::Opaque, false),
        ],
        vec![
            Resource::present(a, value.to_vec()).unwrap(),
            if presence {
                Resource::present(b, b"SYNTHETIC\r\n\0\xff".to_vec()).unwrap()
            } else {
                Resource::absent(b)
            },
        ],
    )
    .unwrap()
}
fn text() -> ProfileText {
    ProfileText::new("SYNTHETIC_LABEL_Ω".into()).unwrap()
}
const A: &[u8] = b"{ \"SYNTHETIC_TOKEN\": \"NOT_REAL_A\", \"unknown\": [1,true] }\r\n";
const B: &[u8] = b"{\"SYNTHETIC_TOKEN\":\"NOT_REAL_B\"}";
fn initial() -> (RootKey, Memory, Storage<Memory>) {
    let root = RootKey::generate().unwrap();
    let disk = Memory::default();
    let store = Storage::create(disk.clone(), &root).unwrap();
    (root, disk, store)
}
fn bytes(store: &Storage<Memory>, root: &RootKey, p: ProfileId) -> Vec<u8> {
    store
        .read_latest(root, p)
        .unwrap()
        .resource(ResourceId::new(0).unwrap())
        .unwrap()
        .as_bytes()
        .unwrap()
        .to_vec()
}

#[test]
fn create_reopen_preserves_exact_bytes_absence_and_opaque_companions() {
    for present in [false, true] {
        let (root, disk, mut s) = initial();
        let (p, g) = s
            .add(&root, text(), text(), capture("SYNTHETIC_A", A, present))
            .unwrap();
        assert!(g.to_bytes() != [0; 16]);
        assert_eq!(p.to_bytes()[6] >> 4, 4);
        assert_eq!(p.to_bytes()[8] >> 6, 2);
        drop(s);
        let s = Storage::open(disk, &root).unwrap();
        assert_eq!(bytes(&s, &root, p), A);
        let set = s.read_latest(&root, p).unwrap();
        assert_eq!(
            set.resource(ResourceId::new(1).unwrap())
                .unwrap()
                .as_bytes()
                .is_some(),
            present
        );
    }
}
#[test]
fn newest_generation_and_stale_parent_do_not_replay_initial_snapshot() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    let b = s
        .append(&root, p, a, capture("SYNTHETIC_A", B, true))
        .unwrap();
    let before = disk.0.borrow().files.clone();
    assert!(matches!(
        s.append(&root, p, a, capture("SYNTHETIC_A", A, false)),
        Err(StorageError::StaleParent)
    ));
    assert!(matches!(
        s.append(&root, p, b, capture("OTHER_SUBJECT", A, false)),
        Err(StorageError::IdentityMismatch)
    ));
    assert_eq!(disk.0.borrow().files, before);
    drop(s);
    let s = Storage::open(disk, &root).unwrap();
    assert_eq!(s.latest(p).unwrap(), b);
    assert_eq!(bytes(&s, &root, p), B);
}
#[test]
fn duplicate_identity_not_label_and_fifty_profile_limit() {
    let (root, disk, mut s) = initial();
    s.add(&root, text(), text(), capture("SYNTHETIC_0", A, false))
        .unwrap();
    assert!(matches!(
        s.add(&root, text(), text(), capture("SYNTHETIC_0", B, true)),
        Err(StorageError::IdentityMismatch)
    ));
    for i in 1..50 {
        s.add(
            &root,
            text(),
            text(),
            capture(&format!("SYNTHETIC_{i}"), A, false),
        )
        .unwrap();
    }
    let old = disk.0.borrow().files.clone();
    assert!(matches!(
        s.add(&root, text(), text(), capture("SYNTHETIC_51", A, false)),
        Err(StorageError::InputLimit)
    ));
    assert_eq!(old, disk.0.borrow().files);
}
#[test]
fn labels_and_input_shapes_are_bounded_and_redacted() {
    for value in ["".to_owned(), "x".repeat(81), "line\nfeed".to_owned()] {
        assert!(ProfileText::new(value).is_err());
    }
    assert!(ProfileText::new("🧪".repeat(80)).is_ok());
    assert_eq!(format!("{:?}", text()), "ProfileText([REDACTED])");
    assert_eq!(
        format!("{:?}", capture("SECRET_SUBJECT_CANARY", A, false)),
        "Capture([REDACTED])"
    );
    assert_eq!(StorageError::Corrupt.to_string(), "Corrupt");
}
#[test]
fn ciphertext_and_generated_names_never_contain_identity_label_or_resource_canaries() {
    let (root, disk, mut s) = initial();
    s.add(
        &root,
        text(),
        text(),
        capture("SYNTHETIC_SECRET_SUBJECT_CANARY", A, true),
    )
    .unwrap();
    for (n, b) in &disk.0.borrow().files {
        for c in [b"SYNTHETIC".as_slice(), b"NOT_REAL_A", b"unknown"] {
            assert!(!n.as_bytes().windows(c.len()).any(|w| w == c));
            assert!(!b.windows(c.len()).any(|w| w == c));
        }
    }
}
#[test]
fn corruption_wrong_key_unrecognized_files_and_swapped_context_refuse() {
    let (root, disk, mut s) = initial();
    s.add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    drop(s);
    assert!(Storage::open(disk.clone(), &RootKey::generate().unwrap()).is_err());
    let bad = disk.snapshot();
    bad.0.borrow_mut().files.get_mut("state.bin").unwrap()[0] ^= 1;
    assert!(Storage::open(bad, &root).is_err());
    let bad = disk.snapshot();
    bad.0
        .borrow_mut()
        .files
        .insert("unexpected.bin".into(), vec![1]);
    assert!(matches!(
        Storage::open(bad, &root),
        Err(StorageError::ExternalChange)
    ));
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    let (p, _) = s
        .add(&root, text(), text(), capture("SYNTHETIC_B", B, false))
        .unwrap();
    let names: Vec<_> = s
        .registry
        .generations
        .iter()
        .map(|g| g.rules[0].blob.as_ref().unwrap().key.name())
        .collect();
    let a = disk.read(&names[0]).unwrap().unwrap();
    disk.0.borrow_mut().files.insert(names[1].clone(), a);
    assert!(s.read_latest(&root, p).is_err());
    assert!(Storage::open(disk, &root).is_err());
}
#[test]
fn external_control_replacement_and_staged_replay_block_without_writes() {
    let (root, disk, mut s) = initial();
    let old = disk.read("state.bin").unwrap().unwrap();
    s.add(&root, text(), text(), capture("SYNTHETIC", A, false))
        .unwrap();
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), old);
    let before = disk.0.borrow().files.clone();
    assert!(s.prune(&root).is_err());
    assert!(Storage::open(disk.clone(), &root).is_err());
    assert_eq!(disk.0.borrow().files, before);
}
#[test]
fn metadata_decoders_reject_every_truncation_trailing_and_unknown_schema() {
    let (root, _, mut s) = initial();
    s.add(&root, text(), text(), capture("SYNTHETIC", A, false))
        .unwrap();
    let reg = codec::registry(&s.registry).unwrap();
    let state = codec::state(&s.state).unwrap();
    for i in 0..reg.len() {
        assert!(codec::read_registry(&reg[..i]).is_err());
    }
    for i in 0..state.len() {
        assert!(codec::read_state(&state[..i]).is_err());
    }
    for mut b in [reg.to_vec(), state.to_vec()] {
        let registry = b.starts_with(b"CAREG001");
        b.push(0);
        assert!(if registry {
            codec::read_registry(&b).is_err()
        } else {
            codec::read_state(&b).is_err()
        });
        b[7] = b'9';
        assert!(if registry {
            codec::read_registry(&b).is_err()
        } else {
            codec::read_state(&b).is_err()
        });
    }
    assert!(codec::read_registry(&vec![0; MAX_METADATA + 1]).is_err());
}
#[test]
fn retention_waits_for_startup_and_preserves_complete_durable_holds() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    let b = s
        .append(&root, p, a, capture("SYNTHETIC_A", B, false))
        .unwrap();
    assert!(matches!(
        s.prune(&root),
        Err(StorageError::LaterStartupRequired)
    ));
    let mut r = s.copy_registry().unwrap();
    r.holds.push(Hold {
        id: [0x51; 16],
        generations: vec![a.to_bytes()],
    });
    s.commit(&root, r, vec![], vec![]).unwrap();
    drop(s);
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    s.prune(&root).unwrap();
    assert_eq!(s.registry.generations.len(), 2);
    assert_eq!(s.latest(p).unwrap(), b);
    let mut r = s.copy_registry().unwrap();
    r.holds.clear();
    s.commit(&root, r, vec![], vec![]).unwrap();
    drop(s);
    let mut s = Storage::open(disk.clone(), &root).unwrap();
    s.prune(&root).unwrap();
    assert_eq!(s.registry.generations.len(), 1);
    assert_eq!(bytes(&s, &root, p), B);
    assert!(s.state.garbage.is_empty());
    drop(s);
    Storage::open(disk, &root).unwrap();
}
#[test]
fn unknown_active_active_and_unresolved_delete_refuse_but_inactive_removal_is_transactional() {
    let (root, disk, mut s) = initial();
    let (p, g) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    assert_eq!(s.remove(&root, p), Err(StorageError::ActiveStateUnknown));
    let mut r = s.copy_registry().unwrap();
    r.active_known = true;
    r.active = Some(p.to_bytes());
    s.commit(&root, r, vec![], vec![]).unwrap();
    assert_eq!(s.remove(&root, p), Err(StorageError::ActiveProfile));
    let mut r = s.copy_registry().unwrap();
    r.active = None;
    r.holds.push(Hold {
        id: [0x51; 16],
        generations: vec![g.to_bytes()],
    });
    s.commit(&root, r, vec![], vec![]).unwrap();
    assert_eq!(s.remove(&root, p), Err(StorageError::Referenced));
    let mut r = s.copy_registry().unwrap();
    r.holds.clear();
    s.commit(&root, r, vec![], vec![]).unwrap();
    s.remove(&root, p).unwrap();
    assert_eq!(s.registry.profiles.len(), 0);
    assert!(!s.state.garbage.is_empty());
    drop(s);
    let mut s = Storage::open(disk, &root).unwrap();
    s.prune(&root).unwrap();
    assert!(s.state.garbage.is_empty());
}
#[test]
fn forged_or_incomplete_hold_and_generation_graphs_fail_validation() {
    let (root, _, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    let b = s
        .append(&root, p, a, capture("SYNTHETIC_A", B, false))
        .unwrap();
    let mut r = s.copy_registry().unwrap();
    r.holds.push(Hold {
        id: [0x51; 16],
        generations: vec![[0x52; 16]],
    });
    assert!(r.validate().is_err());
    let mut r = s.copy_registry().unwrap();
    r.generations[0].parent = Some(b.to_bytes());
    assert!(r.validate().is_err());
    let mut r = s.copy_registry().unwrap();
    r.generations[0].manifest.size = MAX_FILE as u32 + 1;
    assert!(r.validate().is_err());
}

// Exercise every storage effect before/after create, partial write, flush,
// publication and metadata flush. Reopen uses only persisted bytes and the root.
#[test]
fn every_append_crash_preserves_prior_generation_or_valid_committed_generation() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    drop(s);
    let good = disk.snapshot();
    let mut s = Storage::open(good.clone(), &root).unwrap();
    s.append(&root, p, a, capture("SYNTHETIC_A", B, true))
        .unwrap();
    let steps = good.0.borrow().step;
    assert!(steps > 30);
    for code in [
        StorageError::Io,
        StorageError::AccessDenied,
        StorageError::Busy,
    ] {
        for fail in 1..=steps {
            let d = disk.snapshot();
            let mut s = Storage::open(d.clone(), &root).unwrap();
            let previous = s.state.current.clone().unwrap();
            d.fail(fail, code);
            assert!(s
                .append(&root, p, a, capture("SYNTHETIC_A", B, true))
                .is_err());
            drop(s);
            d.heal();
            // The old registry/generation is physically intact even when a partial
            // control write deliberately requires manual recovery instead of guessing.
            let old = Storage::<Memory>::load_registry(&d, &root, &previous).unwrap();
            assert_eq!(old.profiles[0].latest, a.to_bytes());
            match Storage::open(d.clone(), &root) {
                Ok(mut reopened) => match reopened.recovery() {
                    Recovery::Clean => {
                        assert!([A, B].contains(&bytes(&reopened, &root, p).as_slice()))
                    }
                    Recovery::CommitPending => {
                        if reopened.reconcile(&root).is_ok() {
                            assert_eq!(bytes(&reopened, &root, p), B);
                        } else {
                            drop(reopened);
                            let mut r = Storage::open(d.clone(), &root).unwrap();
                            if r.restore_previous(&root).is_ok() {
                                assert_eq!(bytes(&r, &root, p), A);
                            } else {
                                assert!(r.state.current.is_some());
                            }
                        }
                    }
                    other => panic!("unexpected recovery {other:?}"),
                },
                Err(e) => assert!(matches!(
                    e,
                    StorageError::RecoveryRequired
                        | StorageError::ExternalChange
                        | StorageError::Corrupt
                )),
            }
        }
    }
}
#[test]
fn bootstrap_failures_never_claim_clean_without_reachable_registry() {
    let root = RootKey::generate().unwrap();
    let d = Memory::default();
    Storage::create(d.clone(), &root).unwrap();
    let steps = d.0.borrow().step;
    for fail in 1..=steps {
        let d = Memory::default();
        d.fail(fail, StorageError::Io);
        assert!(Storage::create(d.clone(), &root).is_err());
        d.heal();
        if let Ok(mut s) = Storage::open(d.clone(), &root) {
            s.reconcile(&root).unwrap();
            assert_eq!(s.recovery(), Recovery::Clean);
            assert!(s.registry.profiles.is_empty());
            assert!(s.state.current.is_some());
        }
    }
}
#[test]
fn recovery_writes_are_crash_injected_and_reopen_idempotently() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    drop(s);
    // Journal is published at step 9; stop before the first resource creation.
    let pending = disk.snapshot();
    let mut s = Storage::open(pending.clone(), &root).unwrap();
    pending.fail(10, StorageError::Io);
    assert!(s
        .append(&root, p, a, capture("SYNTHETIC_A", B, true))
        .is_err());
    pending.heal();
    drop(s);
    let probe = pending.snapshot();
    let mut s = Storage::open(probe.clone(), &root).unwrap();
    s.restore_previous(&root).unwrap();
    let n = probe.0.borrow().step;
    assert!(n > 0);
    for fail in 1..=n {
        let d = pending.snapshot();
        let mut s = Storage::open(d.clone(), &root).unwrap();
        d.fail(fail, StorageError::Io);
        assert!(s.restore_previous(&root).is_err());
        drop(s);
        d.heal();
        if let Ok(mut s) = Storage::open(d, &root) {
            if s.recovery() == Recovery::CommitPending {
                s.restore_previous(&root).unwrap();
            }
            assert_eq!(bytes(&s, &root, p), A);
        }
    }
}
#[test]
fn cleanup_crash_preserves_latest_and_resumes_without_pruning_held_bytes() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    s.append(&root, p, a, capture("SYNTHETIC_A", B, false))
        .unwrap();
    drop(s);
    let probe = disk.snapshot();
    let mut s = Storage::open(probe.clone(), &root).unwrap();
    s.prune(&root).unwrap();
    let n = probe.0.borrow().step;
    for fail in 1..=n {
        let d = disk.snapshot();
        let mut s = Storage::open(d.clone(), &root).unwrap();
        d.fail(fail, StorageError::Io);
        assert!(s.prune(&root).is_err());
        drop(s);
        d.heal();
        if let Ok(mut s) = Storage::open(d, &root) {
            s.reconcile(&root).unwrap();
            assert_eq!(bytes(&s, &root, p), B);
            s.verify_registry(&root, &s.registry).unwrap();
        }
    }
}

#[test]
fn staged_control_cannot_publish_a_missing_current_registry() {
    let (root, disk, s) = initial();
    let before = disk.read("state.bin").unwrap();
    let mut state = s.state.clone();
    state.sequence += 1;
    state.parent = before.as_deref().map(digest);
    state.current.as_mut().unwrap().key.generation = [0x61; 16];
    let sealed = root
        .seal(
            &control_context(&root).unwrap(),
            &codec::state(&state).unwrap(),
        )
        .unwrap();
    disk.0
        .borrow_mut()
        .files
        .insert("state.bin.stage".into(), sealed.as_bytes().to_vec());
    assert!(Storage::open(disk.clone(), &root).is_err());
    assert_eq!(disk.read("state.bin").unwrap(), before);
}
#[test]
fn reconstructable_metadata_recovery_is_fault_injected_at_every_effect() {
    let (root, disk, mut s) = initial();
    let (p, a) = s
        .add(&root, text(), text(), capture("SYNTHETIC_A", A, false))
        .unwrap();
    s.append(&root, p, a, capture("SYNTHETIC_A", B, false))
        .unwrap();
    drop(s);
    // Produce a prune transaction with a partially written registry stage.
    let pending = disk.snapshot();
    let mut s = Storage::open(pending.clone(), &root).unwrap();
    pending.fail(12, StorageError::Io);
    assert!(s.prune(&root).is_err());
    drop(s);
    pending.heal();
    let probe = pending.snapshot();
    let mut s = Storage::open(probe.clone(), &root).unwrap();
    s.reconcile(&root).unwrap();
    let count = probe.0.borrow().step;
    assert!(count >= 10);
    for at in 1..=count {
        let d = pending.snapshot();
        let mut s = Storage::open(d.clone(), &root).unwrap();
        d.fail(at, StorageError::Io);
        assert!(s.reconcile(&root).is_err());
        drop(s);
        d.heal();
        match Storage::open(d.clone(), &root) {
            Ok(mut s) => {
                s.reconcile(&root).unwrap();
                assert_eq!(bytes(&s, &root, p), B)
            }
            Err(e) => {
                assert!(matches!(
                    e,
                    StorageError::RecoveryRequired | StorageError::ExternalChange
                ));
                let old = Storage::<Memory>::load_registry(
                    &d,
                    &root,
                    &open_state(&root, &disk.read("state.bin").unwrap().unwrap())
                        .unwrap()
                        .current
                        .unwrap(),
                )
                .unwrap();
                assert_ne!(old.profiles[0].latest, a.to_bytes());
            }
        }
    }
}
