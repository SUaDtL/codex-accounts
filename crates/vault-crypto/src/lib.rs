//! In-memory envelope primitives. Not persistent storage or account authority.
//! No key import/export, credential CLI, filesystem, network or runtime dispatch.
#![deny(unsafe_code)]

#[cfg(all(windows, target_arch = "x86_64"))]
#[allow(unsafe_code)]
mod dpapi;
mod envelope;
mod keys;
mod random;

pub use envelope::{Envelope, EnvelopeContext, Purpose, MAX_ENVELOPE_BYTES, MAX_PLAINTEXT_BYTES};
pub use keys::{Fingerprint, ProtectedRootKey, RootKey, SecretBytes};

/// Fixed categories only: never include a library, OS, path or caller error body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    InputLimit,
    InvalidContext,
    InvalidEnvelope,
    UnsupportedVersion,
    UnsupportedAlgorithm,
    AuthenticationFailed,
    RandomnessUnavailable,
    KeyProtectionUnavailable,
    UnsupportedPlatform,
}
impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::InputLimit => "crypto input limit",
            Self::InvalidContext => "invalid envelope context",
            Self::InvalidEnvelope => "invalid encrypted envelope",
            Self::UnsupportedVersion => "unsupported envelope version",
            Self::UnsupportedAlgorithm => "unsupported envelope algorithm",
            Self::AuthenticationFailed => "encrypted data authentication failed",
            Self::RandomnessUnavailable => "OS randomness unavailable",
            Self::KeyProtectionUnavailable => "native key protection unavailable",
            Self::UnsupportedPlatform => "native key protection unsupported on this platform",
        };
        f.write_str(text)
    }
}
impl std::error::Error for CryptoError {}
