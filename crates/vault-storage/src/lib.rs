//! Encrypted account storage and journaled native file-backend handoff.
//! Credential paths, official executable selection, and process ownership stay
//! inside the native adapter rather than crossing the renderer boundary.
//!
//! # Public metadata construction does not perform I/O
//!
//! ```
//! use codex_accounts_vault_storage::{ProfileText, Vault};
//! assert!(ProfileText::new("SYNTHETIC_LABEL".to_owned()).is_ok());
//! assert!(ProfileText::new(String::new()).is_err());
//! let unopened: Option<Vault> = None;
//! assert!(unopened.is_none());
//! ```
//!
//! # Coordinator effects cannot be supplied by a library consumer
//!
//! ```compile_fail,E0603
//! use codex_accounts_vault_storage::engine::coordinator::Effects;
//! ```
//!
//! # A public vault does not expose the low-level coordinator
//!
//! ```compile_fail,E0599
//! use codex_accounts_vault_storage::Vault;
//! fn cannot_dispatch(vault: &mut Vault) {
//!     vault.begin_switch();
//! }
//! ```
//!
//! These executable API checks do not qualify any native or Desktop behavior.
#![deny(unsafe_code)]
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod auth_capture;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod codec;
#[allow(dead_code)]
mod desktop_selection;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod engine;
mod health;
pub use health::AccountHealth;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod login_control;
pub use login_control::{LoginCancellation, LoginStage};
mod switch_control;
pub use switch_control::SwitchCancellation;
#[cfg(all(windows, target_arch = "x86_64"))]
#[allow(unsafe_code)]
mod native;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod records;
// Journal effects remain private to the native operation adapter and tests.
#[allow(dead_code)]
mod journal;
pub use journal::{OperationStatus, SwitchPhase};
/// A saved inactive login or the exact outcome of repairing the current account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReauthenticationResult {
    pub profile: ProfileId,
    pub installation: Option<OperationStatus>,
}
// Observation logic is not a qualified installation or a public effect executor.
#[allow(dead_code)]
mod lifecycle_model;
// Pure placement policy only; native probing and guarded access are separate.
#[allow(dead_code)]
mod storage_location;

use codex_accounts_vault::{CredentialSet, Identity, Resource, ResourceId, ResourceShape};
pub use codex_accounts_vault::{GenerationId, ProfileId};
pub use records::Recovery;
use zeroize::Zeroizing;

/// Fixed, non-secret operation result suitable for presentation by the desktop UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationError(&'static str);
impl OperationError {
    pub fn code(self) -> &'static str {
        self.0
    }
}
impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for OperationError {}
impl From<StorageError> for OperationError {
    fn from(value: StorageError) -> Self {
        Self(match value {
            StorageError::Busy => "E_BUSY",
            StorageError::Missing => "E_NOT_FOUND",
            StorageError::RecoveryRequired | StorageError::SwitchPending => "E_RECOVERY_REQUIRED",
            StorageError::TemporaryCleanupRequired => "E_TEMP_CLEANUP_FAILED",
            StorageError::ExternalChange | StorageError::StaleParent => "E_EXTERNAL_CHANGE",
            StorageError::UnsafePath | StorageError::AccessDenied => "E_STORAGE_UNSAFE",
            StorageError::KeyUnavailable => "E_KEY_UNAVAILABLE",
            StorageError::InputLimit => "E_STORAGE_LIMIT",
            StorageError::IdentityMismatch => "E_IDENTITY_MISMATCH",
            StorageError::UnsupportedPlatform => "E_UNSUPPORTED_PLATFORM",
            StorageError::Corrupt | StorageError::InvalidData => "E_VAULT_CORRUPT",
            _ => "E_STORAGE_FAILED",
        })
    }
}

/// User-facing metadata decrypted from the authenticated registry. Never contains
/// credentials or account identifiers; callers must not include labels in logs.
pub struct ProfileSummary {
    pub id: ProfileId,
    pub label: String,
    pub domain: String,
    pub generation: GenerationId,
    /// Latest credential generation capture time, as Unix seconds; not a health check.
    pub captured_at: u64,
}

/// Fixed errors never contain paths, labels, identities, or OS error text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    UnsupportedPlatform,
    UnsafePath,
    AccessDenied,
    Busy,
    Missing,
    AlreadyExists,
    Io,
    Corrupt,
    KeyUnavailable,
    RecoveryRequired,
    TemporaryCleanupRequired,
    ExternalChange,
    InputLimit,
    InvalidData,
    IdentityMismatch,
    StaleParent,
    ActiveStateUnknown,
    ActiveProfile,
    Referenced,
    LaterStartupRequired,
    SwitchPending,
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StorageError {}
impl From<codex_accounts_vault_crypto::CryptoError> for StorageError {
    fn from(_: codex_accounts_vault_crypto::CryptoError) -> Self {
        Self::Corrupt
    }
}
impl From<codex_accounts_vault::DataError> for StorageError {
    fn from(_: codex_accounts_vault::DataError) -> Self {
        Self::InvalidData
    }
}

/// Structural input only, not validated OAuth or observed Desktop identity.
/// The library accepts owned memory, never a path to a credential file.
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
pub struct Capture {
    identity: Identity,
    schema: u32,
    rules: Vec<(ResourceId, ResourceShape, bool)>,
    data: CredentialSet,
}
impl Capture {
    pub fn new(
        identity: Identity,
        schema: u32,
        rules: Vec<(ResourceId, ResourceShape, bool)>,
        resources: Vec<Resource>,
    ) -> Result<Self, StorageError> {
        if schema == 0 {
            return Err(StorageError::InvalidData);
        }
        let data = CredentialSet::new(&rules, resources)?;
        Ok(Self {
            identity,
            schema,
            rules,
            data,
        })
    }
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Capture([REDACTED])")
    }
}

/// Metadata text is encoded only inside encrypted records. Not a filename.
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
pub struct ProfileText(Zeroizing<String>);
impl ProfileText {
    pub fn new(value: String) -> Result<Self, StorageError> {
        let value = Zeroizing::new(value);
        if value.is_empty()
            || value.chars().count() > 80
            || value.len() > 320
            || value.chars().any(char::is_control)
        {
            return Err(StorageError::InvalidData);
        }
        Ok(Self(value))
    }
}
impl std::fmt::Debug for ProfileText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProfileText([REDACTED])")
    }
}

/// Only the native constructor can supply storage and root-key ownership.
/// Other platforms have no fallback, and there is no public arbitrary-path API.
pub struct Vault {
    #[cfg(all(windows, target_arch = "x86_64"))]
    root: codex_accounts_vault_crypto::RootKey,
    #[cfg(all(windows, target_arch = "x86_64"))]
    storage: engine::Storage<native::Disk>,
}
impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Vault([REDACTED])")
    }
}
impl Vault {
    /// Advisory presence only. Opening or creating still performs full native
    /// validation. An inaccessible or partial layout is never reported absent.
    pub fn exists() -> Result<bool, StorageError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            native::vault_exists()
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            Err(StorageError::UnsupportedPlatform)
        }
    }
    /// Open existing storage only. Never regenerate a lost/corrupt root key.
    pub fn open() -> Result<Self, StorageError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            native::open_vault(false)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            Err(StorageError::UnsupportedPlatform)
        }
    }
    /// Create a new application-owned root, refusing any existing root.
    pub fn create() -> Result<Self, StorageError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            native::open_vault(true)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            Err(StorageError::UnsupportedPlatform)
        }
    }
}
#[cfg(all(windows, target_arch = "x86_64"))]
impl Vault {
    /// Metadata only. This does not start, resume, or authorize a switch.
    pub fn profiles(&self) -> Result<Vec<ProfileSummary>, StorageError> {
        self.storage.profiles()
    }

    /// Last authenticated association observed by Accounts, not live Desktop identity.
    /// Unknown associations and unfinished recovery expose no active profile.
    pub fn last_known_active_profile(&self) -> Result<Option<ProfileId>, StorageError> {
        self.storage.last_known_active_profile()
    }

    /// Rename encrypted profile metadata without changing credential generations.
    pub fn rename_profile(
        &mut self,
        profile: ProfileId,
        label: ProfileText,
    ) -> Result<(), StorageError> {
        self.storage.rename_profile(&self.root, profile, label)
    }

    /// Metadata only. This does not start, resume, or authorize a switch.
    pub fn switch_status(&self) -> Result<Vec<OperationStatus>, StorageError> {
        self.storage.switch_status()
    }
    pub fn recovery(&self) -> Recovery {
        self.storage.recovery()
    }
    /// Explicitly preserve a torn unpublished control stage under encryption,
    /// then retain the last authenticated state. Never adopts the torn record.
    pub fn recover_control(&mut self) -> Result<(), StorageError> {
        self.storage.recover_control(&self.root)
    }
    pub fn restore_previous(&mut self) -> Result<(), StorageError> {
        self.storage.restore_previous(&self.root)
    }
    pub fn reconcile(&mut self) -> Result<(), StorageError> {
        self.storage.reconcile(&self.root)
    }
    pub fn add(
        &mut self,
        label: ProfileText,
        domain: ProfileText,
        capture: Capture,
    ) -> Result<(ProfileId, GenerationId), StorageError> {
        self.storage.add(&self.root, label, domain, capture)
    }
    pub fn append(
        &mut self,
        profile: ProfileId,
        expected: GenerationId,
        capture: Capture,
    ) -> Result<GenerationId, StorageError> {
        self.storage.append(&self.root, profile, expected, capture)
    }
    pub fn latest(&self, profile: ProfileId) -> Result<GenerationId, StorageError> {
        self.storage.latest(profile)
    }
    pub fn read_latest(&self, profile: ProfileId) -> Result<CredentialSet, StorageError> {
        self.storage.read_latest(&self.root, profile)
    }
    pub fn prune(&mut self) -> Result<(), StorageError> {
        self.storage.prune(&self.root)
    }
    /// Refused while the authoritative active association is unknown. No public
    /// method in CA-03C can manufacture that association or clear its protection.
    pub fn remove_inactive(&mut self, profile: ProfileId) -> Result<(), StorageError> {
        self.storage.remove(&self.root, profile)
    }
}
