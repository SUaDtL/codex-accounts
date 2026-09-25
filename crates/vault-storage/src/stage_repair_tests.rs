//! Portable encrypted-store fault proof; native handle proof lives separately.
use super::*;
const TORN: &[u8] = b"SYNTHETIC_TORN_NOT_A_CREDENTIAL\0\xff";
impl StagingRepair for Synthetic {
    fn preserve_then_remove_staging(
        &mut self,
        operation: Id,
        registered: u16,
        preserve: &mut dyn FnMut(Vec<Resource>) -> Result<(), SwitchError>,
    ) -> Result<(), SwitchError> {
        let captured = self.0.borrow().staged.clone();
        let mut resources = Vec::new();
        for ((id, slot), value) in &captured {
            if *id != operation || *slot >= 16 || registered & (1 << slot) == 0 {
                return Err(Failure::ExternalChange.into());
            }
            if let Some(bytes) = value {
                resources.push(
                    Resource::present(ResourceId::new(*slot).unwrap(), bytes.clone()).unwrap(),
                );
            }
        }
        if !resources.is_empty() {
            preserve(resources)?;
        }
        self.point("repair-after-archive")?;
        let mut w = self.0.borrow_mut();
        if w.staged != captured {
            return Err(Failure::ExternalChange.into());
        }
        w.staged.clear();
        drop(w);
        self.point("repair-after-delete")?;
        Ok(())
    }
}
fn interrupted() -> (Setup, Id) {
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::SourceSaved);
    {
        let mut w = f.effects.0.borrow_mut();
        w.events.clear();
        w.fail = Some(2);
    }
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    {
        let mut w = f.effects.0.borrow_mut();
        w.fail = None;
        w.staged.insert((id, 0), Some(TORN.to_vec()));
    }
    (f, id)
}
fn archived(store: &Storage<Memory>, root: &RootKey, id: Id) -> bool {
    store
        .journal(id)
        .unwrap()
        .evidence
        .iter()
        .filter(|e| e.kind == EvidenceKind::Staging)
        .flat_map(|e| &e.resources)
        .filter_map(|r| r.blob.as_ref())
        .any(|blob| {
            let encrypted = Storage::<Memory>::read_blob(&store.disk, blob).unwrap();
            root.open(
                &blob.key.context().unwrap(),
                &Envelope::from_bytes(&encrypted).unwrap(),
            )
            .unwrap()
            .expose()
                == TORN
        })
}
#[test]
fn staging_repair_keeps_origin_exact_bytes_and_recovery_pending() {
    let (mut f, id) = interrupted();
    // An identical LIVE evidence blob must not be mistaken for staging provenance.
    let j = f.store.journal(id).unwrap();
    f.store
        .preserve_conflict(
            &f.root,
            j,
            Snapshot {
                identity: None,
                schema: 0,
                resources: vec![
                    Resource::present(ResourceId::new(0).unwrap(), TORN.to_vec()).unwrap(),
                ],
            },
        )
        .unwrap();
    f.store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .unwrap();
    assert!(archived(&f.store, &f.root, id));
    assert_eq!(f.store.journal(id).unwrap().evidence.len(), 2);
    assert_eq!(f.store.recovery(), Recovery::SwitchPending);
    assert!(f.effects.0.borrow().staged.is_empty());
    assert_eq!(f.effects.0.borrow().live, values(A1, 1));
    for (name, bytes) in &f.disk.0.borrow().files {
        assert!(!name.contains("SYNTHETIC"));
        assert!(!bytes.windows(TORN.len()).any(|w| w == TORN));
    }
    let mut store = load(f.disk.clone(), &f.root);
    assert!(archived(&store, &f.root, id));
    recover_to_terminal(&mut store, &f.root, &mut f.effects, id);
    assert_eq!(store.journal(id).unwrap().failure, Some(Failure::Write));
    assert!(archived(&store, &f.root, id));
    assert_eq!(store.registry.format, 3);
}
#[test]
fn staging_repair_v3_is_explicit_and_v2_reads_without_migration() {
    let (mut f, id) = interrupted();
    let old = codec::registry(&f.store.registry).unwrap();
    assert_eq!(&old[..8], b"CAREG002");
    assert_eq!(
        codec::registry(&codec::read_registry(&old).unwrap()).unwrap(),
        old
    );
    f.store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .unwrap();
    let raw = codec::registry(&f.store.registry).unwrap();
    assert_eq!(&raw[..8], b"CAREG003");
    assert_eq!(
        codec::registry(&codec::read_registry(&raw).unwrap()).unwrap(),
        raw
    );
    for end in 0..raw.len() {
        assert!(codec::read_registry(&raw[..end]).is_err());
    }
    let mut extra = raw.to_vec();
    extra.push(0);
    assert!(codec::read_registry(&extra).is_err());
    let mut old_magic = raw.to_vec();
    old_magic[..8].copy_from_slice(b"CAREG002");
    assert!(codec::read_registry(&old_magic).is_err());
    let evidence_id = f.store.journal(id).unwrap().evidence[0].id;
    let offset = raw.windows(16).position(|b| b == evidence_id).unwrap();
    assert_eq!(raw[offset - 1], EvidenceKind::Staging as u8);
    let mut wrong = raw.to_vec();
    wrong[offset - 1] = 9;
    assert!(codec::read_registry(&wrong).is_err());
    let mut downgraded = codec::read_registry(&raw).unwrap();
    downgraded.format = 2;
    assert!(codec::registry(&downgraded).is_err());
    let mut wrong_slot = codec::read_registry(&raw).unwrap();
    wrong_slot.journals[0].evidence[0].resources[0].slot = 1;
    assert!(wrong_slot.validate().is_err());
}
#[test]
fn staging_repair_revalidates_policy_live_state_registration_key_and_phase() {
    for check in [
        Check::Confirm,
        Check::Lock,
        Check::Recovery,
        Check::Quiesce,
        Check::Write,
    ] {
        let (mut f, id) = interrupted();
        let before = f.disk.0.borrow().files.clone();
        f.effects.0.borrow_mut().deny = Some((check, Failure::Policy));
        assert_eq!(
            f.store
                .repair_registered_staging(&f.root, &mut f.effects, id),
            Err(Failure::Policy.into())
        );
        assert_eq!(f.disk.0.borrow().files, before);
        assert_eq!(f.effects.0.borrow().staged[&(id, 0)].as_deref(), Some(TORN));
    }
    let (mut f, id) = interrupted();
    let before = f.disk.0.borrow().files.clone();
    let wrong = RootKey::generate().unwrap();
    assert!(f
        .store
        .repair_registered_staging(&wrong, &mut f.effects, id)
        .is_err());
    assert_eq!(f.disk.0.borrow().files, before);
    f.effects.0.borrow_mut().live[0] = Some(b"SYNTHETIC_FOREIGN_LIVE".to_vec());
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .is_err());
    assert_eq!(f.disk.0.borrow().files, before);
    f.effects.0.borrow_mut().live = values(A1, 1);
    f.effects
        .0
        .borrow_mut()
        .staged
        .insert((id, 8), Some(TORN.to_vec()));
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .is_err());
    assert!(f.effects.0.borrow().staged.contains_key(&(id, 0)));
    let mut fresh = Setup::new(1, 2);
    let fresh_id = fresh.begin(false);
    assert_eq!(
        fresh
            .store
            .repair_registered_staging(&fresh.root, &mut fresh.effects, fresh_id),
        Err(SwitchError::InvalidTransition)
    );
}
#[test]
fn staging_repair_every_encrypted_commit_fault_preserves_or_archives_before_delete() {
    let (f, id) = interrupted();
    let frozen = f.disk.snapshot();
    let good = frozen.snapshot();
    let mut store = load(good.clone(), &f.root);
    let mut fx = f.effects.fork();
    store
        .repair_registered_staging(&f.root, &mut fx, id)
        .unwrap();
    let count = good.0.borrow().step;
    assert!((1..=256).contains(&count));
    for point in 1..=count {
        let disk = frozen.snapshot();
        let mut store = load(disk.clone(), &f.root);
        let mut fx = f.effects.fork();
        disk.fail(point, StorageError::Io);
        assert!(store
            .repair_registered_staging(&f.root, &mut fx, id)
            .is_err());
        // No external deletion is permitted after a failed archive transaction.
        assert_eq!(fx.0.borrow().staged[&(id, 0)].as_deref(), Some(TORN));
        assert_eq!(fx.0.borrow().live, values(A1, 1));
        drop(store);
        disk.heal();
        let mut reopened = load(disk, &f.root);
        reopened
            .repair_registered_staging(&f.root, &mut fx, id)
            .unwrap();
        assert!(archived(&reopened, &f.root, id));
        assert!(fx.0.borrow().staged.is_empty());
        recover_to_terminal(&mut reopened, &f.root, &mut fx, id);
        assert_eq!(fx.0.borrow().live, values(A1, 1));
        assert_eq!(reopened.journal(id).unwrap().failure, Some(Failure::Write));
    }
}
#[test]
fn staging_repair_retry_deduplicates_and_capacity_never_drops_evidence() {
    let (mut f, id) = interrupted();
    f.effects.0.borrow_mut().events.clear();
    f.effects.0.borrow_mut().fail = Some(1);
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .is_err());
    assert!(archived(&f.store, &f.root, id));
    assert_eq!(f.store.journal(id).unwrap().evidence.len(), 1);
    f.effects.0.borrow_mut().fail = None;
    f.store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .unwrap();
    assert_eq!(f.store.journal(id).unwrap().evidence.len(), 1);
    let (mut f, id) = interrupted();
    for n in 0..MAX_EVIDENCE {
        let j = f.store.journal(id).unwrap();
        f.store
            .preserve_conflict(
                &f.root,
                j,
                Snapshot {
                    identity: None,
                    schema: 0,
                    resources: vec![
                        Resource::present(ResourceId::new(0).unwrap(), vec![n as u8]).unwrap(),
                    ],
                },
            )
            .unwrap();
    }
    assert_eq!(
        f.store
            .repair_registered_staging(&f.root, &mut f.effects, id),
        Err(StorageError::InputLimit.into())
    );
    assert_eq!(f.store.journal(id).unwrap().evidence.len(), MAX_EVIDENCE);
    assert_eq!(f.effects.0.borrow().staged[&(id, 0)].as_deref(), Some(TORN));
}

#[test]
fn staging_repair_partial_batch_retry_reuses_the_authenticated_superset() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::TargetStaged);
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    // A pre-install cancel with clean modeled staging is terminal; create the
    // interrupted state through the real journal's save path, not a raw receipt.
    let mut j = f.store.journal(id).unwrap();
    j.phase = SwitchPhase::Recovery;
    j.cleaned = false;
    f.store.save_journal(&f.root, j).unwrap();
    {
        let mut w = f.effects.0.borrow_mut();
        w.staged.insert((id, 0), Some(TORN.to_vec()));
        w.staged
            .insert((id, 1), Some(b"SYNTHETIC_OTHER_TORN".to_vec()));
        w.events.clear();
        w.fail = Some(1);
    }
    assert!(f
        .store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .is_err());
    let first = f.store.journal(id).unwrap().evidence[0].id;
    {
        let mut w = f.effects.0.borrow_mut();
        w.fail = None;
        // Simulate one completed native handle deletion before process loss.
        w.staged.remove(&(id, 0));
    }
    let mut store = load(f.disk.clone(), &f.root);
    store
        .repair_registered_staging(&f.root, &mut f.effects, id)
        .unwrap();
    let j = store.journal(id).unwrap();
    assert_eq!(j.evidence.len(), 1);
    assert_eq!(j.evidence[0].id, first);
    assert!(f.effects.0.borrow().staged.is_empty());
    assert_eq!(f.effects.0.borrow().live, values(A1, 1));
    assert_eq!(store.recovery(), Recovery::SwitchPending);
}
