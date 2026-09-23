//! Q0 safety vocabulary. No filesystem, process, network, or credential authority.
//! Synthetic status observations are not native integration evidence.
#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    File,
    Auto,
    Keyring,
    Secrets,
    Ephemeral,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    CompatUnknown,
    BackendUnsupported,
    PolicyDenied,
    ClientsRunning,
    RecoveryRequired,
    ObservationUnavailable,
    LaunchFailed,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CompatUnknown => "E_COMPAT_UNKNOWN",
            Self::BackendUnsupported => "E_BACKEND_UNSUPPORTED",
            Self::PolicyDenied => "E_POLICY_DENIED",
            Self::ClientsRunning => "E_CLIENTS_RUNNING",
            Self::RecoveryRequired => "E_RECOVERY_REQUIRED",
            Self::ObservationUnavailable => "E_OBSERVATION_UNAVAILABLE",
            Self::LaunchFailed => "E_LAUNCH_FAILED",
        }
    }
}

/// Q0 has no implementation that can authorize an authentication write.
/// Adding a compatibility JSON record cannot change this function's decision.
pub const fn mutation_availability(backend: Backend) -> Result<(), ErrorCode> {
    match backend {
        Backend::File | Backend::Unknown => Err(ErrorCode::CompatUnknown),
        _ => Err(ErrorCode::BackendUnsupported),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CredentialAcceptance {
    #[default]
    Unknown,
    Accepted,
    Rejected,
    ObservationUnavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DesktopLaunch {
    #[default]
    NotRequested,
    Opened,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DesktopIdentity {
    #[default]
    Unknown,
    AwaitingConfirmation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecoveryState {
    #[default]
    None,
    Required,
    Conflict,
}

/// Observation projection only. This type cannot assert an installed account.
/// Confirmation variants must not be added until their evidence adapters exist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservationStatus {
    acceptance: CredentialAcceptance,
    launch: DesktopLaunch,
    identity: DesktopIdentity,
    recovery: RecoveryState,
}

impl ObservationStatus {
    pub fn observe_credentials(&mut self, value: CredentialAcceptance) {
        self.acceptance = value;
    }

    pub fn observe_launch(&mut self, value: DesktopLaunch) {
        self.launch = value;
        self.identity = if value == DesktopLaunch::Opened && self.recovery == RecoveryState::None {
            DesktopIdentity::AwaitingConfirmation
        } else {
            DesktopIdentity::Unknown
        };
    }

    pub fn require_recovery(&mut self, conflict: bool) {
        self.recovery = if conflict {
            RecoveryState::Conflict
        } else {
            RecoveryState::Required
        };
        self.identity = DesktopIdentity::Unknown;
    }

    pub const fn credential_acceptance(&self) -> CredentialAcceptance {
        self.acceptance
    }

    pub const fn desktop_launch(&self) -> DesktopLaunch {
        self.launch
    }

    pub const fn desktop_identity(&self) -> DesktopIdentity {
        self.identity
    }

    pub const fn recovery_state(&self) -> RecoveryState {
        self.recovery
    }
}

pub mod compatibility;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_can_enable_mutation() {
        for backend in [
            Backend::File,
            Backend::Auto,
            Backend::Keyring,
            Backend::Secrets,
            Backend::Ephemeral,
            Backend::Unknown,
        ] {
            assert!(mutation_availability(backend).is_err());
        }
    }

    #[test]
    fn auto_does_not_become_eligible_because_a_file_exists() {
        assert_eq!(
            mutation_availability(Backend::Auto),
            Err(ErrorCode::BackendUnsupported)
        );
    }

    #[test]
    fn helper_acceptance_does_not_confirm_desktop() {
        let mut status = ObservationStatus::default();
        status.observe_credentials(CredentialAcceptance::Accepted);
        assert_eq!(status.desktop_identity(), DesktopIdentity::Unknown);
        assert_eq!(status.desktop_launch(), DesktopLaunch::NotRequested);
    }

    #[test]
    fn process_appearance_only_requests_confirmation() {
        let mut status = ObservationStatus::default();
        status.observe_launch(DesktopLaunch::Opened);
        assert_eq!(
            status.desktop_identity(),
            DesktopIdentity::AwaitingConfirmation
        );
    }

    #[test]
    fn offline_is_not_rejected() {
        let mut status = ObservationStatus::default();
        status.observe_credentials(CredentialAcceptance::ObservationUnavailable);
        assert_ne!(
            status.credential_acceptance(),
            CredentialAcceptance::Rejected
        );
    }

    #[test]
    fn launch_failure_preserves_credential_observation() {
        let mut status = ObservationStatus::default();
        status.observe_credentials(CredentialAcceptance::Accepted);
        status.observe_launch(DesktopLaunch::Failed);
        assert_eq!(
            status.credential_acceptance(),
            CredentialAcceptance::Accepted
        );
        assert_eq!(status.desktop_identity(), DesktopIdentity::Unknown);
    }

    #[test]
    fn recovery_clears_pending_confirmation() {
        let mut status = ObservationStatus::default();
        status.observe_launch(DesktopLaunch::Opened);
        status.require_recovery(true);
        assert_eq!(status.recovery_state(), RecoveryState::Conflict);
        assert_eq!(status.desktop_identity(), DesktopIdentity::Unknown);
    }
}
