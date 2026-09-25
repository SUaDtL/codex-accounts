//! Controlled synthetic effects only. The world and encrypted medium outlive a
//! Storage instance, so restart tests cannot rely on a coordinator's memory.
use super::super::tests::Memory;
use super::*;
use codex_accounts_core::{DesktopIdentity, RecoveryState};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const A0: &[u8] = b"{ \"SYNTHETIC_TOKEN\":\"NOT_REAL_A0\", \"unknown\":[1,true] }\r\n";
const A1: &[u8] = b"{\"SYNTHETIC_TOKEN\":\"NOT_REAL_A1\",\"extra\":null}\r\n";
const B0: &[u8] = b"{\"SYNTHETIC_TOKEN\":\"NOT_REAL_B0\"}";
const B1: &[u8] = b"{\"SYNTHETIC_TOKEN\":\"NOT_REAL_B1\"}\r\n";
const BINDING: [u8; 32] = [0x53; 32];
fn identity(name: &str) -> Identity {
    Identity::new(
        "SYNTHETIC_ISSUER".into(),
        name.into(),
        "SYNTHETIC_WORKSPACE".into(),
    )
    .unwrap()
}
fn values(main: &[u8], mask: u8) -> Vec<Option<Vec<u8>>> {
    vec![
        Some(main.to_vec()),
        if mask & 1 != 0 {
            Some(b"SYNTHETIC\r\n\0\xff".to_vec())
        } else {
            None
        },
        if mask & 2 != 0 { Some(vec![]) } else { None },
    ]
}
fn resources(v: &[Option<Vec<u8>>]) -> Vec<Resource> {
    v.iter()
        .enumerate()
        .map(|(n, b)| match b {
            Some(b) => Resource::present(ResourceId::new(n as u8).unwrap(), b.clone()).unwrap(),
            None => Resource::absent(ResourceId::new(n as u8).unwrap()),
        })
        .collect()
}
fn capture_for(name: &str, v: &[Option<Vec<u8>>]) -> Capture {
    Capture::new(
        identity(name),
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
fn text() -> ProfileText {
    ProfileText::new("SYNTHETIC_ONLY".into()).unwrap()
}
#[derive(Clone)]
struct World {
    live: Vec<Option<Vec<u8>>>,
    staged: BTreeMap<(Id, u8), Option<Vec<u8>>>,
    helper: Option<Id>,
    stuck: bool,
    refresh: bool,
    acceptance: Result<CredentialAcceptance, Failure>,
    launch: DesktopLaunch,
    launches: usize,
    deny: Option<(Check, Failure)>,
    events: Vec<&'static str>,
    fail: Option<usize>,
    unknown_identity: bool,
}
#[derive(Clone)]
pub(crate) struct Synthetic(Rc<RefCell<World>>);
impl Synthetic {
    fn new(v: Vec<Option<Vec<u8>>>) -> Self {
        Self(Rc::new(RefCell::new(World {
            live: v,
            staged: BTreeMap::new(),
            helper: None,
            stuck: false,
            refresh: false,
            acceptance: Ok(CredentialAcceptance::Accepted),
            launch: DesktopLaunch::Opened,
            launches: 0,
            deny: None,
            events: vec![],
            fail: None,
            unknown_identity: false,
        })))
    }
    fn fork(&self) -> Self {
        let mut w = self.0.borrow().clone();
        w.events.clear();
        w.fail = None;
        Self(Rc::new(RefCell::new(w)))
    }
    fn point(&self, event: &'static str) -> Result<(), Failure> {
        let mut w = self.0.borrow_mut();
        w.events.push(event);
        if w.fail == Some(w.events.len()) {
            Err(Failure::Write)
        } else {
            Ok(())
        }
    }
}
impl Effects for Synthetic {
    fn confirm(&mut self, _: &Request) -> Result<(), Failure> {
        self.guard(Check::Confirm, &BINDING)
    }
    fn guard(&mut self, c: Check, b: &[u8; 32]) -> Result<(), Failure> {
        let w = self.0.borrow();
        if *b != BINDING {
            return Err(Failure::Binding);
        }
        if let Some((check, error)) = w.deny {
            if check == c {
                return Err(error);
            }
        }
        if matches!(c, Check::Write | Check::Commit | Check::Launch) && w.helper.is_some() {
            return Err(Failure::Writers);
        }
        Ok(())
    }
    fn snapshot(&mut self) -> Result<Snapshot, Failure> {
        let w = self.0.borrow();
        let main = w.live.first().and_then(|v| v.as_deref());
        let who = if w.unknown_identity {
            None
        } else if main == Some(A0) || main == Some(A1) {
            Some(identity("SYNTHETIC_A"))
        } else if main == Some(B0) || main == Some(B1) {
            Some(identity("SYNTHETIC_B"))
        } else {
            None
        };
        Ok(Snapshot {
            identity: who,
            schema: 1,
            resources: resources(&w.live),
        })
    }
    fn stage(&mut self, op: Id, slot: u8, value: Option<&[u8]>) -> Result<(), Failure> {
        self.point("before-stage")?;
        let mut w = self.0.borrow_mut();
        let value = value.map(<[u8]>::to_vec);
        if w.staged.get(&(op, slot)).is_some_and(|v| *v != value) {
            return Err(Failure::ExternalChange);
        }
        w.staged.insert((op, slot), value);
        drop(w);
        self.point("after-stage")
    }
    fn replace(
        &mut self,
        op: Id,
        slot: u8,
        expected: Option<&[u8]>,
        value: Option<&[u8]>,
    ) -> Result<(), Failure> {
        self.point("before-replace")?;
        let mut w = self.0.borrow_mut();
        if w.helper.is_some() || !w.staged.contains_key(&(op, slot)) {
            return Err(Failure::Writers);
        }
        let current = w
            .live
            .get_mut(slot as usize)
            .ok_or(Failure::ExternalChange)?;
        if current.as_deref() != expected {
            return Err(Failure::ExternalChange);
        }
        *current = value.map(<[u8]>::to_vec);
        drop(w);
        self.point("after-replace")
    }
    fn cleanup(&mut self, op: Id, mask: u16, _: &[CredentialSet]) -> Result<(), Failure> {
        self.point("before-cleanup")?;
        let mut w = self.0.borrow_mut();
        if w.helper.is_some() {
            return Err(Failure::HelperStuck);
        }
        for (id, slot) in w.staged.keys() {
            if *id == op && mask & (1 << slot) == 0 {
                return Err(Failure::Cleanup);
            }
        }
        w.staged.retain(|(id, _), _| *id != op);
        drop(w);
        self.point("after-cleanup")
    }
    fn start_helper(&mut self, op: Id) -> Result<(), Failure> {
        self.point("before-helper")?;
        let mut w = self.0.borrow_mut();
        if w.helper.is_some() {
            return Err(Failure::HelperStuck);
        }
        w.helper = Some(op);
        drop(w);
        self.point("after-helper")
    }
    fn observe(&mut self, op: Id) -> Result<CredentialAcceptance, Failure> {
        self.point("before-observe")?;
        let w = self.0.borrow();
        if w.helper != Some(op) {
            return Err(Failure::HelperStuck);
        }
        let result = w.acceptance;
        drop(w);
        self.point("after-observe")?;
        result
    }
    fn reap_helper(&mut self, op: Id) -> Result<(), Failure> {
        self.point("before-reap")?;
        let mut w = self.0.borrow_mut();
        if w.stuck || w.helper.is_some_and(|v| v != op) {
            return Err(Failure::HelperStuck);
        }
        if w.helper.is_some() && w.refresh {
            w.live[0] = Some(B1.to_vec());
        }
        w.helper = None;
        drop(w);
        self.point("after-reap")
    }
    fn launch(&mut self, _: Id) -> Result<DesktopLaunch, Failure> {
        self.point("before-launch")?;
        let mut w = self.0.borrow_mut();
        w.launches += 1;
        let result = w.launch;
        drop(w);
        self.point("after-launch")?;
        Ok(result)
    }
}
struct Setup {
    root: RootKey,
    disk: Memory,
    store: Storage<Memory>,
    effects: Synthetic,
    a: ProfileId,
    b: ProfileId,
}
impl Setup {
    fn new(source: u8, target: u8) -> Self {
        let root = RootKey::generate().unwrap();
        let disk = Memory::default();
        let mut store = Storage::create(disk.clone(), &root).unwrap();
        let (a, _) = store
            .add(
                &root,
                text(),
                text(),
                capture_for("SYNTHETIC_A", &values(A0, source)),
            )
            .unwrap();
        let (b, _) = store
            .add(
                &root,
                text(),
                text(),
                capture_for("SYNTHETIC_B", &values(B0, target)),
            )
            .unwrap();
        Self {
            root,
            disk,
            store,
            effects: Synthetic::new(values(A1, source)),
            a,
            b,
        }
    }
    fn request(&self, online: bool) -> Request {
        request(&self.store, self.a, self.b, online)
    }
    fn begin(&mut self, online: bool) -> Id {
        let r = self.request(online);
        self.store
            .begin_switch(&self.root, &mut self.effects, r)
            .unwrap()
    }
    fn phase(&self, id: Id) -> SwitchPhase {
        self.store.journal(id).unwrap().phase
    }
    fn until(&mut self, id: Id, phase: SwitchPhase) {
        for _ in 0..100 {
            if self.phase(id) == phase {
                return;
            }
            self.store
                .advance_switch(&self.root, &mut self.effects, id)
                .unwrap();
        }
        panic!("bounded transition not reached")
    }
}
fn request<D: Files>(s: &Storage<D>, a: ProfileId, b: ProfileId, online: bool) -> Request {
    Request {
        source: GenerationRef {
            profile: a.to_bytes(),
            generation: s.latest(a).unwrap().to_bytes(),
        },
        target: GenerationRef {
            profile: b.to_bytes(),
            generation: s.latest(b).unwrap().to_bytes(),
        },
        binding: BINDING,
        online,
    }
}
fn run<D: Files>(s: &mut Storage<D>, root: &RootKey, e: &mut Synthetic, id: Id) {
    for _ in 0..100 {
        if s.journal(id).unwrap().terminal() {
            return;
        }
        s.advance_switch(root, e, id).unwrap();
    }
    panic!("coordinator did not terminate")
}
fn load(d: Memory, root: &RootKey) -> Storage<Memory> {
    let mut s = Storage::open(d.clone(), root).expect("reopen encrypted state");
    if s.recovery() == Recovery::ControlRepairRequired {
        s.recover_control(root).unwrap();
    }
    if matches!(
        s.recovery(),
        Recovery::CommitPending | Recovery::CleanupPending
    ) && s.reconcile(root).is_err()
    {
        s = Storage::open(d, root).unwrap();
        s.restore_previous(root)
            .expect("uncommitted storage restore");
    }
    assert!(matches!(
        s.recovery(),
        Recovery::Clean | Recovery::SwitchPending
    ));
    s
}
fn recover_to_terminal(s: &mut Storage<Memory>, root: &RootKey, e: &mut Synthetic, id: Id) {
    let j = s.journal(id).unwrap();
    if !j.terminal() {
        let choice = if j.committed {
            Choice::Finish
        } else {
            Choice::Restore
        };
        s.recover_switch(root, e, id, choice).unwrap();
    }
    run(s, root, e, id);
}
#[test]
fn refreshed_roundtrip_and_all_optional_presence_combinations() {
    for source in 0..4 {
        for target in 0..4 {
            let mut f = Setup::new(source, target);
            let id = f.begin(false);
            run(&mut f.store, &f.root, &mut f.effects, id);
            assert_eq!(f.effects.0.borrow().live, values(B0, target));
            let status = f.store.switch_status().unwrap();
            let st = &status[0];
            assert_eq!(st.installed_profile, Some(f.b));
            assert_eq!(
                st.observations.desktop_identity(),
                DesktopIdentity::AwaitingConfirmation
            );
            assert_eq!(
                st.observations.credential_acceptance(),
                CredentialAcceptance::Unknown
            );
            let req = request(&f.store, f.b, f.a, false);
            let back = f.store.begin_switch(&f.root, &mut f.effects, req).unwrap();
            run(&mut f.store, &f.root, &mut f.effects, back);
            assert_eq!(f.effects.0.borrow().live, values(A1, source));
            assert!(f.effects.0.borrow().staged.is_empty());
        }
    }
}
#[test]
fn helper_refresh_is_preserved_before_rejected_target_restoration() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    f.effects.0.borrow_mut().acceptance = Ok(CredentialAcceptance::Rejected);
    f.until(id, SwitchPhase::Observed);
    assert_eq!(
        f.store.advance_switch(&f.root, &mut f.effects, id),
        Err(SwitchError::Refused(Failure::LoginRequired))
    );
    let b1 = f.store.latest(f.b).unwrap();
    assert_ne!(
        b1.to_bytes(),
        f.store.journal(id).unwrap().original_target.generation
    );
    f.store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .unwrap();
    run(&mut f.store, &f.root, &mut f.effects, id);
    assert_eq!(f.effects.0.borrow().live, values(A1, 1));
    assert_eq!(
        f.store
            .read_latest(&f.root, f.b)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(B1)
    );
    assert_eq!(f.store.latest(f.b).unwrap(), b1);
    let req = f.request(false);
    assert_eq!(
        f.store.begin_switch(&f.root, &mut f.effects, req),
        Err(SwitchError::Refused(Failure::LoginRequired))
    );
}
#[test]
fn unavailable_online_observation_is_not_rejection_or_desktop_confirmation() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(true);
    f.effects.0.borrow_mut().acceptance = Ok(CredentialAcceptance::ObservationUnavailable);
    run(&mut f.store, &f.root, &mut f.effects, id);
    let s = f.store.journal(id).unwrap().status();
    assert_eq!(
        s.observations.credential_acceptance(),
        CredentialAcceptance::ObservationUnavailable
    );
    assert_eq!(
        s.observations.desktop_identity(),
        DesktopIdentity::AwaitingConfirmation
    );
}
#[test]
fn helper_policy_failure_is_not_laundered_into_an_offline_success() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    f.effects.0.borrow_mut().acceptance = Err(Failure::Policy);
    f.until(id, SwitchPhase::Recovery);
    let j = f.store.journal(id).unwrap();
    assert!(!j.committed);
    assert_eq!(j.failure, Some(Failure::Policy));
    assert!(f.effects.0.borrow().helper.is_none());
    assert_eq!(
        f.store
            .read_latest(&f.root, f.b)
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(B1)
    );
}
#[test]
fn preconditions_refuse_and_duplicate_requests_cannot_mutate() {
    for (check, code) in [
        (Check::Confirm, Failure::Consent),
        (Check::Lock, Failure::Busy),
        (Check::Quiesce, Failure::Writers),
    ] {
        let mut f = Setup::new(0, 0);
        f.effects.0.borrow_mut().deny = Some((check, code));
        let req = f.request(false);
        let begin = f.store.begin_switch(&f.root, &mut f.effects, req);
        if check == Check::Confirm {
            assert_eq!(begin, Err(SwitchError::Refused(code)));
        } else {
            let id = begin.unwrap();
            for _ in 0..3 {
                if f.store.advance_switch(&f.root, &mut f.effects, id).is_err() {
                    break;
                }
            }
            assert_eq!(f.store.journal(id).unwrap().failure, Some(code));
        }
        assert_eq!(f.effects.0.borrow().live, values(A1, 0));
        assert!(f.effects.0.borrow().staged.is_empty());
    }
    let mut f = Setup::new(0, 0);
    let req = f.request(false);
    let id = f.begin(false);
    assert_eq!(
        f.store.begin_switch(&f.root, &mut f.effects, req),
        Err(SwitchError::Storage(StorageError::SwitchPending))
    );
    assert_eq!(f.store.prune(&f.root), Err(StorageError::SwitchPending));
    assert_eq!(
        f.store.append(
            &f.root,
            f.a,
            GenerationId::from_bytes(req.source.generation).unwrap(),
            capture_for("SYNTHETIC_A", &values(A0, 0))
        ),
        Err(StorageError::SwitchPending)
    );
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    let mut stale = req;
    stale.target.generation = [0x55; 16];
    assert_eq!(
        f.store.begin_switch(&f.root, &mut f.effects, stale),
        Err(SwitchError::Storage(StorageError::StaleParent))
    );
}
#[test]
fn unknown_external_bytes_are_encrypted_and_never_overwritten() {
    let mut f = Setup::new(0, 0);
    let id = f.begin(false);
    f.until(id, SwitchPhase::Applying);
    let unknown = b"SYNTHETIC_EXTERNAL_SECRET_CANARY".to_vec();
    f.effects.0.borrow_mut().live[0] = Some(unknown.clone());
    assert_eq!(
        f.store.advance_switch(&f.root, &mut f.effects, id),
        Err(SwitchError::Refused(Failure::ExternalChange))
    );
    let j = f.store.journal(id).unwrap();
    assert_eq!(j.evidence.len(), 1);
    assert_eq!(
        j.status().observations.recovery_state(),
        RecoveryState::Conflict
    );
    assert!(f
        .store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .is_err());
    assert_eq!(f.effects.0.borrow().live[0], Some(unknown.clone()));
    for bytes in f.disk.0.borrow().files.values() {
        assert!(!bytes.windows(unknown.len()).any(|w| w == unknown));
    }
    assert!(!format!("{:?}", j.status()).contains("CANARY"));
    assert_eq!(f.store.journal(id).unwrap().evidence.len(), 1); // retries deduplicate evidence
}
#[test]
fn missing_identity_blocks_capture_before_any_external_write() {
    let mut f = Setup::new(0, 0);
    let id = f.begin(false);
    f.until(id, SwitchPhase::Quiescent);
    f.effects.0.borrow_mut().unknown_identity = true;
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    assert!(f.effects.0.borrow().staged.is_empty());
    assert_eq!(f.effects.0.borrow().live, values(A1, 0));
}
#[test]
fn helper_signal_or_observation_does_not_allow_writes_before_exit() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(true);
    f.until(id, SwitchPhase::HelperReaping);
    f.effects.0.borrow_mut().stuck = true;
    let live = f.effects.0.borrow().live.clone();
    assert_eq!(
        f.store.advance_switch(&f.root, &mut f.effects, id),
        Err(SwitchError::Refused(Failure::HelperStuck))
    );
    assert!(f
        .store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .is_err());
    assert_eq!(f.effects.0.borrow().live, live);
    f.effects.0.borrow_mut().stuck = false;
    f.effects.0.borrow_mut().refresh = true;
    f.store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .unwrap();
    run(&mut f.store, &f.root, &mut f.effects, id);
    assert_eq!(f.effects.0.borrow().live, values(A1, 0));
}
#[test]
fn cancellation_at_each_executable_stage_uses_reconciliation() {
    let stages = [
        SwitchPhase::Requested,
        SwitchPhase::Locked,
        SwitchPhase::Quiescent,
        SwitchPhase::SourceSaved,
        SwitchPhase::TargetStaged,
        SwitchPhase::Applying,
        SwitchPhase::TargetInstalled,
        SwitchPhase::HelperRunning,
        SwitchPhase::HelperReaping,
        SwitchPhase::Observed,
        SwitchPhase::Committed,
        SwitchPhase::RelaunchRequested,
        SwitchPhase::AwaitingConfirmation,
    ];
    for phase in stages {
        let mut f = Setup::new(1, 2);
        let id = f.begin(true);
        f.until(id, phase);
        let committed = f.store.journal(id).unwrap().committed;
        let result = f.store.cancel_switch(&f.root, &mut f.effects, id);
        if committed {
            assert_eq!(result, Err(SwitchError::AlreadyCommitted));
            assert_eq!(f.effects.0.borrow().live, values(B0, 2));
        } else {
            result.unwrap();
            recover_to_terminal(&mut f.store, &f.root, &mut f.effects, id);
            assert_eq!(f.effects.0.borrow().live, values(A1, 1));
        }
    }
}
#[test]
fn launch_failure_keeps_committed_target_and_no_stale_switch_back() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(false);
    f.effects.0.borrow_mut().launch = DesktopLaunch::Failed;
    run(&mut f.store, &f.root, &mut f.effects, id);
    let j = f.store.journal(id).unwrap();
    assert!(j.committed);
    assert_eq!(j.status().installed_profile, Some(f.b));
    assert_eq!(j.failure, Some(Failure::Launch));
    assert_eq!(
        j.status().observations.desktop_identity(),
        DesktopIdentity::Unknown
    );
    assert_eq!(f.effects.0.borrow().live, values(B0, 1));
    assert!(f.store.cancel_switch(&f.root, &mut f.effects, id).is_err());
}
#[test]
fn startup_never_replays_a_committed_launch_or_uses_stale_session_authority() {
    for phase in [
        SwitchPhase::Applying,
        SwitchPhase::HelperRunning,
        SwitchPhase::RelaunchRequested,
    ] {
        let mut f = Setup::new(0, 1);
        let id = f.begin(true);
        f.until(id, phase);
        if phase == SwitchPhase::RelaunchRequested {
            f.effects.launch(id).unwrap();
        }
        let mut restarted = load(f.disk.clone(), &f.root);
        assert_eq!(
            restarted.advance_switch(&f.root, &mut f.effects, id),
            Err(SwitchError::Storage(StorageError::SwitchPending))
        );
        recover_to_terminal(&mut restarted, &f.root, &mut f.effects, id);
        let j = restarted.journal(id).unwrap();
        if j.committed {
            assert_eq!(f.effects.0.borrow().launches, 1);
            assert_eq!(f.effects.0.borrow().live, values(B0, 1));
        } else {
            assert_eq!(f.effects.0.borrow().live, values(A1, 0));
        }
    }
}
#[test]
fn explicit_finish_requires_complete_target_and_cannot_bypass_cancellation() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(false);
    f.until(id, SwitchPhase::TargetInstalled);
    let mut s = load(f.disk.clone(), &f.root);
    s.recover_switch(&f.root, &mut f.effects, id, Choice::Finish)
        .unwrap();
    run(&mut s, &f.root, &mut f.effects, id);
    assert_eq!(f.effects.0.borrow().live, values(B0, 1));
    let mut f = Setup::new(0, 1);
    let id = f.begin(false);
    f.until(id, SwitchPhase::Applying);
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    assert_eq!(
        f.store
            .recover_switch(&f.root, &mut f.effects, id, Choice::Finish),
        Err(SwitchError::InvalidTransition)
    );
}
#[test]
fn primary_and_restoration_failures_survive_separate_restarts() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::Applying);
    f.effects.0.borrow_mut().deny = Some((Check::Write, Failure::Write));
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    f.effects.0.borrow_mut().deny = None;
    f.store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .unwrap();
    f.effects.0.borrow_mut().deny = Some((Check::Write, Failure::Writers));
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    let mut s = load(f.disk.clone(), &f.root);
    let j = s.journal(id).unwrap();
    assert_eq!(j.failure, Some(Failure::Write));
    assert_eq!(j.restoration_failure, Some(Failure::Writers));
    f.effects.0.borrow_mut().deny = None;
    recover_to_terminal(&mut s, &f.root, &mut f.effects, id);
    let j = s.journal(id).unwrap();
    assert_eq!(j.failure, Some(Failure::Write));
    assert_eq!(j.restoration_failure, Some(Failure::Writers));
    assert_eq!(f.effects.0.borrow().live, values(A1, 1));
}
#[test]
fn journal_holds_are_complete_and_pruned_only_after_terminal_later_startup() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    f.until(id, SwitchPhase::HelperReaping);
    let mut s = load(f.disk.clone(), &f.root);
    assert_eq!(s.prune(&f.root), Err(StorageError::SwitchPending));
    recover_to_terminal(&mut s, &f.root, &mut f.effects, id);
    assert_eq!(s.prune(&f.root), Err(StorageError::LaterStartupRequired));
    let j = s.journal(id).unwrap();
    let hold = s.registry.holds.iter().find(|h| h.id == id).unwrap();
    assert_eq!(hold.generations, j.references());
    let mut s = load(f.disk.clone(), &f.root);
    s.prune(&f.root).unwrap();
    assert!(s.registry.journals.is_empty());
    assert!(s.registry.holds.is_empty());
    assert_eq!(s.registry.generations.len(), 2);
    assert_eq!(s.remove(&f.root, f.a), Err(StorageError::ActiveProfile));
    s.remove(&f.root, f.b).unwrap();
}
#[test]
fn journal_graph_codec_and_authenticated_expected_marks_reject_drift() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(false);
    f.until(id, SwitchPhase::SourceSaved);
    for variant in 0..9 {
        let mut r = f.store.copy_registry().unwrap();
        let j = &mut r.journals[0];
        match variant {
            0 => j.write_done = 1,
            1 => j.committed = true,
            2 => j.helper = Helper::MayRun,
            3 => j.source.generation = [99; 16],
            4 => j.source_marks[0].present = false,
            5 => j.write_intent = 4,
            6 => j.staging_done = 8,
            7 => j.phase = SwitchPhase::AwaitingConfirmation,
            _ => r.holds.clear(),
        }
        assert!(codec::registry(&r).is_err());
    }
    let mut r = f.store.copy_registry().unwrap();
    r.journals[0].source_marks[0].tag[0] ^= 1;
    assert!(Storage::<Memory>::verify_registry_on(&f.disk, &f.root, &r).is_err());
    let raw = codec::registry(&f.store.registry).unwrap();
    assert_eq!(&raw[..8], b"CAREG002");
    for end in [0, 7, 8, raw.len() - 1] {
        assert!(codec::read_registry(&raw[..end]).is_err());
    }
    let mut trailing = raw.to_vec();
    trailing.push(0);
    assert!(codec::read_registry(&trailing).is_err());
    assert_eq!(
        &codec::registry(&Registry::empty()).unwrap()[..8],
        b"CAREG001"
    );
}
#[test]
fn every_external_forward_and_restoration_effect_failure_recovers_without_false_success() {
    let mut baseline = Setup::new(1, 2);
    let id = baseline.begin(true);
    run(
        &mut baseline.store,
        &baseline.root,
        &mut baseline.effects,
        id,
    );
    let count = baseline.effects.0.borrow().events.len();
    assert!(count >= 20);
    for fail in 1..=count {
        let mut f = Setup::new(1, 2);
        let id = f.begin(true);
        f.effects.0.borrow_mut().refresh = true;
        f.effects.0.borrow_mut().fail = Some(fail);
        for _ in 0..100 {
            if f.store.journal(id).unwrap().terminal() {
                break;
            }
            if f.store.advance_switch(&f.root, &mut f.effects, id).is_err() {
                break;
            }
            if matches!(f.phase(id), SwitchPhase::Recovery | SwitchPhase::Conflict) {
                break;
            }
        }
        f.effects.0.borrow_mut().fail = None;
        let mut s = load(f.disk.clone(), &f.root);
        recover_to_terminal(&mut s, &f.root, &mut f.effects, id);
        let j = s.journal(id).unwrap();
        assert!(j.terminal());
        assert!(f.effects.0.borrow().helper.is_none());
        if j.committed {
            assert_eq!(f.effects.0.borrow().live, values(B1, 2));
        } else {
            assert_eq!(f.effects.0.borrow().live, values(A1, 1));
        }
    }
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::TargetInstalled);
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    f.store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .unwrap();
    let disk = f.disk.snapshot();
    let world = f.effects.fork();
    let mut probe = load(disk.snapshot(), &f.root);
    let mut e = world.fork();
    probe
        .recover_switch(&f.root, &mut e, id, Choice::Restore)
        .unwrap();
    run(&mut probe, &f.root, &mut e, id);
    let count = e.0.borrow().events.len();
    for fail in 1..=count {
        let d = disk.snapshot();
        let mut s = load(d.clone(), &f.root);
        let mut e = world.fork();
        s.recover_switch(&f.root, &mut e, id, Choice::Restore)
            .unwrap();
        e.0.borrow_mut().fail = Some(fail);
        for _ in 0..30 {
            if s.journal(id).unwrap().terminal() {
                break;
            }
            if s.advance_switch(&f.root, &mut e, id).is_err() {
                break;
            }
        }
        e.0.borrow_mut().fail = None;
        let mut s = load(d, &f.root);
        recover_to_terminal(&mut s, &f.root, &mut e, id);
        assert_eq!(e.0.borrow().live, values(A1, 1));
        assert_eq!(s.journal(id).unwrap().failure, Some(Failure::Cancelled));
    }
}
// Native storage reuses these controlled memory-only external effects. No real
// credential path or process adapter is compiled into the synthetic driver.
pub(crate) fn native_roundtrip<D: Files>(s: &mut Storage<D>, root: &RootKey) {
    let (a, _) = s
        .add(
            root,
            text(),
            text(),
            capture_for("SYNTHETIC_A", &values(A0, 1)),
        )
        .unwrap();
    let (b, _) = s
        .add(
            root,
            text(),
            text(),
            capture_for("SYNTHETIC_B", &values(B0, 2)),
        )
        .unwrap();
    let mut e = Synthetic::new(values(A1, 1));
    let req = request(s, a, b, true);
    let id = s.begin_switch(root, &mut e, req).unwrap();
    run(s, root, &mut e, id);
    assert_eq!(e.0.borrow().live, values(B0, 2));
    let req = request(s, b, a, false);
    let id = s.begin_switch(root, &mut e, req).unwrap();
    run(s, root, &mut e, id);
    assert_eq!(e.0.borrow().live, values(A1, 1));
}

// Copy an already-running synthetic instance BEFORE injecting a fault. This is
// not a restart or native-authority receipt. After the fault all runtime fields
// are discarded and only independently persisted encrypted bytes are reopened.
fn running_copy(s: &Storage<Memory>, disk: Memory) -> Storage<Memory> {
    Storage {
        disk,
        state: s.state.clone(),
        state_bytes: s.state_bytes.clone(),
        registry: s.copy_registry().unwrap(),
        later_startup: s.later_startup,
        blocked: s.blocked,
        control_repair: s.control_repair,
        torn_control: s.torn_control.clone(),
        switch_session: s.switch_session,
    }
}
fn fault_transition(f: &Setup, id: Id) -> usize {
    let base = f.disk.snapshot();
    let world = f.effects.fork();
    let probe = base.snapshot();
    let mut s = running_copy(&f.store, probe.clone());
    let mut effects = world.fork();
    s.advance_switch(&f.root, &mut effects, id).unwrap();
    let count = probe.0.borrow().step;
    for step in 1..=count {
        let disk = base.snapshot();
        let mut s = running_copy(&f.store, disk.clone());
        let mut e = world.fork();
        disk.fail(step, StorageError::Io);
        assert!(
            s.advance_switch(&f.root, &mut e, id).is_err(),
            "fault must interrupt the selected durable boundary"
        );
        drop(s);
        disk.heal();
        let mut restarted = load(disk.clone(), &f.root);
        recover_to_terminal(&mut restarted, &f.root, &mut e, id);
        let j = restarted.journal(id).unwrap();
        assert!(j.terminal());
        assert!(j.cleaned);
        assert_eq!(
            e.0.borrow().live,
            if j.committed {
                values(B1, 2)
            } else {
                values(A1, 1)
            }
        );
        if e.0.borrow().live[0].as_deref() == Some(B1)
            || world.0.borrow().live[0].as_deref() == Some(B1)
        {
            assert_eq!(
                restarted
                    .read_latest(&f.root, f.b)
                    .unwrap()
                    .resource(ResourceId::new(0).unwrap())
                    .unwrap()
                    .as_bytes(),
                Some(B1)
            );
        }
        Storage::<Memory>::verify_registry_on(&disk, &f.root, &restarted.registry).unwrap();
        let hold = restarted
            .registry
            .holds
            .iter()
            .find(|h| h.id == id)
            .unwrap();
        assert_eq!(hold.generations, j.references());
        // The completion record can be published before a final flush reports
        // failure. Retain its awaiting-confirmation observation, not a fabricated
        // Desktop confirmation. An unrecorded launch remains unknown.
        assert_eq!(
            j.status().observations.desktop_identity(),
            if j.launch == DesktopLaunch::Opened {
                DesktopIdentity::AwaitingConfirmation
            } else {
                DesktopIdentity::Unknown
            }
        );
    }
    count
}
#[test]
fn every_durable_forward_and_restoration_boundary_reopens_reachable_generations() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    let mut forward = 0;
    for _ in 0..100 {
        if f.store.journal(id).unwrap().terminal() {
            break;
        }
        forward += fault_transition(&f, id);
        f.store.advance_switch(&f.root, &mut f.effects, id).unwrap();
    }
    assert!(f.store.journal(id).unwrap().terminal());
    assert!(forward > 400);
    let mut f = Setup::new(1, 2);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    f.until(id, SwitchPhase::Observed);
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    f.store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Restore)
        .unwrap();
    let mut restoration = 0;
    for _ in 0..20 {
        if f.store.journal(id).unwrap().terminal() {
            break;
        }
        restoration += fault_transition(&f, id);
        f.store.advance_switch(&f.root, &mut f.effects, id).unwrap();
    }
    assert_eq!(f.phase(id), SwitchPhase::Restored);
    assert!(restoration > 150);
}
#[test]
fn late_external_change_blocks_launch_without_reverting_committed_selection() {
    let mut f = Setup::new(0, 1);
    let id = f.begin(false);
    f.until(id, SwitchPhase::RelaunchRequested);
    f.effects.0.borrow_mut().live[0] = Some(b"SYNTHETIC_EXTERNAL_AFTER_COMMIT".to_vec());
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    let j = f.store.journal(id).unwrap();
    assert!(j.committed);
    assert_eq!(j.phase, SwitchPhase::Conflict);
    assert_eq!(f.effects.0.borrow().launches, 0);
    assert_eq!(
        f.store
            .recover_switch(&f.root, &mut f.effects, id, Choice::Restore),
        Err(SwitchError::AlreadyCommitted)
    );
}
#[test]
fn authenticated_but_tampered_journal_payload_cannot_be_selected_on_restart() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::SourceSaved);
    let mut r = f.store.copy_registry().unwrap();
    r.journals[0].target_marks[0].tag[0] ^= 1;
    // Structural validity alone is insufficient. The pre-publication graph check
    // must independently recompute expected fingerprints from encrypted resources.
    assert!(f.store.commit(&f.root, r, vec![], vec![]).is_err());
    let mut restarted = Storage::open(f.disk.clone(), &f.root).unwrap();
    assert!(restarted.reconcile(&f.root).is_err());
    let mut restarted = Storage::open(f.disk.clone(), &f.root).unwrap();
    restarted.restore_previous(&f.root).unwrap();
    assert_eq!(f.effects.0.borrow().live, values(A1, 1));
    assert_eq!(
        restarted.journal(id).unwrap().phase,
        SwitchPhase::SourceSaved
    );
}

#[test]
fn durable_request_and_cancellation_boundaries_restart_without_untracked_effects() {
    let initial = Setup::new(1, 2);
    let disk = initial.disk.snapshot();
    let mut probe = running_copy(&initial.store, disk.clone());
    let mut world = initial.effects.fork();
    probe
        .begin_switch(&initial.root, &mut world, initial.request(true))
        .unwrap();
    let count = disk.0.borrow().step;
    assert!(count > 20);
    for step in 1..=count {
        let disk = initial.disk.snapshot();
        let mut store = running_copy(&initial.store, disk.clone());
        let mut world = initial.effects.fork();
        disk.fail(step, StorageError::Io);
        assert!(store
            .begin_switch(&initial.root, &mut world, initial.request(true))
            .is_err());
        drop(store);
        disk.heal();
        let mut store = load(disk, &initial.root);
        let ids: Vec<_> = store.registry.journals.iter().map(|j| j.id).collect();
        for id in ids {
            recover_to_terminal(&mut store, &initial.root, &mut world, id);
        }
        assert_eq!(world.0.borrow().live, values(A1, 1));
        assert!(world.0.borrow().staged.is_empty());
        assert_eq!(world.0.borrow().launches, 0);
    }
    for phase in [SwitchPhase::SourceSaved, SwitchPhase::HelperRunning] {
        let mut f = Setup::new(1, 2);
        let id = f.begin(true);
        f.effects.0.borrow_mut().refresh = true;
        f.until(id, phase);
        let disk = f.disk.snapshot();
        let mut probe = running_copy(&f.store, disk.clone());
        probe
            .cancel_switch(&f.root, &mut f.effects.fork(), id)
            .unwrap();
        let count = disk.0.borrow().step;
        assert!(count > 40);
        for step in 1..=count {
            let disk = f.disk.snapshot();
            let mut store = running_copy(&f.store, disk.clone());
            let mut world = f.effects.fork();
            disk.fail(step, StorageError::Io);
            assert!(store.cancel_switch(&f.root, &mut world, id).is_err());
            drop(store);
            disk.heal();
            let mut store = load(disk, &f.root);
            recover_to_terminal(&mut store, &f.root, &mut world, id);
            assert_eq!(world.0.borrow().live, values(A1, 1));
            assert!(world.0.borrow().helper.is_none());
            assert!(world.0.borrow().staged.is_empty());
            if phase == SwitchPhase::HelperRunning {
                assert_eq!(
                    store
                        .read_latest(&f.root, f.b)
                        .unwrap()
                        .resource(ResourceId::new(0).unwrap())
                        .unwrap()
                        .as_bytes(),
                    Some(B1)
                );
            }
        }
    }
}

#[test]
fn durable_recovery_choice_and_refresh_capture_boundaries_preserve_latest_target() {
    let mut f = Setup::new(1, 2);
    let id = f.begin(true);
    f.effects.0.borrow_mut().refresh = true;
    f.until(id, SwitchPhase::HelperReaping);
    f.store.cancel_switch(&f.root, &mut f.effects, id).unwrap();
    let disk = f.disk.snapshot();
    let mut probe = running_copy(&f.store, disk.clone());
    probe
        .recover_switch(&f.root, &mut f.effects.fork(), id, Choice::Restore)
        .unwrap();
    let count = disk.0.borrow().step;
    assert!(count > 40);
    for step in 1..=count {
        let disk = f.disk.snapshot();
        let mut store = running_copy(&f.store, disk.clone());
        let mut world = f.effects.fork();
        disk.fail(step, StorageError::Io);
        assert!(store
            .recover_switch(&f.root, &mut world, id, Choice::Restore)
            .is_err());
        drop(store);
        disk.heal();
        let mut store = load(disk.clone(), &f.root);
        recover_to_terminal(&mut store, &f.root, &mut world, id);
        assert_eq!(world.0.borrow().live, values(A1, 1));
        assert_eq!(store.journal(id).unwrap().failure, Some(Failure::Cancelled));
        assert_eq!(
            store
                .read_latest(&f.root, f.b)
                .unwrap()
                .resource(ResourceId::new(0).unwrap())
                .unwrap()
                .as_bytes(),
            Some(B1)
        );
        Storage::<Memory>::verify_registry_on(&disk, &f.root, &store.registry).unwrap();
    }
}

#[test]
fn sensitive_transition_guards_and_repeated_cleanup_failure_remain_blocking() {
    for (phase, check) in [
        (SwitchPhase::Quiescent, Check::Write),
        (SwitchPhase::SourceSaved, Check::Write),
        (SwitchPhase::TargetStaged, Check::Write),
        (SwitchPhase::Applying, Check::Write),
        (SwitchPhase::TargetInstalled, Check::Helper),
        (SwitchPhase::Observed, Check::Commit),
        (SwitchPhase::RelaunchRequested, Check::Launch),
    ] {
        for failure in [Failure::Binding, Failure::Policy, Failure::Writers] {
            let mut f = Setup::new(1, 2);
            let id = f.begin(true);
            f.until(id, phase);
            let before = f.effects.fork();
            f.effects.0.borrow_mut().deny = Some((check, failure));
            let result = f.store.advance_switch(&f.root, &mut f.effects, id);
            if phase == SwitchPhase::RelaunchRequested {
                result.unwrap();
            } else {
                assert_eq!(result, Err(SwitchError::Refused(failure)));
            }
            assert_eq!(f.store.journal(id).unwrap().failure, Some(failure));
            assert_eq!(f.effects.0.borrow().live, before.0.borrow().live);
            assert_eq!(f.effects.0.borrow().staged, before.0.borrow().staged);
            assert_eq!(f.effects.0.borrow().helper, before.0.borrow().helper);
            assert_eq!(f.effects.0.borrow().launches, 0);
        }
    }
    let mut f = Setup::new(1, 2);
    let id = f.begin(false);
    f.until(id, SwitchPhase::Committed);
    f.effects.0.borrow_mut().events.clear();
    f.effects.0.borrow_mut().fail = Some(1);
    assert!(f.store.advance_switch(&f.root, &mut f.effects, id).is_err());
    let mut store = load(f.disk.clone(), &f.root);
    f.effects.0.borrow_mut().events.clear();
    assert!(store
        .recover_switch(&f.root, &mut f.effects, id, Choice::Finish)
        .is_err());
    assert!(!store.journal(id).unwrap().terminal());
    assert_eq!(store.prune(&f.root), Err(StorageError::SwitchPending));
    assert_eq!(f.effects.0.borrow().live, values(B0, 2));
    assert_eq!(f.effects.0.borrow().launches, 0);
    f.effects.0.borrow_mut().fail = None;
    recover_to_terminal(&mut store, &f.root, &mut f.effects, id);
    assert!(store.journal(id).unwrap().cleaned);
}

#[cfg(all(windows, target_arch = "x86_64"))]
pub(crate) fn native_prepare_interrupted<D: Files>(
    s: &mut Storage<D>,
    root: &RootKey,
) -> (Id, Synthetic) {
    let (a, _) = s
        .add(
            root,
            text(),
            text(),
            capture_for("SYNTHETIC_A", &values(A0, 1)),
        )
        .unwrap();
    let (b, _) = s
        .add(
            root,
            text(),
            text(),
            capture_for("SYNTHETIC_B", &values(B0, 2)),
        )
        .unwrap();
    let mut e = Synthetic::new(values(A1, 1));
    e.0.borrow_mut().refresh = true;
    let id = s
        .begin_switch(root, &mut e, request(s, a, b, true))
        .unwrap();
    for _ in 0..100 {
        if s.journal(id).unwrap().phase == SwitchPhase::HelperReaping {
            return (id, e);
        }
        s.advance_switch(root, &mut e, id).unwrap();
    }
    panic!("bounded native journal preparation did not complete")
}
#[cfg(all(windows, target_arch = "x86_64"))]
pub(crate) fn native_finish_interrupted<D: Files>(
    s: &mut Storage<D>,
    root: &RootKey,
    id: Id,
    e: &mut Synthetic,
) {
    assert_eq!(s.recovery(), Recovery::SwitchPending);
    assert_eq!(
        s.advance_switch(root, e, id),
        Err(SwitchError::Storage(StorageError::SwitchPending))
    );
    assert_eq!(s.prune(root), Err(StorageError::SwitchPending));
    s.recover_switch(root, e, id, Choice::Restore).unwrap();
    run(s, root, e, id);
    let j = s.journal(id).unwrap();
    assert_eq!(j.phase, SwitchPhase::Restored);
    assert_eq!(e.0.borrow().live, values(A1, 1));
    assert_eq!(
        s.read_latest(root, ProfileId::from_bytes(j.target.profile).unwrap())
            .unwrap()
            .resource(ResourceId::new(0).unwrap())
            .unwrap()
            .as_bytes(),
        Some(B1)
    );
}

#[path = "stage_repair_tests.rs"]
mod stage_repair_tests;
