use crate::{DataError, MAX_GENERATIONS};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use zeroize::Zeroize;

/// Composite principal/workspace identity. This is a comparison model only:
/// constructing one does not verify token claims, an issuer, or Desktop identity.
#[derive(PartialEq, Eq)]
pub struct Identity {
    issuer: String,
    subject: String,
    workspace: String,
}
impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Identity([REDACTED])")
    }
}
impl Drop for Identity {
    fn drop(&mut self) {
        self.issuer.zeroize();
        self.subject.zeroize();
        self.workspace.zeroize();
    }
}
impl Identity {
    pub fn new(issuer: String, subject: String, workspace: String) -> Result<Self, DataError> {
        let identity = Self {
            issuer,
            subject,
            workspace,
        };
        for value in [&identity.issuer, &identity.subject, &identity.workspace] {
            if value.is_empty()
                || value.len() > 2048
                || value.trim() != value
                || value.chars().any(char::is_control)
            {
                return Err(DataError::InvalidIdentity);
            }
        }
        Ok(identity)
    }
}

// IDs are accepted from a future reviewed UUID generator. No RNG or filesystem
// naming authority is implemented here. Distinct types prevent ID interchange.
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name([u8; 16]);
        impl $name {
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, DataError> {
                if bytes == [0; 16] {
                    return Err(DataError::InvalidId);
                }
                Ok(Self(bytes))
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }
    };
}
id_type!(ProfileId);
id_type!(GenerationId);

/// Metadata-only history; it stores no credential bytes and performs no deletion.
/// Parent checks prevent stale writes within this model, not across processes or
/// durable storage. A future journal/storage commit must make both updates atomic.
pub struct GenerationIndex {
    profile: ProfileId,
    identity: Identity,
    latest: GenerationId,
    parents: BTreeMap<GenerationId, Option<GenerationId>>,
}
impl fmt::Debug for GenerationIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GenerationIndex([REDACTED])")
    }
}
impl GenerationIndex {
    pub fn new(profile: ProfileId, identity: Identity, initial: GenerationId) -> Self {
        Self {
            profile,
            identity,
            latest: initial,
            parents: BTreeMap::from([(initial, None)]),
        }
    }
    pub fn latest(&self) -> GenerationId {
        self.latest
    }
    pub fn append(
        &mut self,
        profile: ProfileId,
        identity: &Identity,
        expected_parent: GenerationId,
        new_generation: GenerationId,
    ) -> Result<(), DataError> {
        if profile != self.profile || identity != &self.identity {
            return Err(DataError::IdentityMismatch);
        }
        if self.parents.contains_key(&new_generation) {
            return Err(DataError::DuplicateGeneration);
        }
        if expected_parent != self.latest {
            return Err(DataError::StaleParent);
        }
        if self.parents.len() >= MAX_GENERATIONS {
            return Err(DataError::InputLimit);
        }
        self.parents.insert(new_generation, Some(expected_parent));
        self.latest = new_generation;
        Ok(())
    }
    /// Compute candidates only. The caller must first establish a clean terminal
    /// state and verified later startup before applying any durable pruning.
    /// Every unresolved journal reference must be supplied; this API cannot prove
    /// completeness of that list. Unknown references refuse the whole decision.
    pub fn unreferenced_candidates(
        &self,
        unresolved: &[GenerationId],
    ) -> Result<Vec<GenerationId>, DataError> {
        if unresolved.len() > MAX_GENERATIONS {
            return Err(DataError::InputLimit);
        }
        if unresolved.iter().any(|id| !self.parents.contains_key(id)) {
            return Err(DataError::UnknownGeneration);
        }
        let protected: BTreeSet<_> = unresolved
            .iter()
            .copied()
            .chain(std::iter::once(self.latest))
            .collect();
        Ok(self
            .parents
            .keys()
            .filter(|id| !protected.contains(*id))
            .copied()
            .collect())
    }
}
