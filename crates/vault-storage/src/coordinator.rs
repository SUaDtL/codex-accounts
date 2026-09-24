//! Storage-backed cold-handoff coordinator. This module is private: the only
//! Effects implementation is cfg(test). There is no production qualification,
//! consent, filesystem, process, helper or launch adapter in CA-04A.
#![forbid(unsafe_code)]
use super::*;
use crate::journal::*;
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};
use codex_accounts_vault::Identity;
use codex_accounts_vault_crypto::Fingerprint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Check {
    Confirm,
    Lock,
    Quiesce,
    Write,
    Helper,
    Commit,
    Launch,
    Recovery,
}
/// Narrow internal effects; no path, executable, URL, RPC, or arbitrary callback
/// crosses the public library boundary. Return success only after the indicated
/// modeled prerequisite/effect actually happened. Future native adapters require
/// their own reviewed evidence and must never accept the synthetic implementation.
pub(crate) trait Effects {
    fn confirm(&mut self, request: &Request) -> Result<(), Failure>;
    fn guard(&mut self, check: Check, binding: &[u8; 32]) -> Result<(), Failure>;
    fn snapshot(&mut self) -> Result<Snapshot, Failure>;
    fn stage(&mut self, operation: Id, slot: u8, value: Option<&[u8]>) -> Result<(), Failure>;
    fn replace(
        &mut self,
        operation: Id,
        slot: u8,
        expected: Option<&[u8]>,
        value: Option<&[u8]>,
    ) -> Result<(), Failure>;
    fn cleanup(&mut self, operation: Id, registered: u16) -> Result<(), Failure>;
    fn start_helper(&mut self, operation: Id) -> Result<(), Failure>;
    fn observe(&mut self, operation: Id) -> Result<CredentialAcceptance, Failure>;
    /// Success means the exact owned helper AND descendants have exited, not
    /// signal delivery or an observation from an unrelated/stale process identity.
    fn reap_helper(&mut self, operation: Id) -> Result<(), Failure>;
    fn launch(&mut self, operation: Id) -> Result<DesktopLaunch, Failure>;
}
pub(crate) struct Snapshot {
    pub identity: Option<Identity>,
    pub schema: u32,
    pub resources: Vec<Resource>,
}
impl Snapshot {
    fn resource(&self, slot: u8) -> Result<Option<&[u8]>, Failure> {
        self.resources
            .iter()
            .find(|r| r.id() == ResourceId::new(slot).expect("bounded slot"))
            .map(|r| r.as_bytes())
            .ok_or(Failure::ExternalChange)
    }
    fn shape(&self, g: &Generation) -> Result<(), Failure> {
        let actual: BTreeSet<_> = self.resources.iter().map(|r| r.id()).collect();
        let expected: BTreeSet<_> = g
            .rules
            .iter()
            .map(|r| ResourceId::new(r.slot).expect("validated slot"))
            .collect();
        if self.schema != g.schema || actual.len() != self.resources.len() || actual != expected {
            return Err(Failure::ExternalChange);
        }
        Ok(())
    }
    fn capture(&self, g: &Generation, identity: &Identity) -> Result<Capture, Failure> {
        self.shape(g)?;
        if self.identity.as_ref() != Some(identity) {
            return Err(Failure::ExternalChange);
        }
        let (i, s, w) = identity.components();
        let identity = Identity::new(i.to_owned(), s.to_owned(), w.to_owned())
            .map_err(|_| Failure::InvalidData)?;
        let mut resources = vec![];
        for resource in &self.resources {
            resources.push(match resource.as_bytes() {
                Some(bytes) => Resource::present(resource.id(), bytes.to_vec())
                    .map_err(|_| Failure::InvalidData)?,
                None => Resource::absent(resource.id()),
            });
        }
        Capture::new(
            identity,
            g.schema,
            g.rules
                .iter()
                .map(|r| {
                    (
                        ResourceId::new(r.slot).expect("validated slot"),
                        if r.json {
                            ResourceShape::JsonObject
                        } else {
                            ResourceShape::Opaque
                        },
                        r.required,
                    )
                })
                .collect(),
            resources,
        )
        .map_err(|_| Failure::InvalidData)
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Request {
    pub source: GenerationRef,
    pub target: GenerationRef,
    pub binding: [u8; 32],
    pub online: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwitchError {
    Storage(StorageError),
    Refused(Failure),
    InvalidTransition,
    AlreadyCommitted,
}
impl From<StorageError> for SwitchError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}
impl From<Failure> for SwitchError {
    fn from(e: Failure) -> Self {
        Self::Refused(e)
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Choice {
    Restore,
    Finish,
}

fn generation(r: &Registry, g: GenerationRef) -> Result<&Generation, StorageError> {
    r.generations
        .iter()
        .find(|v| v.id == g.generation && v.profile == g.profile)
        .ok_or(StorageError::Corrupt)
}
fn mark_context(g: GenerationRef, slot: u8) -> Result<EnvelopeContext, StorageError> {
    Key::resource(g.profile, g.generation, slot).context()
}
fn marks<D: Files>(disk: &D, root: &RootKey, g: &Generation) -> Result<Vec<Mark>, StorageError> {
    let set = Storage::<D>::read_generation_on(disk, root, g)?;
    g.rules
        .iter()
        .map(|r| {
            let bytes = set
                .resource(ResourceId::new(r.slot)?)
                .ok_or(StorageError::Corrupt)?
                .as_bytes();
            let tag = *root
                .fingerprint(&mark_context(GenerationRef::from(g), r.slot)?, bytes)?
                .storage_bytes();
            Ok(Mark {
                slot: r.slot,
                present: bytes.is_some(),
                tag,
            })
        })
        .collect()
}
fn matches(
    root: &RootKey,
    g: GenerationRef,
    mark: &Mark,
    bytes: Option<&[u8]>,
) -> Result<bool, StorageError> {
    Ok(mark.present == bytes.is_some()
        && root
            .verify_fingerprint(
                &Fingerprint::from_storage_bytes(&mark.tag)?,
                &mark_context(g, mark.slot)?,
                bytes,
            )
            .is_ok())
}
/// Verify independent generation references, ALL persisted holds, keyed expected
/// resources and encrypted evidence before a registry becomes selectable.
pub(super) fn verify_journals<D: Files>(
    disk: &D,
    root: &RootKey,
    r: &Registry,
) -> Result<(), StorageError> {
    for j in &r.journals {
        for (reference, expected) in [(j.source, &j.source_marks), (j.target, &j.target_marks)] {
            if marks(disk, root, generation(r, reference)?)? != *expected {
                return Err(StorageError::Corrupt);
            }
        }
        for e in &j.evidence {
            for resource in &e.resources {
                if let Some(b) = &resource.blob {
                    let encrypted = Storage::<D>::read_blob(disk, b)?;
                    root.open(&b.key.context()?, &Envelope::from_bytes(&encrypted)?)?;
                }
            }
        }
    }
    Ok(())
}
impl<D: Files> Storage<D> {
    fn journal(&self, id: Id) -> Result<Journal, SwitchError> {
        self.ready()?;
        self.registry
            .journals
            .iter()
            .find(|j| j.id == id)
            .cloned()
            .ok_or(SwitchError::InvalidTransition)
    }
    fn write_journal(
        &mut self,
        root: &RootKey,
        mut j: Journal,
        mut next: Registry,
        writes: EncryptedWrites,
        active: Option<Id>,
    ) -> Result<(), StorageError> {
        j.updated = now()?;
        next.format = 2;
        next.holds.retain(|h| h.id != j.id);
        next.holds.push(Hold {
            id: j.id,
            generations: j.references(),
        });
        if j.acceptance == CredentialAcceptance::Rejected
            && !next.rejected.contains(&j.target.profile)
        {
            next.rejected.push(j.target.profile);
        }
        if let Some(active) = active {
            next.active = Some(active);
            next.active_known = true;
        }
        match next.journals.iter_mut().find(|v| v.id == j.id) {
            Some(existing) => *existing = j,
            None => next.journals.push(j),
        }
        self.commit(root, next, writes, vec![])
    }
    fn save_journal(&mut self, root: &RootKey, j: Journal) -> Result<(), StorageError> {
        let next = self.copy_registry()?;
        self.write_journal(root, j, next, vec![], None)
    }
    pub(crate) fn begin_switch(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        request: Request,
    ) -> Result<Id, SwitchError> {
        self.idle()?;
        if self.registry.journals.len() >= MAX_JOURNALS {
            return Err(StorageError::InputLimit.into());
        }
        if self
            .registry
            .holds
            .iter()
            .any(|h| !self.registry.journals.iter().any(|j| j.id == h.id))
        {
            return Err(StorageError::SwitchPending.into());
        }
        if request.source.profile == request.target.profile || request.binding == [0; 32] {
            return Err(SwitchError::InvalidTransition);
        }
        for reference in [request.source, request.target] {
            let p = self
                .registry
                .profiles
                .iter()
                .find(|p| p.id == reference.profile)
                .ok_or(StorageError::Missing)?;
            if p.latest != reference.generation {
                return Err(StorageError::StaleParent.into());
            }
        }
        if self.registry.active_known && self.registry.active != Some(request.source.profile) {
            return Err(Failure::ExternalChange.into());
        }
        if self.registry.rejected.contains(&request.target.profile) {
            return Err(Failure::LoginRequired.into());
        }
        let source = generation(&self.registry, request.source)?;
        let target = generation(&self.registry, request.target)?;
        if source.schema != target.schema
            || source
                .rules
                .iter()
                .map(|r| (r.slot, r.json, r.required))
                .collect::<Vec<_>>()
                != target
                    .rules
                    .iter()
                    .map(|r| (r.slot, r.json, r.required))
                    .collect::<Vec<_>>()
        {
            return Err(Failure::Binding.into());
        }
        effects.confirm(&request)?;
        let id = random_id()?;
        if self.registry.journals.iter().any(|j| j.id == id) {
            return Err(StorageError::AlreadyExists.into());
        }
        let timestamp = now()?;
        let j = Journal {
            id,
            binding: request.binding,
            created: timestamp,
            updated: timestamp,
            source: request.source,
            target: request.target,
            original_target: request.target,
            phase: SwitchPhase::Requested,
            source_saved: false,
            online: request.online,
            staging_intent: 0,
            staging_done: 0,
            write_intent: 0,
            write_done: 0,
            restore_intent: 0,
            restore_done: 0,
            helper: Helper::Never,
            cancel: false,
            committed: false,
            cleaned: false,
            failure: None,
            restoration_failure: None,
            acceptance: CredentialAcceptance::Unknown,
            launch: DesktopLaunch::NotRequested,
            source_marks: marks(&self.disk, root, source)?,
            target_marks: marks(&self.disk, root, target)?,
            evidence: vec![],
        };
        self.save_journal(root, j)?;
        self.switch_session = Some(id);
        Ok(id)
    }
    fn capture_journal(
        &mut self,
        root: &RootKey,
        mut j: Journal,
        snapshot: &Snapshot,
        target: bool,
    ) -> Result<(), SwitchError> {
        let reference = if target { j.target } else { j.source };
        let template = generation(&self.registry, reference)?;
        let p = self
            .registry
            .profiles
            .iter()
            .find(|p| p.id == reference.profile)
            .ok_or(StorageError::Corrupt)?;
        if p.latest != reference.generation {
            return Err(StorageError::StaleParent.into());
        }
        let capture = snapshot.capture(template, &p.identity)?;
        let expected = if target {
            &j.target_marks
        } else {
            &j.source_marks
        };
        let mut same = true;
        for mark in expected {
            same &= matches(root, reference, mark, snapshot.resource(mark.slot)?)?;
        }
        let mut next = self.copy_registry()?;
        let mut writes = vec![];
        if !same {
            if next.generations.len() >= MAX_GENERATIONS {
                return Err(StorageError::InputLimit.into());
            }
            let (g, new_writes) = self.make_generation(
                root,
                reference.profile,
                Some(reference.generation),
                &capture,
            )?;
            let reference = GenerationRef::from(&g);
            let mut expected = vec![];
            for r in &g.rules {
                let bytes = snapshot.resource(r.slot)?;
                expected.push(Mark {
                    slot: r.slot,
                    present: bytes.is_some(),
                    tag: *root
                        .fingerprint(&mark_context(reference, r.slot)?, bytes)
                        .map_err(StorageError::from)?
                        .storage_bytes(),
                });
            }
            if target {
                j.target = reference;
                j.target_marks = expected;
            } else {
                j.source = reference;
                j.source_marks = expected;
            }
            next.profiles
                .iter_mut()
                .find(|p| p.id == g.profile)
                .ok_or(StorageError::Corrupt)?
                .latest = g.id;
            next.generations.push(g);
            writes = new_writes;
        }
        if target {
            j.helper = Helper::Reaped;
            j.phase = if j.failure.is_some() {
                SwitchPhase::Recovery
            } else {
                SwitchPhase::Observed
            };
        } else {
            j.source_saved = true;
            j.phase = SwitchPhase::SourceSaved;
        }
        self.write_journal(root, j, next, writes, None)?;
        Ok(())
    }
    fn exact_live(
        &self,
        root: &RootKey,
        j: &Journal,
        s: &Snapshot,
        target: bool,
    ) -> Result<(), SwitchError> {
        let reference = if target { j.target } else { j.source };
        s.shape(generation(&self.registry, reference)?)?;
        for mark in if target {
            &j.target_marks
        } else {
            &j.source_marks
        } {
            if !matches(root, reference, mark, s.resource(mark.slot)?)? {
                return Err(Failure::ExternalChange.into());
            }
        }
        Ok(())
    }
    fn mixed_live(&self, root: &RootKey, j: &Journal, s: &Snapshot) -> Result<(), SwitchError> {
        s.shape(generation(&self.registry, j.source)?)?;
        for (a, b) in j.source_marks.iter().zip(&j.target_marks) {
            let bit = 1 << a.slot;
            let bytes = s.resource(a.slot)?;
            let source = matches(root, j.source, a, bytes)?;
            let target = matches(root, j.target, b, bytes)?;
            let valid = if j.restore_done & bit != 0 {
                source
            } else if j.restore_intent & bit != 0 {
                source || target
            } else if j.write_done & bit != 0 {
                target
            } else if j.write_intent & bit != 0 {
                source || target
            } else {
                source
            };
            if !valid {
                return Err(Failure::ExternalChange.into());
            }
        }
        Ok(())
    }
    fn fail_switch(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        mut j: Journal,
        f: Failure,
        restoring: bool,
    ) -> Result<(), SwitchError> {
        if !restoring && j.failure.is_none() {
            j.failure = Some(f);
        }
        if restoring {
            j.restoration_failure = Some(f);
        }
        j.phase = if matches!(f, Failure::ExternalChange | Failure::InvalidData) {
            SwitchPhase::Conflict
        } else {
            SwitchPhase::Recovery
        };
        if matches!(f, Failure::ExternalChange | Failure::InvalidData) {
            // Reading failure cannot be disguised as captured evidence. Persist the
            // failure even when a bounded snapshot is unavailable; never overwrite.
            if let Ok(snapshot) = effects.snapshot() {
                return self
                    .preserve_conflict(root, j, snapshot)
                    .and(Err(SwitchError::Refused(f)));
            }
        }
        self.save_journal(root, j)?;
        Err(SwitchError::Refused(f))
    }
    fn preserve_conflict(
        &mut self,
        root: &RootKey,
        mut j: Journal,
        s: Snapshot,
    ) -> Result<(), SwitchError> {
        if s.resources.is_empty()
            || s.resources.len() > 16
            || s.resources
                .iter()
                .map(|r| r.as_bytes().map_or(0, <[u8]>::len))
                .sum::<usize>()
                > 4 * 1024 * 1024
        {
            self.save_journal(root, j)?;
            return Err(StorageError::InputLimit.into());
        }
        let ids: BTreeSet<_> = s.resources.iter().map(|r| r.id()).collect();
        if ids.len() != s.resources.len() {
            self.save_journal(root, j)?;
            return Err(StorageError::InvalidData.into());
        }
        for e in &j.evidence {
            let mut same = e.resources.len() == s.resources.len();
            for resource in &e.resources {
                let bytes = match &resource.blob {
                    Some(b) => {
                        let encrypted = Self::read_blob(&self.disk, b)?;
                        Some(
                            root.open(
                                &b.key.context()?,
                                &Envelope::from_bytes(&encrypted).map_err(StorageError::from)?,
                            )
                            .map_err(StorageError::from)?,
                        )
                    }
                    None => None,
                };
                same &= s
                    .resource(resource.slot)
                    .is_ok_and(|v| v == bytes.as_ref().map(|v| v.expose()));
            }
            if same {
                self.save_journal(root, j)?;
                return Ok(());
            }
        }
        if j.evidence.len() >= MAX_EVIDENCE {
            self.save_journal(root, j)?;
            return Err(StorageError::InputLimit.into());
        }
        let id = random_id()?;
        let mut writes = vec![];
        let mut resources = vec![];
        for resource in &s.resources {
            let slot = (0..16)
                .find(|slot| ResourceId::new(*slot).expect("bounded slot") == resource.id())
                .ok_or(StorageError::InvalidData)?;
            let blob = if let Some(bytes) = resource.as_bytes() {
                let key = Key::resource(j.id, id, slot);
                let encrypted = root
                    .seal(&key.context()?, bytes)
                    .map_err(StorageError::from)?;
                let blob = Blob::new(key, encrypted.as_bytes())?;
                writes.push((blob.clone(), encrypted.as_bytes().to_vec()));
                Some(blob)
            } else {
                None
            };
            resources.push(Rule {
                slot,
                json: false,
                required: false,
                blob,
            });
        }
        j.evidence.push(Evidence { id, resources });
        let next = self.copy_registry()?;
        self.write_journal(root, j, next, writes, None)?;
        Ok(())
    }
    pub(crate) fn advance_switch(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        id: Id,
    ) -> Result<(), SwitchError> {
        let j = self.journal(id)?;
        if j.terminal() {
            return Ok(());
        }
        if self.switch_session != Some(id) {
            return Err(StorageError::SwitchPending.into());
        }
        let restoring = j.phase == SwitchPhase::Restoring;
        match self.advance_inner(root, effects, j.clone()) {
            Err(SwitchError::Refused(f)) => {
                let current = self.journal(id)?;
                self.fail_switch(root, effects, current, f, restoring)
            }
            other => other,
        }
    }
    fn advance_inner(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        mut j: Journal,
    ) -> Result<(), SwitchError> {
        use SwitchPhase::*;
        match j.phase {
            Requested => {
                effects.guard(Check::Lock, &j.binding)?;
                j.phase = Locked;
                self.save_journal(root, j)?;
            }
            Locked => {
                effects.guard(Check::Quiesce, &j.binding)?;
                j.phase = Quiescent;
                self.save_journal(root, j)?;
            }
            Quiescent => {
                effects.guard(Check::Write, &j.binding)?;
                let s = effects.snapshot()?;
                self.capture_journal(root, j, &s, false)?;
            }
            SourceSaved => {
                effects.guard(Check::Write, &j.binding)?;
                self.exact_live(root, &j, &effects.snapshot()?, false)?;
                if let Some(mark) = j
                    .target_marks
                    .iter()
                    .find(|m| j.staging_done & (1 << m.slot) == 0)
                {
                    let slot = mark.slot;
                    let bit = 1 << slot;
                    let set = self.read_generation(root, generation(&self.registry, j.target)?)?;
                    let bytes = set
                        .resource(ResourceId::new(slot).map_err(StorageError::from)?)
                        .ok_or(StorageError::Corrupt)?
                        .as_bytes();
                    j.staging_intent |= bit;
                    self.save_journal(root, j.clone())?;
                    effects.guard(Check::Write, &j.binding)?;
                    self.exact_live(root, &j, &effects.snapshot()?, false)?;
                    effects.stage(j.id, slot, bytes)?;
                    j.staging_done |= bit;
                    self.save_journal(root, j)?;
                } else {
                    j.phase = TargetStaged;
                    self.save_journal(root, j)?;
                }
            }
            TargetStaged | Applying => {
                effects.guard(Check::Write, &j.binding)?;
                let snapshot = effects.snapshot()?;
                self.mixed_live(root, &j, &snapshot)?;
                if let Some(mark) = j
                    .target_marks
                    .iter()
                    .find(|m| j.write_done & (1 << m.slot) == 0)
                {
                    let slot = mark.slot;
                    let bit = 1 << slot;
                    let set = self.read_generation(root, generation(&self.registry, j.target)?)?;
                    let value = set
                        .resource(ResourceId::new(slot).map_err(StorageError::from)?)
                        .ok_or(StorageError::Corrupt)?
                        .as_bytes();
                    j.phase = Applying;
                    j.write_intent |= bit;
                    self.save_journal(root, j.clone())?;
                    effects.guard(Check::Write, &j.binding)?;
                    let before = effects.snapshot()?;
                    self.mixed_live(root, &j, &before)?;
                    if before.resource(slot)? != snapshot.resource(slot)? {
                        return Err(Failure::ExternalChange.into());
                    }
                    effects.replace(j.id, slot, snapshot.resource(slot)?, value)?;
                    let after = effects.snapshot()?;
                    j.write_done |= bit;
                    self.mixed_live(root, &j, &after)?;
                    self.save_journal(root, j)?;
                } else {
                    j.phase = TargetInstalled;
                    self.save_journal(root, j)?;
                }
            }
            TargetInstalled => {
                effects.guard(Check::Helper, &j.binding)?;
                self.exact_live(root, &j, &effects.snapshot()?, true)?;
                if j.online {
                    // Persist launch intent before a helper could begin writing.
                    j.helper = Helper::MayRun;
                    j.phase = HelperRunning;
                    self.save_journal(root, j.clone())?;
                    effects.guard(Check::Helper, &j.binding)?;
                    self.exact_live(root, &j, &effects.snapshot()?, true)?;
                    effects.start_helper(j.id)?;
                } else {
                    j.phase = Observed;
                    self.save_journal(root, j)?;
                }
            }
            HelperRunning => {
                // Transport unavailability is an explicit observation value. Other
                // failures must not be laundered into permission to commit. Reap
                // and preserve any refreshed bytes before offering recovery.
                j.acceptance = match effects.observe(j.id) {
                    Ok(value) => value,
                    Err(f) => {
                        j.failure.get_or_insert(f);
                        if f == Failure::LoginRequired {
                            CredentialAcceptance::Rejected
                        } else {
                            CredentialAcceptance::ObservationUnavailable
                        }
                    }
                };
                j.phase = HelperReaping;
                self.save_journal(root, j)?;
            }
            HelperReaping => {
                effects.reap_helper(j.id)?;
                effects.guard(Check::Write, &j.binding)?;
                let snapshot = effects.snapshot()?;
                self.capture_journal(root, j, &snapshot, true)?;
            }
            Observed => {
                if j.acceptance == CredentialAcceptance::Rejected {
                    return Err(Failure::LoginRequired.into());
                }
                effects.guard(Check::Commit, &j.binding)?;
                self.exact_live(root, &j, &effects.snapshot()?, true)?;
                j.committed = true;
                j.phase = Committed;
                let selected = j.target.profile;
                let next = self.copy_registry()?;
                self.write_journal(root, j, next, vec![], Some(selected))?;
            }
            Committed => {
                effects.cleanup(j.id, j.staging_intent)?;
                j.cleaned = true;
                j.phase = RelaunchRequested;
                self.save_journal(root, j)?;
            }
            RelaunchRequested => {
                if let Err(f) = effects.guard(Check::Launch, &j.binding) {
                    j.failure.get_or_insert(f);
                    j.launch = DesktopLaunch::Failed;
                    j.phase = Finished;
                    self.save_journal(root, j)?;
                    return Ok(());
                }
                self.exact_live(root, &j, &effects.snapshot()?, true)?;
                j.launch = effects.launch(j.id).unwrap_or(DesktopLaunch::Failed);
                if j.launch == DesktopLaunch::Opened {
                    j.phase = AwaitingConfirmation;
                } else {
                    j.phase = Finished;
                    j.failure.get_or_insert(Failure::Launch);
                }
                self.save_journal(root, j)?;
            }
            Restoring => self.restore_resource(root, effects, j)?,
            _ => return Err(SwitchError::InvalidTransition),
        }
        Ok(())
    }
    pub(crate) fn cancel_switch(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        id: Id,
    ) -> Result<(), SwitchError> {
        let mut j = self.journal(id)?;
        if j.committed {
            return Err(SwitchError::AlreadyCommitted);
        }
        if j.terminal() {
            return Err(SwitchError::InvalidTransition);
        }
        j.cancel = true;
        j.failure.get_or_insert(Failure::Cancelled);
        self.save_journal(root, j.clone())?;
        if j.write_intent == 0 {
            // Cancellation before any replacement never closes a user app or
            // changes the live bytes/active association merely to "restore" them.
            if let Err(f) = effects.cleanup(j.id, j.staging_intent) {
                return self.fail_switch(root, effects, j, f, true);
            }
            j.cleaned = true;
            j.phase = SwitchPhase::Cancelled;
        } else {
            j.phase = SwitchPhase::Recovery;
        }
        self.save_journal(root, j)?;
        Ok(())
    }
    pub(crate) fn recover_switch(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        id: Id,
        choice: Choice,
    ) -> Result<(), SwitchError> {
        let j = self.journal(id)?;
        if j.terminal() {
            return Err(SwitchError::InvalidTransition);
        }
        if j.committed && matches!(choice, Choice::Restore) {
            return Err(SwitchError::AlreadyCommitted);
        }
        match self.recover_inner(root, effects, j.clone(), choice) {
            Err(SwitchError::Refused(f)) => {
                let current = self.journal(id)?;
                self.fail_switch(root, effects, current, f, true)
            }
            other => other,
        }
    }
    fn recover_inner(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        mut j: Journal,
        choice: Choice,
    ) -> Result<(), SwitchError> {
        effects.guard(Check::Confirm, &j.binding)?;
        effects.guard(Check::Lock, &j.binding)?;
        if j.committed {
            // The app might already have launched. No automatic relaunch, signal,
            // snapshot replay or rollback is performed from an uncertain result.
            effects.cleanup(j.id, j.staging_intent)?;
            j.cleaned = true;
            j.phase = if j.launch == DesktopLaunch::Opened {
                SwitchPhase::AwaitingConfirmation
            } else {
                SwitchPhase::Finished
            };
            self.save_journal(root, j)?;
            return Ok(());
        }
        // A process/storage interruption may have prevented recording the exact
        // initial error. Preserve that uncertainty, not an invented I/O diagnosis.
        if j.failure.is_none() {
            j.failure = Some(Failure::Interrupted);
            self.save_journal(root, j.clone())?;
        }
        if j.write_intent == 0 {
            return self.cancel_switch(root, effects, j.id);
        }
        effects.guard(Check::Recovery, &j.binding)?;
        effects.guard(Check::Quiesce, &j.binding)?;
        self.switch_session = Some(j.id);
        if j.helper == Helper::MayRun {
            effects.reap_helper(j.id)?;
            effects.guard(Check::Write, &j.binding)?;
            let snapshot = effects.snapshot()?;
            self.capture_journal(root, j.clone(), &snapshot, true)?;
            j = self.journal(j.id)?;
        }
        let snapshot = effects.snapshot()?;
        self.mixed_live(root, &j, &snapshot)?;
        match choice {
            Choice::Finish => {
                if j.cancel
                    || j.restore_intent != 0
                    || j.acceptance == CredentialAcceptance::Rejected
                {
                    return Err(SwitchError::InvalidTransition);
                }
                self.exact_live(root, &j, &snapshot, true)?;
                j.write_done = j.mask();
                j.write_intent = j.mask();
                j.phase = SwitchPhase::Observed;
                self.save_journal(root, j)?;
            }
            Choice::Restore => {
                j.phase = SwitchPhase::Restoring;
                self.save_journal(root, j)?;
            }
        }
        Ok(())
    }
    fn restore_resource(
        &mut self,
        root: &RootKey,
        effects: &mut impl Effects,
        mut j: Journal,
    ) -> Result<(), SwitchError> {
        effects.guard(Check::Write, &j.binding)?;
        if j.committed || j.helper == Helper::MayRun {
            return Err(SwitchError::InvalidTransition);
        }
        let snapshot = effects.snapshot()?;
        self.mixed_live(root, &j, &snapshot)?;
        if let Some(mark) = j
            .source_marks
            .iter()
            .find(|m| j.restore_done & (1 << m.slot) == 0)
        {
            let slot = mark.slot;
            let bit = 1 << slot;
            let source = self.read_generation(root, generation(&self.registry, j.source)?)?;
            let value = source
                .resource(ResourceId::new(slot).map_err(StorageError::from)?)
                .ok_or(StorageError::Corrupt)?
                .as_bytes();
            j.restore_intent |= bit;
            self.save_journal(root, j.clone())?;
            effects.guard(Check::Write, &j.binding)?;
            let before = effects.snapshot()?;
            self.mixed_live(root, &j, &before)?;
            if before.resource(slot)? != snapshot.resource(slot)? {
                return Err(Failure::ExternalChange.into());
            }
            effects.replace(j.id, slot, snapshot.resource(slot)?, value)?;
            j.restore_done |= bit;
            self.mixed_live(root, &j, &effects.snapshot()?)?;
            self.save_journal(root, j)?;
        } else {
            self.exact_live(root, &j, &snapshot, false)?;
            effects.cleanup(j.id, j.staging_intent)?;
            j.cleaned = true;
            j.phase = SwitchPhase::Restored;
            let selected = j.source.profile;
            let next = self.copy_registry()?;
            self.write_journal(root, j, next, vec![], Some(selected))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "switch_tests.rs"]
pub(crate) mod tests;
