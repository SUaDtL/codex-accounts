//! The vault's native location lock is thread-affine. Keep its entire lifetime
//! on one worker and exchange metadata only with the webview command handlers.
use crate::metadata::{self, Operation, Profile, ProfileList, VaultState};
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};
use codex_accounts_runtime::{Observed, RateLimitWindow};
use codex_accounts_vault_storage::{
    LoginCancellation, OperationStatus, ProfileId, ProfileText, Recovery, StorageError,
    SwitchCancellation, SwitchPhase, Vault,
};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;

pub enum Intent {
    List,
    Open,
    Unlock(zeroize::Zeroizing<String>),
    Create,
    Close,
    Harden(zeroize::Zeroizing<String>),
    Unharden,
    OpenCodex,
    Capture(String),
    Add(String, LoginCancellation),
    Reauthenticate(String, LoginCancellation),
    Switch(String, SwitchCancellation),
    Recover(String, bool),
    RecoverVault,
    Remove(String),
    Rename(String, String),
    RefreshUsage(String),
    ExportDiagnostics,
}

type Reply = Result<ProfileList, &'static str>;
struct Request {
    intent: Intent,
    reply: mpsc::Sender<Reply>,
}

pub struct Worker {
    sender: Option<SyncSender<Request>>,
    thread: Option<JoinHandle<()>>,
    busy: Arc<AtomicBool>,
    active: Arc<Mutex<Option<ActiveControl>>>,
    close_requested: AtomicBool,
    hardened: Arc<AtomicBool>,
}

enum ActiveControl {
    Login(LoginCancellation),
    Switch(SwitchCancellation),
}

impl ActiveControl {
    fn cancel(&self) {
        match self {
            Self::Login(control) => control.cancel(),
            Self::Switch(control) => control.cancel(),
        }
    }
}

struct RequestGuard {
    busy: Arc<AtomicBool>,
    active: Arc<Mutex<Option<ActiveControl>>>,
}
impl Drop for RequestGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
        self.busy.store(false, Ordering::Release);
    }
}

impl Worker {
    pub fn start() -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Request>(1);
        let hardened = Arc::new(AtomicBool::new(false));
        let worker_hardened = hardened.clone();
        let thread = std::thread::Builder::new()
            .name("account-vault".to_owned())
            .spawn(move || {
                let mut vault: Option<Vault> = None;
                let mut health: Option<metadata::Health> = None;
                while let Ok(request) = receiver.recv() {
                    let response = execute(&mut vault, &mut health, request.intent);
                    worker_hardened.store(
                        vault.as_ref().is_some_and(|vault| {
                            vault.protection()
                                == codex_accounts_vault_storage::VaultProtection::Passphrase
                        }),
                        Ordering::Release,
                    );
                    let _ = request.reply.send(response);
                }
                // Vault and its native lock are dropped on this owning thread.
            })?;
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
            busy: Arc::default(),
            active: Arc::default(),
            close_requested: AtomicBool::new(false),
            hardened,
        })
    }

    pub async fn request(&self, intent: Intent) -> Reply {
        if self.close_requested.load(Ordering::Acquire)
            && !matches!(intent, Intent::List | Intent::Close)
        {
            return Err("vault_closing");
        }
        self.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| "vault_busy")?;
        let guard = RequestGuard {
            busy: self.busy.clone(),
            active: self.active.clone(),
        };
        let control = match &intent {
            Intent::Add(_, control) | Intent::Reauthenticate(_, control) => {
                Some(ActiveControl::Login(control.clone()))
            }
            Intent::Switch(_, control) => Some(ActiveControl::Switch(control.clone())),
            _ => None,
        };
        if control.is_some() {
            *self.active.lock().map_err(|_| "vault_unavailable")? = control;
        }
        // A close event may race with admission before the control is installed.
        // Refuse before queueing any new effects, rather than losing that cancel.
        if self.close_requested.load(Ordering::Acquire)
            && !matches!(intent, Intent::List | Intent::Close)
        {
            return Err("vault_closing");
        }
        let (reply, receiver) = mpsc::channel();
        self.sender
            .as_ref()
            .ok_or("vault_unavailable")?
            .try_send(Request { intent, reply })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => "vault_busy",
                mpsc::TrySendError::Disconnected(_) => "vault_unavailable",
            })?;
        tauri::async_runtime::spawn_blocking(move || {
            // Keep admission occupied even if the IPC future is cancelled.
            // No timeout/retry may abandon a live native operation or grow tasks.
            let _guard = guard;
            receiver.recv()
        })
        .await
        .map_err(|_| "vault_unavailable")?
        .map_err(|_| "vault_unavailable")?
    }

    pub async fn add_account(&self, label: String) -> Reply {
        self.request(Intent::Add(label, LoginCancellation::new()))
            .await
    }

    pub async fn reauthenticate(&self, profile: String) -> Reply {
        self.request(Intent::Reauthenticate(profile, LoginCancellation::new()))
            .await
    }

    pub async fn switch_profile(&self, profile: String) -> Reply {
        self.request(Intent::Switch(profile, SwitchCancellation::new()))
            .await
    }

    // Cancellation must not queue behind the blocking login on the vault thread.
    pub fn cancel_login(&self) -> Result<(), &'static str> {
        let active = self.active.lock().map_err(|_| "vault_unavailable")?;
        let Some(ActiveControl::Login(control)) = active.as_ref() else {
            return Err("login_not_running");
        };
        control.cancel();
        Ok(())
    }

    pub fn cancel_switch(&self) -> Result<(), &'static str> {
        let active = self.active.lock().map_err(|_| "vault_unavailable")?;
        let Some(ActiveControl::Switch(control)) = active.as_ref() else {
            return Err("switch_not_running");
        };
        control.cancel();
        Ok(())
    }

    pub fn action_status(&self) -> Result<&'static str, &'static str> {
        if self.close_requested.load(Ordering::Acquire) {
            return Ok("closing");
        }
        let active = self.active.lock().map_err(|_| "vault_unavailable")?;
        Ok(match active.as_ref() {
            Some(ActiveControl::Login(control)) => control.state().code(),
            Some(ActiveControl::Switch(_)) => "switching",
            None => "idle",
        })
    }

    pub fn locks_on_hide(&self) -> bool {
        self.hardened.load(Ordering::Acquire)
    }

    /// Keep the UI alive while the owned operation is reaped and its files are
    /// reconciled. Drop alone may be delayed by an outstanding command future.
    pub fn cancel_for_close(&self) -> bool {
        // Also blocks a command future which has not yet reached admission.
        self.close_requested.store(true, Ordering::Release);
        let Ok(active) = self.active.lock() else {
            return self.busy.load(Ordering::Acquire);
        };
        let Some(control) = active.as_ref() else {
            return self.busy.load(Ordering::Acquire);
        };
        control.cancel();
        true
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Ok(active) = self.active.lock() {
            if let Some(control) = active.as_ref() {
                control.cancel();
            }
        }
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn execute(
    vault: &mut Option<Vault>,
    health: &mut Option<metadata::Health>,
    intent: Intent,
) -> Reply {
    let mut reauthentication = None;
    let mut diagnostics_exported = None;
    if !matches!(
        intent,
        Intent::List
            | Intent::Open
            | Intent::Unlock(_)
            | Intent::OpenCodex
            | Intent::ExportDiagnostics
    ) {
        *health = None;
    }
    match intent {
        Intent::Open if vault.is_none() => *vault = Some(Vault::open().map_err(error_code)?),
        Intent::Unlock(passphrase) if vault.is_none() => {
            *vault = Some(Vault::open_with_passphrase(passphrase.as_bytes()).map_err(error_code)?);
        }
        Intent::Create => {
            if vault.is_some() {
                return Err("vault_exists");
            }
            *vault = Some(Vault::create().map_err(error_code)?);
        }
        Intent::Close => *vault = None,
        Intent::Harden(passphrase) => {
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .enable_passphrase(passphrase.as_bytes())
                .map_err(error_code)?;
        }
        Intent::Unharden => vault
            .as_mut()
            .ok_or("vault_not_open")?
            .disable_passphrase()
            .map_err(error_code)?,
        Intent::Open | Intent::Unlock(_) | Intent::List => {}
        Intent::OpenCodex => {
            vault
                .as_ref()
                .ok_or("vault_not_open")?
                .open_codex()
                .map_err(|error| error.code())?;
        }
        Intent::Capture(label) => {
            if label.len() > 320 {
                return Err("invalid_label");
            }
            let label = ProfileText::new(label.trim().to_owned()).map_err(|_| "invalid_label")?;
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .capture_current(label)
                .map_err(|error| error.code())?;
        }
        Intent::Add(label, cancellation) => {
            if label.len() > 320 {
                return Err("invalid_label");
            }
            let label = ProfileText::new(label.trim().to_owned()).map_err(|_| "invalid_label")?;
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .add_account(label, cancellation)
                .map_err(|error| error.code())?;
        }
        Intent::Reauthenticate(id, cancellation) => {
            let result = vault
                .as_mut()
                .ok_or("vault_not_open")?
                .reauthenticate(profile_id(&id)?, cancellation)
                .map_err(|error| error.code())?;
            reauthentication = Some(metadata::Reauthentication {
                profile: hex(result.profile.to_bytes()),
                installation: result.installation.map(operation),
            });
        }
        Intent::Switch(id, cancellation) => {
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .switch_to_cancellable(profile_id(&id)?, cancellation)
                .map_err(|error| error.code())?;
        }
        Intent::Recover(id, finish) => {
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .recover_switch(parse_id(&id)?, finish)
                .map_err(|error| error.code())?;
        }
        Intent::RecoverVault => {
            let vault = vault.as_mut().ok_or("vault_not_open")?;
            match vault.recovery() {
                Recovery::ControlRepairRequired => vault.recover_control(),
                Recovery::CommitPending | Recovery::CleanupPending => vault.reconcile(),
                Recovery::Clean => Ok(()),
                _ => Err(StorageError::RecoveryRequired),
            }
            .map_err(error_code)?;
        }
        Intent::Remove(id) => {
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .remove_inactive(profile_id(&id)?)
                .map_err(error_code)?;
        }
        Intent::Rename(id, label) => {
            if label.len() > 320 {
                return Err("invalid_label");
            }
            let label = ProfileText::new(label.trim().to_owned()).map_err(|_| "invalid_label")?;
            vault
                .as_mut()
                .ok_or("vault_not_open")?
                .rename_profile(profile_id(&id)?, label)
                .map_err(error_code)?;
        }
        Intent::RefreshUsage(id) => {
            let requested = Some(profile_id(&id)?);
            let observed = vault
                .as_mut()
                .ok_or("vault_not_open")?
                .refresh_usage(requested)
                .map_err(|error| error.code())?;
            *health = Some(metadata::Health {
                source: "usage",
                profile: hex(observed.profile.to_bytes()),
                acceptance: acceptance(observed.acceptance),
                checked_at: observed.checked_at,
                available_resets: observed.available_resets,
                primary: observed
                    .quotas
                    .and_then(|quotas| quota_window(quotas.primary)),
                secondary: observed
                    .quotas
                    .and_then(|quotas| quota_window(quotas.secondary)),
            });
        }
        Intent::ExportDiagnostics => {
            diagnostics_exported = Some(
                vault
                    .as_mut()
                    .ok_or("vault_not_open")?
                    .export_sanitized_diagnostics()
                    .map_err(|error| error.code())?,
            );
        }
    }
    let Some(vault) = vault else {
        return Ok(metadata::unopened_profiles(Vault::exists().ok()));
    };
    let mut result = snapshot(vault, health.as_ref()).map_err(error_code)?;
    result.reauthentication = reauthentication;
    result.diagnostics_exported = diagnostics_exported;
    Ok(result)
}

#[cfg(not(target_arch = "x86_64"))]
fn execute(_: &mut Option<Vault>, _: &mut Option<metadata::Health>, _: Intent) -> Reply {
    Err("vault_platform_unsupported")
}

fn parse_id(value: &str) -> Result<[u8; 16], &'static str> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid_selection");
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid_selection")?;
    }
    Ok(bytes)
}

fn profile_id(value: &str) -> Result<ProfileId, &'static str> {
    ProfileId::from_bytes(parse_id(value)?).map_err(|_| "invalid_selection")
}

#[cfg(target_arch = "x86_64")]
fn snapshot(vault: &Vault, health: Option<&metadata::Health>) -> Result<ProfileList, StorageError> {
    let recovery = vault.recovery();
    let recovery_name = match recovery {
        Recovery::Clean => "none",
        Recovery::SwitchPending => "switch_pending",
        Recovery::ControlRepairRequired => "control_repair",
        Recovery::CommitPending => "commit_pending",
        Recovery::CleanupPending => "cleanup_pending",
        Recovery::Blocked => "blocked",
    };
    if !matches!(recovery, Recovery::Clean | Recovery::SwitchPending) {
        return Ok(ProfileList {
            state: VaultState::RecoveryRequired,
            vault_exists: Some(true),
            protection: Some(match vault.protection() {
                codex_accounts_vault_storage::VaultProtection::WindowsUser => "windows_user",
                codex_accounts_vault_storage::VaultProtection::Passphrase => "passphrase",
            }),
            last_known_active_profile: None,
            profiles: Vec::new(),
            operations: Vec::new(),
            recovery: recovery_name,
            health: None,
            reauthentication: None,
            diagnostics_exported: None,
        });
    }
    let profiles = vault.profiles()?;
    if profiles.len() > 50 {
        return Err(StorageError::InputLimit);
    }
    Ok(ProfileList {
        vault_exists: Some(true),
        protection: Some(match vault.protection() {
            codex_accounts_vault_storage::VaultProtection::WindowsUser => "windows_user",
            codex_accounts_vault_storage::VaultProtection::Passphrase => "passphrase",
        }),
        last_known_active_profile: vault
            .last_known_active_profile()?
            .map(|id| hex(id.to_bytes())),
        state: if recovery == Recovery::Clean {
            VaultState::Open
        } else {
            VaultState::RecoveryRequired
        },
        recovery: recovery_name,
        health: health.cloned(),
        reauthentication: None,
        diagnostics_exported: None,
        operations: vault.switch_status()?.into_iter().map(operation).collect(),
        profiles: profiles
            .into_iter()
            .map(|profile| Profile {
                id: hex(profile.id.to_bytes()),
                label: profile.label,
                domain: profile.domain,
                generation: hex(profile.generation.to_bytes()),
                captured_at: profile.captured_at,
            })
            .collect(),
    })
}

fn operation(status: OperationStatus) -> Operation {
    use SwitchPhase::*;
    let phase = match status.phase {
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
    };
    Operation {
        id: hex(status.operation_id),
        phase,
        installed_profile: status.installed_profile.map(|id| hex(id.to_bytes())),
        credential_acceptance: match status.observations.credential_acceptance() {
            CredentialAcceptance::Unknown => "unknown",
            CredentialAcceptance::Accepted => "accepted",
            CredentialAcceptance::Rejected => "rejected",
            CredentialAcceptance::ObservationUnavailable => "unavailable",
        },
        desktop_launch: match status.observations.desktop_launch() {
            DesktopLaunch::NotRequested => "not_requested",
            DesktopLaunch::Opened => "opened",
            DesktopLaunch::Failed => "failed",
        },
        primary_error: status.primary_error,
        restoration_error: status.restoration_error,
        can_recover: !matches!(
            status.phase,
            AwaitingConfirmation | Finished | Restored | Cancelled | NeedsLogin
        ),
        can_restore: status.can_restore,
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn snapshot(_: &Vault, _: Option<&metadata::Health>) -> Result<ProfileList, StorageError> {
    Err(StorageError::UnsupportedPlatform)
}

fn acceptance(value: CredentialAcceptance) -> &'static str {
    match value {
        CredentialAcceptance::Accepted => "accepted",
        CredentialAcceptance::Rejected => "rejected",
        CredentialAcceptance::Unknown => "unknown",
        CredentialAcceptance::ObservationUnavailable => "unavailable",
    }
}

fn quota_window(value: Observed<RateLimitWindow>) -> Option<metadata::QuotaWindow> {
    match value {
        Observed::Value(window) => Some(metadata::QuotaWindow {
            used_percent: window.used_percent,
            duration_mins: match window.duration_mins {
                Observed::Value(value) => Some(value),
                _ => None,
            },
            resets_at: match window.resets_at {
                Observed::Value(value) => Some(value),
                _ => None,
            },
        }),
        _ => None,
    }
}

fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn error_code(error: StorageError) -> &'static str {
    match error {
        StorageError::Missing => "vault_missing",
        StorageError::AlreadyExists => "vault_exists",
        StorageError::Busy => "vault_busy",
        StorageError::AccessDenied => "vault_access_denied",
        StorageError::UnsafePath => "vault_unsafe_path",
        StorageError::KeyUnavailable => "vault_key_unavailable",
        StorageError::PassphraseRequired => "vault_passphrase_required",
        StorageError::InvalidPassphrase => "vault_invalid_passphrase",
        StorageError::ProtectionChangeUncertain => "vault_protection_uncertain",
        StorageError::Cancelled => "vault_cancelled",
        StorageError::Corrupt => "vault_corrupt",
        StorageError::RecoveryRequired => "vault_recovery_required",
        StorageError::TemporaryCleanupRequired => "login_cleanup_required",
        StorageError::LaterStartupRequired => "vault_later_startup_required",
        StorageError::ExternalChange => "vault_changed",
        StorageError::InputLimit => "vault_input_limit",
        StorageError::UnsupportedPlatform => "vault_platform_unsupported",
        StorageError::SwitchPending => "vault_switch_pending",
        StorageError::ActiveStateUnknown => "vault_active_unknown",
        StorageError::ActiveProfile => "vault_active_profile",
        StorageError::Referenced => "vault_profile_referenced",
        StorageError::InvalidData => "invalid_label",
        _ => "vault_unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn open_codex_requires_an_open_vault_before_any_launch() {
        let result = execute(&mut None, &mut None, Intent::OpenCodex);
        assert!(matches!(result, Err("vault_not_open")));
    }

    #[test]
    fn window_close_waits_for_owned_operation_cleanup() {
        for control in [
            ActiveControl::Login(LoginCancellation::new()),
            ActiveControl::Switch(SwitchCancellation::new()),
        ] {
            let worker = Worker {
                sender: None,
                thread: None,
                busy: Arc::new(AtomicBool::new(true)),
                active: Arc::new(Mutex::new(Some(control))),
                close_requested: AtomicBool::new(false),
                hardened: Arc::new(AtomicBool::new(false)),
            };
            assert!(worker.cancel_for_close());
            assert_eq!(worker.action_status(), Ok("closing"));
            // Requesting cancellation alone is not evidence of cleanup completion.
            assert!(worker.cancel_for_close());
            *worker.active.lock().unwrap() = None;
            worker.busy.store(false, Ordering::Release);
            assert!(!worker.cancel_for_close());
        }
    }

    #[test]
    fn ipc_selection_rejects_non_ascii_and_wrong_width() {
        assert_eq!(parse_id(&"01".repeat(16)), Ok([1; 16]));
        for input in ["é".repeat(16), "a".repeat(31), "g".repeat(32)] {
            assert_eq!(parse_id(&input), Err("invalid_selection"));
        }
        assert!(profile_id(&"00".repeat(16)).is_err());
    }

    #[test]
    fn renderer_preserves_failed_switch_and_distinct_observations() {
        let mut observations = codex_accounts_core::ObservationStatus::default();
        observations.observe_credentials(CredentialAcceptance::Accepted);
        observations.observe_launch(DesktopLaunch::Failed);
        let projected = operation(OperationStatus {
            operation_id: [1; 16],
            phase: SwitchPhase::Restored,
            installed_profile: None,
            observations,
            primary_error: Some("E_WRITE_FAILED"),
            restoration_error: None,
            can_restore: false,
        });
        assert_eq!(projected.phase, "restored");
        assert_eq!(projected.primary_error, Some("E_WRITE_FAILED"));
        assert_eq!(projected.credential_acceptance, "accepted");
        assert_eq!(projected.desktop_launch, "failed");
        assert!(!projected.can_recover);
        let encoded = serde_json::to_string(&projected).unwrap();
        assert!(!encoded.contains("identity_confirmed"));
        let mut rejected = codex_accounts_core::ObservationStatus::default();
        rejected.observe_credentials(CredentialAcceptance::Rejected);
        let needs_login = operation(OperationStatus {
            operation_id: [2; 16],
            phase: SwitchPhase::NeedsLogin,
            installed_profile: Some(ProfileId::from_bytes([3; 16]).unwrap()),
            observations: rejected,
            primary_error: Some("E_LOGIN_REQUIRED"),
            restoration_error: None,
            can_restore: false,
        });
        assert_eq!(needs_login.phase, "needs_login");
        assert_eq!(needs_login.credential_acceptance, "rejected");
        assert!(!needs_login.can_recover);
        assert!(!needs_login.can_restore);
    }
}
