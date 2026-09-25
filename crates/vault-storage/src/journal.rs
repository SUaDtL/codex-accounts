//! Private authenticated operation records. No caller-provided observation grants
//! production authority; the executable effects implementation exists only in tests.
#![forbid(unsafe_code)]
use crate::{records::*, StorageError};
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch, ObservationStatus};
use std::collections::BTreeSet;
use zeroize::Zeroize;

pub(crate) const MAX_JOURNALS: usize = 16;
pub(crate) const MAX_EVIDENCE: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SwitchPhase {
    Requested = 0,
    Locked,
    Quiescent,
    SourceSaved,
    TargetStaged,
    Applying,
    TargetInstalled,
    HelperRunning,
    HelperReaping,
    Observed,
    Committed,
    RelaunchRequested,
    AwaitingConfirmation,
    Finished,
    Recovery,
    Restoring,
    Restored,
    Cancelled,
    Conflict,
}
impl SwitchPhase {
    pub(crate) fn parse(n: u8) -> Result<Self, StorageError> {
        use SwitchPhase::*;
        [
            Requested,
            Locked,
            Quiescent,
            SourceSaved,
            TargetStaged,
            Applying,
            TargetInstalled,
            HelperRunning,
            HelperReaping,
            Observed,
            Committed,
            RelaunchRequested,
            AwaitingConfirmation,
            Finished,
            Recovery,
            Restoring,
            Restored,
            Cancelled,
            Conflict,
        ]
        .get(n as usize)
        .copied()
        .ok_or(StorageError::Corrupt)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Failure {
    Binding = 1,
    Policy,
    Consent,
    Busy,
    Writers,
    ExternalChange,
    HelperStuck,
    Write,
    LoginRequired,
    Cleanup,
    Cancelled,
    Launch,
    InvalidData,
    Interrupted,
}
impl Failure {
    pub fn parse(n: u8) -> Result<Self, StorageError> {
        use Failure::*;
        [
            Binding,
            Policy,
            Consent,
            Busy,
            Writers,
            ExternalChange,
            HelperStuck,
            Write,
            LoginRequired,
            Cleanup,
            Cancelled,
            Launch,
            InvalidData,
            Interrupted,
        ]
        .get(n.checked_sub(1).ok_or(StorageError::Corrupt)? as usize)
        .copied()
        .ok_or(StorageError::Corrupt)
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Binding => "E_COMPAT_UNKNOWN",
            Self::Policy => "E_POLICY_DENIED",
            Self::Consent => "E_CONSENT_REQUIRED",
            Self::Busy => "E_BUSY",
            Self::Writers => "E_CLIENTS_RUNNING",
            Self::ExternalChange => "E_EXTERNAL_CHANGE",
            Self::HelperStuck => "E_HELPER_STUCK",
            Self::Write => "E_WRITE_FAILED",
            Self::LoginRequired => "E_LOGIN_REQUIRED",
            Self::Cleanup => "E_RECOVERY_REQUIRED",
            Self::Cancelled => "E_CANCELLED",
            Self::Launch => "E_LAUNCH_FAILED",
            Self::InvalidData => "E_VAULT_CORRUPT",
            Self::Interrupted => "E_RECOVERY_REQUIRED",
        }
    }
}
/// Random operation ID and enumerated, non-secret outcomes. No labels, account
/// identities, paths, resource bytes, keyed fingerprints, or helper output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationStatus {
    pub operation_id: [u8; 16],
    pub phase: SwitchPhase,
    pub installed_profile: Option<crate::ProfileId>,
    pub observations: ObservationStatus,
    pub primary_error: Option<&'static str>,
    pub restoration_error: Option<&'static str>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationRef {
    pub profile: Id,
    pub generation: Id,
}
impl GenerationRef {
    pub fn from(g: &Generation) -> Self {
        Self {
            profile: g.profile,
            generation: g.id,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Mark {
    pub slot: u8,
    pub present: bool,
    pub tag: [u8; 32],
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.tag.zeroize();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum EvidenceKind {
    Live = 0,
    Staging = 1,
}
#[derive(Clone)]
pub(crate) struct Evidence {
    pub kind: EvidenceKind,
    pub id: Id,
    pub resources: Vec<Rule>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Helper {
    Never = 0,
    MayRun,
    Reaped,
}
#[derive(Clone)]
pub(crate) struct Journal {
    pub id: Id,
    pub binding: [u8; 32],
    pub created: u64,
    pub updated: u64,
    pub source: GenerationRef,
    pub target: GenerationRef,
    pub original_target: GenerationRef,
    pub phase: SwitchPhase,
    pub source_saved: bool,
    pub online: bool,
    pub staging_intent: u16,
    pub staging_done: u16,
    pub write_intent: u16,
    pub write_done: u16,
    pub restore_intent: u16,
    pub restore_done: u16,
    // Ownership is the operation-scoped helper launch intent. A concrete native
    // adapter must bind actual handles/start identity, never just this random ID.
    pub helper: Helper,
    pub cancel: bool,
    pub committed: bool,
    pub cleaned: bool,
    pub failure: Option<Failure>,
    pub restoration_failure: Option<Failure>,
    pub acceptance: CredentialAcceptance,
    pub launch: DesktopLaunch,
    pub source_marks: Vec<Mark>,
    pub target_marks: Vec<Mark>,
    pub evidence: Vec<Evidence>,
}
impl Journal {
    pub fn terminal(&self) -> bool {
        matches!(
            self.phase,
            SwitchPhase::AwaitingConfirmation
                | SwitchPhase::Finished
                | SwitchPhase::Restored
                | SwitchPhase::Cancelled
        )
    }
    pub fn references(&self) -> Vec<Id> {
        BTreeSet::from([
            self.source.generation,
            self.target.generation,
            self.original_target.generation,
        ])
        .into_iter()
        .collect()
    }
    pub fn mask(&self) -> u16 {
        self.source_marks.iter().fold(0, |m, r| m | (1 << r.slot))
    }
    pub fn status(&self) -> OperationStatus {
        let mut observations = ObservationStatus::default();
        observations.observe_credentials(if self.phase == SwitchPhase::Restored {
            CredentialAcceptance::Unknown
        } else {
            self.acceptance
        });
        observations.observe_launch(self.launch);
        if matches!(
            self.phase,
            SwitchPhase::Recovery | SwitchPhase::Restoring | SwitchPhase::Conflict
        ) {
            observations.require_recovery(self.phase == SwitchPhase::Conflict);
        }
        let installed = if self.phase == SwitchPhase::Restored {
            Some(self.source.profile)
        } else if self.write_done == self.mask()
            && self.restore_intent == 0
            && self.phase != SwitchPhase::Conflict
        {
            Some(self.target.profile)
        } else {
            None
        };
        OperationStatus {
            operation_id: self.id,
            phase: self.phase,
            installed_profile: installed.and_then(|id| crate::ProfileId::from_bytes(id).ok()),
            observations,
            primary_error: self.failure.map(Failure::code),
            restoration_error: self.restoration_failure.map(Failure::code),
        }
    }
    pub fn validate(&self, r: &Registry) -> Result<(), StorageError> {
        use SwitchPhase::*;
        if self.id == [0; 16]
            || self.binding == [0; 32]
            || self.created > self.updated
            || self.source.profile == self.target.profile
            || self.target.profile != self.original_target.profile
            || self.evidence.len() > MAX_EVIDENCE
        {
            return Err(StorageError::Corrupt);
        }
        let get = |reference: GenerationRef| {
            r.generations
                .iter()
                .find(|g| GenerationRef::from(g) == reference)
                .ok_or(StorageError::Corrupt)
        };
        let source = get(self.source)?;
        let target = get(self.target)?;
        let original = get(self.original_target)?;
        let shape = |g: &Generation| {
            g.rules
                .iter()
                .map(|r| (r.slot, r.json, r.required))
                .collect::<Vec<_>>()
        };
        if source.schema != target.schema
            || target.schema != original.schema
            || shape(source) != shape(target)
            || shape(target) != shape(original)
        {
            return Err(StorageError::Corrupt);
        }
        for (marks, g) in [(&self.source_marks, source), (&self.target_marks, target)] {
            if marks.len() != g.rules.len()
                || marks
                    .iter()
                    .zip(&g.rules)
                    .any(|(m, r)| m.slot != r.slot || m.present != r.blob.is_some())
            {
                return Err(StorageError::Corrupt);
            }
        }
        let mask = self.mask();
        let prefix = |bits: u16| {
            let mut gap = false;
            for mark in &self.source_marks {
                if bits & (1 << mark.slot) == 0 {
                    gap = true;
                } else if gap {
                    return false;
                }
            }
            true
        };
        for bits in [
            self.staging_intent,
            self.staging_done,
            self.write_intent,
            self.write_done,
            self.restore_intent,
            self.restore_done,
        ] {
            if bits & !mask != 0 || !prefix(bits) {
                return Err(StorageError::Corrupt);
            }
        }
        if self.staging_done & !self.staging_intent != 0
            || self.write_done & !self.write_intent != 0
            || self.restore_done & !self.restore_intent != 0
            || (self.write_intent != 0 && self.staging_done != mask)
            || (self.restore_intent != 0 && self.write_intent == 0)
            || (self.staging_intent ^ self.staging_done).count_ones() > 1
            || (self.write_intent ^ self.write_done).count_ones() > 1
            || (self.restore_intent ^ self.restore_done).count_ones() > 1
            || (!self.source_saved && (self.staging_intent != 0 || self.write_intent != 0))
            || (self.helper != Helper::Never && (!self.online || self.write_done != mask))
            || (self.committed
                && (self.write_done != mask
                    || self.helper == Helper::MayRun
                    || self.acceptance == CredentialAcceptance::Rejected
                    || self.restore_intent != 0))
            || (self.launch != DesktopLaunch::NotRequested && !self.committed)
            || (self.terminal() && (!self.cleaned || self.helper == Helper::MayRun))
        {
            return Err(StorageError::Corrupt);
        }
        if matches!(self.phase, Requested | Locked | Quiescent)
            && (self.source_saved || self.staging_intent != 0)
            || matches!(
                self.phase,
                SourceSaved
                    | TargetStaged
                    | Applying
                    | TargetInstalled
                    | HelperRunning
                    | HelperReaping
                    | Observed
                    | Committed
                    | RelaunchRequested
                    | AwaitingConfirmation
                    | Finished
                    | Restoring
                    | Restored
            ) && !self.source_saved
            || matches!(self.phase, TargetStaged | Applying) && self.staging_done != mask
            || matches!(
                self.phase,
                TargetInstalled
                    | HelperRunning
                    | HelperReaping
                    | Observed
                    | Committed
                    | RelaunchRequested
                    | AwaitingConfirmation
                    | Finished
            ) && self.write_done != mask
            || matches!(
                self.phase,
                Committed | RelaunchRequested | AwaitingConfirmation | Finished
            ) && !self.committed
            || matches!(self.phase, HelperRunning | HelperReaping) && self.helper != Helper::MayRun
            || self.phase == Restored && (self.restore_done != mask || self.committed)
            || self.phase == Cancelled && (self.write_intent != 0 || self.committed)
            || self.phase == AwaitingConfirmation && self.launch != DesktopLaunch::Opened
        {
            return Err(StorageError::Corrupt);
        }
        let hold = r
            .holds
            .iter()
            .find(|h| h.id == self.id)
            .ok_or(StorageError::Corrupt)?;
        if hold.generations != self.references() {
            return Err(StorageError::Corrupt);
        }
        let mut ids = BTreeSet::new();
        for e in &self.evidence {
            if (e.kind == EvidenceKind::Staging && r.format < 3)
                || e.id == [0; 16]
                || !ids.insert(e.id)
                || e.resources.is_empty()
                || e.resources.len() > 16
            {
                return Err(StorageError::Corrupt);
            }
            let mut slots = BTreeSet::new();
            let mut total = 0;
            for resource in &e.resources {
                if resource.slot >= 16
                    || (e.kind == EvidenceKind::Staging
                        && (resource.blob.is_none()
                            || (self.staging_intent | self.restore_intent) & (1 << resource.slot)
                                == 0))
                    || !slots.insert(resource.slot)
                    || resource.required
                    || resource.json
                {
                    return Err(StorageError::Corrupt);
                }
                if let Some(b) = &resource.blob {
                    if b.key != Key::resource(self.id, e.id, resource.slot)
                        || b.size < 66
                        || b.size as usize > 1024 * 1024 + 66
                    {
                        return Err(StorageError::Corrupt);
                    }
                    total += b.size as usize - 66;
                }
            }
            if total > 4 * 1024 * 1024 {
                return Err(StorageError::InputLimit);
            }
        }
        Ok(())
    }
}
