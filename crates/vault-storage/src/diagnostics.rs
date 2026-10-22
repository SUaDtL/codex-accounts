//! Deliberate allowlist projection: never serialize a registry or operation status.
#![forbid(unsafe_code)]
use super::{Files, Storage};
use crate::{journal::Journal, StorageError, SwitchPhase};
use serde::Serialize;

const MAX_EVENTS: usize = 500;
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
struct Diagnostics {
    schema_version: u32,
    build_version: &'static str,
    adapter_id: &'static str,
    recovery_code: Option<&'static str>,
    events: Vec<Event>,
}

#[derive(Serialize)]
struct Event {
    operation_id: String,
    phase: &'static str,
    primary_error: Option<&'static str>,
    restoration_error: Option<&'static str>,
    created_at: u64,
    updated_at: u64,
}

impl<D: Files> Storage<D> {
    pub(crate) fn sanitized_diagnostics(&self) -> Result<Vec<u8>, StorageError> {
        match self.ready() {
            Ok(()) => project(&self.registry.journals, false),
            // A retained vault may require control repair. Export only the fixed
            // outcome in this case; do not inspect its questionable registry.
            Err(StorageError::RecoveryRequired) => project(&[], true),
            Err(error) => Err(error),
        }
    }
}

fn project(journals: &[Journal], recovery_required: bool) -> Result<Vec<u8>, StorageError> {
    let mut newest: Vec<_> = journals.iter().collect();
    newest.sort_unstable_by_key(|journal| {
        std::cmp::Reverse((journal.updated, journal.created, journal.id))
    });
    let events = newest
        .into_iter()
        .take(MAX_EVENTS)
        .map(|journal| Event {
            // Journal IDs are random and unrelated to profile IDs or identities.
            operation_id: journal
                .id
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            phase: phase(journal.phase),
            primary_error: journal.failure.map(|failure| failure.code()),
            restoration_error: journal.restoration_failure.map(|failure| failure.code()),
            created_at: journal.created,
            updated_at: journal.updated,
        })
        .collect();
    let bytes = serde_json::to_vec_pretty(&Diagnostics {
        schema_version: 1,
        build_version: env!("CARGO_PKG_VERSION"),
        adapter_id: "windows-native-file-v1",
        recovery_code: recovery_required.then_some("E_RECOVERY_REQUIRED"),
        events,
    })
    .map_err(|_| StorageError::InvalidData)?;
    if bytes.len() > MAX_BYTES {
        return Err(StorageError::InputLimit);
    }
    Ok(bytes)
}

fn phase(phase: SwitchPhase) -> &'static str {
    use SwitchPhase::*;
    match phase {
        Requested => "requested",
        Locked => "locked",
        Quiescent => "quiescent",
        SourceSaved => "source_saved",
        TargetStaged => "target_staged",
        Applying => "applying",
        TargetInstalled => "target_installed",
        HelperRunning => "helper_running",
        HelperReaping => "helper_reaping",
        Observed => "observed",
        Committed => "committed",
        RelaunchRequested => "relaunch_requested",
        AwaitingConfirmation => "awaiting_confirmation",
        Finished => "finished",
        Recovery => "recovery",
        Restoring => "restoring",
        Restored => "restored",
        Cancelled => "cancelled",
        Conflict => "conflict",
        NeedsLogin => "needs_login",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{Failure, GenerationRef, Helper, Mark};
    use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};

    #[test]
    fn sanitized_projection_is_bounded_and_excludes_private_record_fields() {
        let private = GenerationRef {
            profile: [0xab; 16],
            generation: [0xcd; 16],
        };
        let mut journals = Vec::new();
        for timestamp in 1..=502_u64 {
            journals.push(Journal {
                id: [0x42; 16],
                binding: [0xee; 32],
                created: timestamp,
                updated: timestamp,
                source: private,
                target: private,
                original_target: private,
                phase: SwitchPhase::Recovery,
                source_saved: true,
                online: true,
                staging_intent: 1,
                staging_done: 1,
                write_intent: 1,
                write_done: 1,
                restore_intent: 0,
                restore_done: 0,
                helper: Helper::Reaped,
                cancel: false,
                committed: false,
                cleaned: false,
                failure: Some(Failure::Policy),
                restoration_failure: Some(Failure::Cleanup),
                acceptance: CredentialAcceptance::Unknown,
                launch: DesktopLaunch::NotRequested,
                source_marks: vec![Mark {
                    slot: 0,
                    present: true,
                    tag: [0xef; 32],
                }],
                target_marks: vec![],
                evidence: vec![],
            });
        }
        let bytes = project(&journals, false).unwrap();
        assert!(bytes.len() <= MAX_BYTES);
        let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let object = document.as_object().unwrap();
        assert_eq!(
            object.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "adapter_id",
                "build_version",
                "events",
                "recovery_code",
                "schema_version"
            ]
        );
        let events = document["events"].as_array().unwrap();
        assert_eq!(events.len(), 500);
        assert_eq!(events.first().unwrap()["updated_at"], 502);
        assert_eq!(events.last().unwrap()["updated_at"], 3);
        for event in events {
            assert_eq!(
                event
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                [
                    "created_at",
                    "operation_id",
                    "phase",
                    "primary_error",
                    "restoration_error",
                    "updated_at"
                ]
            );
            assert_eq!(event["operation_id"], "42424242424242424242424242424242");
            assert_eq!(event["phase"], "recovery");
            assert_eq!(event["primary_error"], "E_POLICY_DENIED");
            assert_eq!(event["restoration_error"], "E_RECOVERY_REQUIRED");
        }
        assert!(document["recovery_code"].is_null());
        let recovery: serde_json::Value =
            serde_json::from_slice(&project(&[], true).unwrap()).unwrap();
        assert_eq!(recovery["recovery_code"], "E_RECOVERY_REQUIRED");
        assert!(recovery["events"].as_array().unwrap().is_empty());
    }
}
