#![forbid(unsafe_code)]
use crate::random::{self, Entropy, SystemEntropy};
use crate::{CryptoError, EnvelopeContext};
use hkdf::{
    hmac::{Hmac, Mac},
    Hkdf,
};
use sha2::Sha256;
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

pub(crate) const ENCRYPTION_INFO: &[u8] = b"codex-accounts/v1/encryption/xchacha20poly1305";
const FINGERPRINT_INFO: &[u8] = b"codex-accounts/v1/fingerprint/hmac-sha256";
const PROTECTED_MAGIC: &[u8; 4] = b"CAKP";
const MAX_PROTECTED_BYTES: usize = 4096;

/// A plaintext buffer with explicit, borrow-only library access and redacted Debug.
/// Does not promise removal of caller copies, parser scratch, swap or crash dumps.
pub struct SecretBytes(pub(crate) Zeroizing<Vec<u8>>);
impl SecretBytes {
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

/// Random root key and random key identifier. No implicit duplication or serializer implementation.
/// There is no caller-supplied raw-key constructor in production.
pub struct RootKey {
    pub(crate) secret: Zeroizing<[u8; 32]>,
    pub(crate) id: [u8; 16],
}
impl RootKey {
    /// Non-secret random identifier recovered from native-protected root material.
    /// Storage may use it to select context independently of untrusted envelopes.
    pub fn identifier(&self) -> [u8; 16] {
        self.id
    }

    pub fn generate() -> Result<Self, CryptoError> {
        Self::generate_using(&mut SystemEntropy)
    }
    fn generate_using(rng: &mut impl Entropy) -> Result<Self, CryptoError> {
        let mut secret = Zeroizing::new([0; 32]);
        let mut id = [0; 16];
        random::fill(rng, secret.as_mut())?;
        random::fill(rng, &mut id)?;
        // Defensive rejection, not an entropy-quality test or retry mechanism.
        if *secret == [0; 32] || id == [0; 16] {
            return Err(CryptoError::RandomnessUnavailable);
        }
        Ok(Self { secret, id })
    }
    pub(crate) fn derive(&self, info: &[u8]) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        let mut key = Zeroizing::new([0; 32]);
        Hkdf::<Sha256>::new(Some(&self.id), self.secret.as_ref())
            .expand(info, key.as_mut())
            .map_err(|_| CryptoError::InputLimit)?;
        Ok(key)
    }
    fn fingerprint_mac(
        &self,
        context: &EnvelopeContext,
        bytes: Option<&[u8]>,
    ) -> Result<Hmac<Sha256>, CryptoError> {
        let data = bytes.unwrap_or_default();
        context.check_length(data.len())?;
        let key = self.derive(FINGERPRINT_INFO)?;
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_ref())
            .map_err(|_| CryptoError::InputLimit)?;
        mac.update(b"codex-accounts/fingerprint/v1\0");
        mac.update(&context.encoded());
        mac.update(&[u8::from(bytes.is_some())]);
        mac.update(&(data.len() as u32).to_le_bytes());
        mac.update(data);
        Ok(mac)
    }
    /// Context-bound equality evidence only, never exportable diagnostics.
    pub fn fingerprint(
        &self,
        context: &EnvelopeContext,
        bytes: Option<&[u8]>,
    ) -> Result<Fingerprint, CryptoError> {
        let mut tag = self
            .fingerprint_mac(context, bytes)?
            .finalize()
            .into_bytes();
        let mut value = Zeroizing::new([0; 32]);
        value.copy_from_slice(&tag);
        tag.as_mut_slice().zeroize();
        Ok(Fingerprint(value))
    }
    pub fn verify_fingerprint(
        &self,
        expected: &Fingerprint,
        context: &EnvelopeContext,
        bytes: Option<&[u8]>,
    ) -> Result<(), CryptoError> {
        self.fingerprint_mac(context, bytes)?
            .verify_slice(expected.0.as_ref())
            .map_err(|_| CryptoError::AuthenticationFailed)
    }
    pub fn protect(&self) -> Result<ProtectedRootKey, CryptoError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            crate::dpapi::protect(self)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            Err(CryptoError::UnsupportedPlatform)
        }
    }
    pub fn unprotect(protected: &ProtectedRootKey) -> Result<Self, CryptoError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            crate::dpapi::unprotect(protected)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            let _ = protected;
            Err(CryptoError::UnsupportedPlatform)
        }
    }
}

pub struct Fingerprint(Zeroizing<[u8; 32]>);

/// Versioned ciphertext, not proof of its origin, user scope, or storage safety.
pub struct ProtectedRootKey(Vec<u8>);
impl ProtectedRootKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() > MAX_PROTECTED_BYTES {
            return Err(CryptoError::InputLimit);
        }
        if bytes.len() < 11 || &bytes[..4] != PROTECTED_MAGIC {
            return Err(CryptoError::InvalidEnvelope);
        }
        if bytes[4] != 1 {
            return Err(CryptoError::UnsupportedVersion);
        }
        if bytes[5] != 1 {
            return Err(CryptoError::UnsupportedAlgorithm);
        }
        let len = u32::from_le_bytes(
            bytes[6..10]
                .try_into()
                .map_err(|_| CryptoError::InvalidEnvelope)?,
        ) as usize;
        if len != bytes.len() - 10 {
            return Err(CryptoError::InvalidEnvelope);
        }
        Ok(Self(bytes.to_vec()))
    }
    /// Encrypted bytes only. Persistent storage is deliberately not implemented.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    pub(crate) fn wrap(blob: &[u8]) -> Result<Self, CryptoError> {
        if blob.is_empty() || blob.len() > MAX_PROTECTED_BYTES - 10 {
            return Err(CryptoError::InputLimit);
        }
        let mut bytes = Vec::with_capacity(10 + blob.len());
        bytes.extend_from_slice(PROTECTED_MAGIC);
        bytes.extend_from_slice(&[1, 1]);
        bytes.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        bytes.extend_from_slice(blob);
        Self::from_bytes(&bytes)
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    pub(crate) fn blob(&self) -> &[u8] {
        &self.0[10..]
    }
}

macro_rules! redacted {
    ($($kind:ty),+) => {$(impl fmt::Debug for $kind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(concat!(stringify!($kind), "([REDACTED])"))
        }
    })+};
}
redacted!(RootKey, ProtectedRootKey, SecretBytes, Fingerprint);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::FailingEntropy;
    #[test]
    fn root_failure_returns_no_key() {
        assert!(matches!(
            RootKey::generate_using(&mut FailingEntropy),
            Err(CryptoError::RandomnessUnavailable)
        ));
    }
    #[test]
    fn key_id_failure_also_returns_no_key() {
        struct SecondFails(usize);
        impl Entropy for SecondFails {
            fn fill(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
                self.0 += 1;
                out.fill(0x53);
                if self.0 == 2 {
                    Err(CryptoError::RandomnessUnavailable)
                } else {
                    Ok(())
                }
            }
        }
        assert!(matches!(
            RootKey::generate_using(&mut SecondFails(0)),
            Err(CryptoError::RandomnessUnavailable)
        ));
    }
    #[test]
    fn encryption_and_fingerprint_keys_are_separate_and_key_id_bound() {
        let root = RootKey {
            secret: Zeroizing::new([0x53; 32]),
            id: [0x41; 16],
        };
        assert!(*root.derive(ENCRYPTION_INFO).unwrap() != *root.derive(FINGERPRINT_INFO).unwrap());
        let other = RootKey {
            secret: Zeroizing::new([0x53; 32]),
            id: [0x42; 16],
        };
        assert!(*root.derive(ENCRYPTION_INFO).unwrap() != *other.derive(ENCRYPTION_INFO).unwrap());
    }
}
