//! CA-03A: bounded in-memory data, not an encrypted or persistent vault.
//! No file, network, process, key-store, or live account authority is exposed.
//! Structural JSON validation does not establish a supported OAuth schema.
#![forbid(unsafe_code)]

mod generations;
mod resources;
mod strict_json;

pub use generations::{GenerationId, GenerationIndex, Identity, ProfileId};
pub use resources::{CredentialSet, Resource, ResourceId, ResourceShape};
pub use strict_json::validate_json_object;

pub const MAX_RESOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_SET_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_JSON_DEPTH: usize = 64;
pub const MAX_RESOURCES: usize = 16;
pub const MAX_GENERATIONS: usize = 128;

/// Error categories deliberately contain no input bytes, identities, or paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataError {
    InputLimit,
    InvalidJson,
    DuplicateKey,
    DepthLimit,
    InvalidIdentity,
    InvalidId,
    ResourceSetMismatch,
    DuplicateResource,
    RequiredResourceAbsent,
    IdentityMismatch,
    StaleParent,
    DuplicateGeneration,
    UnknownGeneration,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DataError {}
