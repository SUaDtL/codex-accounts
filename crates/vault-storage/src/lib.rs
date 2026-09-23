//! CA-03C: encrypted storage library. No Codex file reads, login, process control,
//! credential CLI, or authority to qualify a Desktop installation.
#![deny(unsafe_code)]
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod codec;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod engine;
#[cfg(all(windows, target_arch = "x86_64"))]
#[allow(unsafe_code)]
mod native;
#[cfg_attr(not(all(windows, target_arch = "x86_64")), allow(dead_code))]
mod records;

use codex_accounts_vault::{CredentialSet, Identity, Resource, ResourceId, ResourceShape};
pub use codex_accounts_vault::{GenerationId, ProfileId};
pub use records::Recovery;
use zeroize::Zeroizing;

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
    ExternalChange,
    InputLimit,
    InvalidData,
    IdentityMismatch,
    StaleParent,
    ActiveStateUnknown,
    ActiveProfile,
    Referenced,
    LaterStartupRequired,
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
    pub fn recovery(&self) -> Recovery {
        self.storage.recovery()
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
