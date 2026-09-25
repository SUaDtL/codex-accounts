//! Explicit staging-only repair; never part of normal cleanup or auto startup.
#![forbid(unsafe_code)]
use super::*;

/// The adapter retains exclusive, validated stage handles across the callback.
/// It may remove ONLY the exact registered objects after the callback succeeds.
/// No path or callback crosses the public library/renderer boundary.
pub(crate) trait StagingRepair: Effects {
    fn preserve_then_remove_staging(
        &mut self,
        operation: Id,
        registered: u16,
        preserve: &mut dyn FnMut(Vec<Resource>) -> Result<(), SwitchError>,
    ) -> Result<(), SwitchError>;
}
impl<D: Files> Storage<D> {
    pub(crate) fn repair_registered_staging(
        &mut self,
        root: &RootKey,
        effects: &mut impl StagingRepair,
        id: Id,
    ) -> Result<(), SwitchError> {
        let mut j = self.journal(id)?;
        // Do not repair a running operation, launched app, uncaptured helper
        // generation, terminal operation or a caller-invented registration.
        if j.terminal()
            || j.committed
            || j.cleaned
            || !j.source_saved
            || !matches!(
                j.phase,
                SwitchPhase::Conflict | SwitchPhase::Recovery | SwitchPhase::Restoring
            )
            || j.helper == Helper::MayRun
            || j.staging_intent | j.restore_intent == 0
        {
            return Err(SwitchError::InvalidTransition);
        }
        self.verify_registry(root, &self.registry)?;
        for check in [
            Check::Confirm,
            Check::Lock,
            Check::Recovery,
            Check::Quiesce,
            Check::Write,
        ] {
            effects.guard(check, &j.binding)?;
        }
        self.mixed_live(root, &j, &effects.snapshot()?)?;
        // Durable cancellation intent keeps the operation blocked after a crash
        // anywhere in archival/deletion. Repair never finishes or replays a switch.
        j.cancel = true;
        j.failure.get_or_insert(Failure::Interrupted);
        j.phase = SwitchPhase::Recovery;
        self.save_journal(root, j.clone())?;
        let registered = j.staging_intent | j.restore_intent;
        let result = effects.preserve_then_remove_staging(id, registered, &mut |resources| {
            if resources.is_empty() {
                return Err(StorageError::InvalidData.into());
            }
            // Metadata and bytes are one existing encrypted-store transaction.
            // A successful return includes authenticated readback of every blob.
            let current = self.journal(id)?;
            self.preserve_evidence(
                root,
                current,
                Snapshot {
                    identity: None,
                    schema: 0,
                    resources,
                },
                EvidenceKind::Staging,
            )?;
            self.verify_registry(root, &self.registry)?;
            Ok(())
        });
        if let Err(SwitchError::Refused(f)) = result {
            let current = self.journal(id)?;
            return self.fail_switch(root, effects, current, f, true);
        }
        result?;
        effects.guard(Check::Recovery, &j.binding)?;
        self.mixed_live(root, &j, &effects.snapshot()?)?;
        // Still Recovery/SwitchPending. A separate explicit Restore choice uses
        // the original write-ahead path and newest authenticated generations.
        Ok(())
    }
}
