//! Production operations expose account IDs and fixed outcomes, never credentials,
//! paths, executable choices, or caller-supplied effects.
use super::{runtime_context::RuntimeContext, target::Target};
use crate::{
    auth_capture,
    engine::coordinator::{Check, Choice, Effects, Request, Snapshot, StagingRepair, SwitchError},
    journal::{Failure, GenerationRef},
    OperationError, OperationStatus, ProfileId, ProfileText, StorageError, SwitchCancellation,
    SwitchPhase, Vault,
};
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};
use codex_accounts_vault::{CredentialSet, Resource, ResourceId};

impl From<Failure> for OperationError {
    fn from(value: Failure) -> Self {
        Self(value.code())
    }
}

impl Vault {
    /// Explicit normal Desktop launch through the current trusted installation.
    /// No path, credential arguments, helper, or account-identity claim is exposed.
    pub fn open_codex(&self) -> Result<(), OperationError> {
        self.storage.ensure_operation_idle()?;
        RuntimeContext::discover()?.launch_desktop()?;
        Ok(())
    }
}
impl From<SwitchError> for OperationError {
    fn from(value: SwitchError) -> Self {
        match value {
            SwitchError::Storage(error) => error.into(),
            SwitchError::Refused(error) => error.into(),
            SwitchError::InvalidTransition => Self("E_RECOVERY_CHOICE_UNAVAILABLE"),
            SwitchError::AlreadyCommitted => Self("E_ALREADY_COMMITTED"),
        }
    }
}
fn failure(value: StorageError) -> Failure {
    match value {
        StorageError::Busy => Failure::Busy,
        StorageError::ExternalChange | StorageError::UnsafePath | StorageError::AccessDenied => {
            Failure::ExternalChange
        }
        StorageError::InvalidData | StorageError::Corrupt | StorageError::InputLimit => {
            Failure::InvalidData
        }
        _ => Failure::Write,
    }
}

struct NativeEffects {
    // Context drops first, so no retained helper outlives the target's home lock.
    context: RuntimeContext,
    target: Target,
}
impl NativeEffects {
    fn open() -> Result<Self, OperationError> {
        let mut context = RuntimeContext::discover()?;
        context.quiesce()?;
        let target = Target::open_file_backend(context.home())?;
        // Selection reads launch no helper. A caller must preserve the outgoing
        // snapshot before official startup can refresh or invalidate auth data.
        context.check_selection()?;
        context.revalidate()?;
        context.quiesce()?;
        target.validate()?;
        Ok(Self { context, target })
    }
    fn capture(&self) -> Result<crate::Capture, OperationError> {
        self.context.revalidate()?;
        self.context.quiesce()?;
        let mut bytes = self
            .target
            .read(0)?
            .ok_or(OperationError("E_LOGIN_REQUIRED"))?;
        auth_capture::capture(std::mem::take(&mut *bytes))
            .map_err(|_| OperationError("E_AUTH_FORMAT_UNSUPPORTED"))
    }
}
impl Drop for NativeEffects {
    fn drop(&mut self) {
        let _ = self.context.stop_observer_and_wait();
    }
}
impl Effects for NativeEffects {
    fn confirm(&mut self, request: &Request) -> Result<(), Failure> {
        // The public operation is an explicit user action. There is no renderer
        // consent token capable of overriding binding, policy, or writer checks.
        self.guard(Check::Confirm, &request.binding)
    }
    fn guard(&mut self, check: Check, binding: &[u8; 32]) -> Result<(), Failure> {
        if self.context.binding() != *binding {
            return Err(Failure::Binding);
        }
        self.context.revalidate()?;
        self.target.validate().map_err(failure)?;
        if !matches!(check, Check::Confirm | Check::Lock) {
            self.context.quiesce()?;
        }
        Ok(())
    }
    fn snapshot(&mut self) -> Result<Snapshot, Failure> {
        let resources = self.target.snapshot().map_err(failure)?;
        let bytes = resources
            .iter()
            .find(|r| r.id() == ResourceId::new(0).expect("fixed slot"))
            .and_then(|r| r.as_bytes());
        if let Some(bytes) = bytes {
            if let Ok(snapshot) = auth_capture::snapshot(bytes.to_vec()) {
                return Ok(snapshot);
            }
        }
        // Retain malformed/absent data for the coordinator's encrypted conflict
        // evidence. It cannot match a valid generation's account identity.
        Ok(Snapshot {
            identity: None,
            schema: 1,
            resources,
        })
    }
    fn stage(
        &mut self,
        operation: [u8; 16],
        slot: u8,
        value: Option<&[u8]>,
    ) -> Result<(), Failure> {
        self.target.stage(operation, slot, value).map_err(failure)
    }
    fn replace(
        &mut self,
        operation: [u8; 16],
        slot: u8,
        expected: Option<&[u8]>,
        value: Option<&[u8]>,
    ) -> Result<(), Failure> {
        self.target
            .replace(operation, slot, expected, value)
            .map_err(failure)
    }
    fn cleanup(
        &mut self,
        operation: [u8; 16],
        registered: u16,
        allowed: &[CredentialSet],
    ) -> Result<(), Failure> {
        self.context.quiesce()?;
        self.target
            .cleanup(operation, registered, allowed)
            .map_err(failure)
    }
    fn start_helper(&mut self, _: [u8; 16]) -> Result<(), Failure> {
        self.context.start_observer()
    }
    fn observe(&mut self, _: [u8; 16]) -> Result<CredentialAcceptance, Failure> {
        self.context.observe_account()
    }
    fn reap_helper(&mut self, _: [u8; 16]) -> Result<(), Failure> {
        self.context.stop_observer_and_wait()
    }
    fn launch(&mut self, _: [u8; 16]) -> Result<DesktopLaunch, Failure> {
        self.context.launch_desktop()
    }
}
impl StagingRepair for NativeEffects {
    fn preserve_then_remove_staging(
        &mut self,
        operation: [u8; 16],
        registered: u16,
        preserve: &mut dyn FnMut(Vec<Resource>) -> Result<(), SwitchError>,
    ) -> Result<(), SwitchError> {
        self.target
            .preserve_then_remove(operation, registered, preserve)
    }
}

impl Vault {
    /// Called only after a matching official staging login, proven helper exit,
    /// and scratch cleanup. Its fresh capture has not yet been published.
    pub(super) fn install_reauthenticated_current(
        &mut self,
        profile: ProfileId,
        fresh: crate::Capture,
        cancel: crate::LoginCancellation,
    ) -> Result<OperationStatus, OperationError> {
        self.storage.ensure_operation_idle()?;
        let mut effects = NativeEffects::open()?;
        let control = SwitchCancellation::from_login(cancel);
        if control.cancelled() {
            return Err(OperationError("E_CANCELLED"));
        }
        effects.context.switch_cancellation(control.clone());
        // Browser sign-in may take minutes. Recheck current policy before any
        // installation, preserving live auth before that helper can maintain it.
        // The fresh login is still unpublished, so this cannot downgrade it.
        let source = effects.capture()?;
        if self.storage.current_reference(&source.identity)?.profile != profile.to_bytes()
            || source.identity != fresh.identity
        {
            return Err(OperationError("E_EXTERNAL_CHANGE"));
        }
        self.storage.save_current(&self.root, source, None)?;
        let checked = effects.context.check_configuration();
        if checked == Err(Failure::HelperStuck) {
            return Err(Failure::HelperStuck.into());
        }
        let checked = if control.cancelled() {
            Err(Failure::Cancelled)
        } else {
            checked
        };
        self.preserve_current_after_helper(&mut effects, profile, checked)?;
        let binding = effects.context.binding();
        let id = self.storage.begin_reauthentication(
            &self.root,
            &mut effects,
            profile,
            fresh,
            binding,
        )?;
        let result = self.run_operation_cancellable(&mut effects, id, Some(&control));
        // Even a failed initialization can own a live writer. Cancellation or a
        // protocol error never substitutes for the native exit receipt.
        effects.context.stop_observer_and_wait()?;
        let journal = self.storage.journal(id)?;
        if control.cancelled() && !journal.committed && !journal.terminal() && !journal.cancel {
            self.storage.cancel_switch(&self.root, &mut effects, id)?;
        }
        match result {
            Ok(status) => Ok(status),
            Err(SwitchError::Refused(_)) => self.operation_status(id),
            Err(error) => Err(error.into()),
        }
    }
    fn preserve_current_after_helper<T>(
        &mut self,
        effects: &mut NativeEffects,
        expected: ProfileId,
        primary: Result<T, Failure>,
    ) -> Result<T, OperationError> {
        // Failed reaping retains the observer in RuntimeContext. Never read live
        // auth bytes or authorize a later mutation without this successful receipt.
        effects.context.stop_observer_and_wait()?;
        let preserved: Result<(), OperationError> = (|| {
            let refreshed = effects.capture()?;
            let reference = self
                .storage
                .current_reference(&refreshed.identity)
                .map_err(|error| {
                    if error == StorageError::Missing {
                        OperationError("E_EXTERNAL_CHANGE")
                    } else {
                        error.into()
                    }
                })?;
            if reference.profile != expected.to_bytes() {
                return Err(OperationError("E_EXTERNAL_CHANGE"));
            }
            self.storage.save_current(&self.root, refreshed, None)?;
            Ok(())
        })();
        // Preserve the original protocol/policy failure if the subsequent
        // snapshot cannot be saved. The pre-helper encrypted source still exists.
        let value = primary.map_err(OperationError::from)?;
        preserved?;
        Ok(value)
    }

    /// Save the current native file-backend account under a user-provided label.
    /// Repeated captures of one account update the same encrypted profile.
    pub fn capture_current(&mut self, label: ProfileText) -> Result<ProfileId, OperationError> {
        self.storage.ensure_operation_idle()?;
        let mut effects = NativeEffects::open()?;
        let capture = effects.capture()?;
        let profile = self
            .storage
            .save_current(&self.root, capture, Some(label))
            .map_err(OperationError::from)?;
        let checked = effects.context.check_configuration();
        self.preserve_current_after_helper(&mut effects, profile, checked)?;
        Ok(profile)
    }

    /// Cold handoff with automatic source restoration after a precommit refusal.
    /// Callers must inspect the returned phase; a restored outcome is not success.
    pub fn switch_to(&mut self, profile: ProfileId) -> Result<OperationStatus, OperationError> {
        self.switch_to_cancellable(profile, SwitchCancellation::new())
    }

    /// Cancellation before commit reconciles and restores through the persisted
    /// transaction. Completion, including cleanup failure, is the only busy-state
    /// release; requesting cancellation alone does not authorize another action.
    pub fn switch_to_cancellable(
        &mut self,
        profile: ProfileId,
        control: SwitchCancellation,
    ) -> Result<OperationStatus, OperationError> {
        self.storage.ensure_operation_idle()?;
        if control.cancelled() {
            return Err(OperationError("E_CANCELLED"));
        }
        let mut effects = NativeEffects::open()?;
        effects.context.switch_cancellation(control.clone());
        let capture = effects.capture()?;
        let source = self
            .storage
            .current_reference(&capture.identity)
            .map_err(|error| {
                if error == StorageError::Missing {
                    OperationError("E_CAPTURE_CURRENT_FIRST")
                } else {
                    error.into()
                }
            })?;
        if source.profile == profile.to_bytes() {
            return Err(OperationError("E_ALREADY_CURRENT"));
        }
        let target = GenerationRef {
            profile: profile.to_bytes(),
            generation: self.storage.latest(profile)?.to_bytes(),
        };
        let (source_label, target_label) = self.operation_labels(source.profile, target.profile)?;
        super::consent::switch(&source_label, &target_label)?;
        if control.cancelled() {
            return Err(OperationError("E_CANCELLED"));
        }
        // Save source token refreshes and the source association atomically before
        // beginning the journal. The coordinator snapshots again before staging.
        let source_profile = self.storage.save_current(&self.root, capture, None)?;
        let checked = if control.cancelled() {
            Err(Failure::Cancelled)
        } else {
            effects.context.check_configuration()
        };
        let checked = if control.cancelled() {
            Err(Failure::Cancelled)
        } else {
            checked
        };
        self.preserve_current_after_helper(&mut effects, source_profile, checked)?;
        let source = GenerationRef {
            profile: source_profile.to_bytes(),
            generation: self.storage.latest(source_profile)?.to_bytes(),
        };
        let request = Request {
            source,
            target,
            binding: effects.context.binding(),
            online: true,
        };
        let id = self
            .storage
            .begin_switch(&self.root, &mut effects, request)?;
        match self.run_operation_cancellable(&mut effects, id, Some(&control)) {
            Ok(status) => Ok(status),
            Err(SwitchError::Refused(_)) => {
                // First reap the exact owned helper even if its initialization or
                // observation failed. Recovery never races a still-live writer.
                effects.context.stop_observer_and_wait()?;
                let mut journal = self.storage.journal(id)?;
                if control.cancelled()
                    && !journal.committed
                    && !journal.terminal()
                    && !journal.cancel
                {
                    self.storage.cancel_switch(&self.root, &mut effects, id)?;
                    journal = self.storage.journal(id)?;
                }
                if journal.terminal() {
                    return Ok(journal.status());
                }
                if !journal.committed
                    && self
                        .storage
                        .recover_switch(&self.root, &mut effects, id, Choice::Restore)
                        .is_ok()
                {
                    let _ = self.run_operation(&mut effects, id);
                }
                self.operation_status(id)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Recover an existing authenticated operation. Restoration launches no
    /// helper and never overwrites live data that fails the journal comparison.
    pub fn recover_switch(
        &mut self,
        operation: [u8; 16],
        finish: bool,
    ) -> Result<OperationStatus, OperationError> {
        let journal = self.storage.journal(operation)?;
        let (source_label, target_label) =
            self.operation_labels(journal.source.profile, journal.target.profile)?;
        super::consent::recover(&source_label, &target_label, finish)?;
        let mut effects = NativeEffects::open()?;
        let admission =
            if finish && journal.reauthentication() && !journal.committed && !journal.terminal() {
                self.storage
                    .check_reauthentication_recovery(&self.root, &mut effects, operation)
            } else {
                Ok(())
            };
        let result = admission
            .and_then(|()| {
                self.storage.recover_switch(
                    &self.root,
                    &mut effects,
                    operation,
                    if finish {
                        Choice::Finish
                    } else {
                        Choice::Restore
                    },
                )
            })
            .and_then(|()| self.run_operation(&mut effects, operation));
        match result {
            Ok(status) => Ok(status),
            Err(SwitchError::Refused(_)) => {
                effects.context.stop_observer_and_wait()?;
                if !finish {
                    // The user's explicit Restore action also permits recovery
                    // of torn app-owned staging objects. The core first compares
                    // live bytes, archives every exact registered stage under
                    // encryption, and only then allows retained-handle removal.
                    // Unknown stage names and changed live bytes remain refused.
                    match self.storage.repair_registered_staging(
                        &self.root,
                        &mut effects,
                        operation,
                    ) {
                        Ok(()) => {
                            self.storage.recover_switch(
                                &self.root,
                                &mut effects,
                                operation,
                                Choice::Restore,
                            )?;
                            match self.run_operation(&mut effects, operation) {
                                Ok(status) => return Ok(status),
                                Err(SwitchError::Refused(_)) => {}
                                Err(error) => return Err(error.into()),
                            }
                        }
                        Err(SwitchError::Storage(error)) => return Err(error.into()),
                        Err(_) => {}
                    }
                }
                self.operation_status(operation)
            }
            Err(error) => Err(error.into()),
        }
    }
    fn operation_status(&self, id: [u8; 16]) -> Result<OperationStatus, OperationError> {
        Ok(self.storage.journal(id)?.status())
    }
    fn operation_labels(
        &self,
        source: [u8; 16],
        target: [u8; 16],
    ) -> Result<(String, String), OperationError> {
        let profiles = self.storage.profiles()?;
        let label = |id| {
            profiles
                .iter()
                .find(|profile| profile.id.to_bytes() == id)
                .map(|profile| profile.label.clone())
                .ok_or_else(|| OperationError::from(StorageError::Missing))
        };
        Ok((label(source)?, label(target)?))
    }
    fn run_operation(
        &mut self,
        effects: &mut NativeEffects,
        id: [u8; 16],
    ) -> Result<OperationStatus, SwitchError> {
        self.run_operation_cancellable(effects, id, None)
    }
    fn run_operation_cancellable(
        &mut self,
        effects: &mut NativeEffects,
        id: [u8; 16],
        control: Option<&SwitchCancellation>,
    ) -> Result<OperationStatus, SwitchError> {
        // The fixed single-file resource needs fewer than 32 transitions. The
        // bound also prevents an accidental nonprogress loop from freezing UI.
        for _ in 0..128 {
            let journal = self.storage.journal(id)?;
            if journal.terminal() {
                return Ok(journal.status());
            }
            if !journal.committed
                && !journal.cancel
                && control.is_some_and(SwitchCancellation::cancelled)
            {
                // A pipe cancellation only wakes the request. Reaping and the
                // coordinator's post-helper capture remain mandatory recovery.
                effects.context.stop_observer_and_wait()?;
                self.storage.cancel_switch(&self.root, effects, id)?;
                let cancelled = self.storage.journal(id)?;
                if cancelled.terminal() {
                    return Ok(cancelled.status());
                }
                return Err(SwitchError::Refused(Failure::Cancelled));
            }
            if matches!(journal.phase, SwitchPhase::Recovery | SwitchPhase::Conflict) {
                return Err(SwitchError::Refused(
                    journal.failure.unwrap_or(Failure::Interrupted),
                ));
            }
            self.storage.advance_switch(&self.root, effects, id)?;
        }
        Err(SwitchError::Refused(Failure::Interrupted))
    }
}
