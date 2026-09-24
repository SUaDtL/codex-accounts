#![forbid(unsafe_code)]
use crate::keys::{SecretBytes, ENCRYPTION_INFO};
use crate::random::{self, Entropy, SystemEntropy};
use crate::{CryptoError, RootKey};
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    Key, Tag, XChaCha20Poly1305, XNonce,
};
use std::fmt;
use zeroize::Zeroizing;

pub const MAX_PLAINTEXT_BYTES: usize = 4 * 1024 * 1024;
const RESOURCE_LIMIT: usize = 1024 * 1024;
const HEADER_BYTES: usize = 50;
const TAG_BYTES: usize = 16;
pub const MAX_ENVELOPE_BYTES: usize = MAX_PLAINTEXT_BYTES + HEADER_BYTES + TAG_BYTES;
const MAGIC: &[u8; 4] = b"CAEN";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Purpose {
    Registry = 1,
    Identity = 2,
    Generation = 3,
    Journal = 4,
    CredentialResource = 5,
}

/// Structural, opaque identifiers only. No credential identities, paths or labels.
/// This expected context must come from the selected record, not from the envelope.
#[derive(Clone, Copy)]
pub struct EnvelopeContext {
    purpose: Purpose,
    schema: u32,
    profile: [u8; 16],
    generation: [u8; 16],
    resource: u16,
}
impl EnvelopeContext {
    pub fn new(
        purpose: Purpose,
        schema: u32,
        profile: [u8; 16],
        generation: [u8; 16],
        resource: u16,
    ) -> Result<Self, CryptoError> {
        if schema == 0
            || profile == [0; 16]
            || generation == [0; 16]
            || (purpose == Purpose::CredentialResource && resource >= 16)
            || (purpose != Purpose::CredentialResource && resource != 0)
        {
            return Err(CryptoError::InvalidContext);
        }
        Ok(Self {
            purpose,
            schema,
            profile,
            generation,
            resource,
        })
    }
    pub(crate) fn encoded(&self) -> [u8; 39] {
        let mut out = [0; 39];
        out[0] = self.purpose as u8;
        out[1..5].copy_from_slice(&self.schema.to_le_bytes());
        out[5..21].copy_from_slice(&self.profile);
        out[21..37].copy_from_slice(&self.generation);
        out[37..39].copy_from_slice(&self.resource.to_le_bytes());
        out
    }
    pub(crate) fn check_length(&self, size: usize) -> Result<(), CryptoError> {
        let limit = if self.purpose == Purpose::CredentialResource {
            RESOURCE_LIMIT
        } else {
            MAX_PLAINTEXT_BYTES
        };
        if size > limit {
            Err(CryptoError::InputLimit)
        } else {
            Ok(())
        }
    }
}
impl fmt::Debug for EnvelopeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EnvelopeContext([REDACTED])")
    }
}

/// Strict fixed-format encrypted bytes. Context is authenticated but not stored here.
/// A valid tag is not freshness, Codex-schema validation or account authority.
pub struct Envelope(Vec<u8>);
impl Envelope {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        validate(bytes)?;
        Ok(Self(bytes.to_vec()))
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for Envelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Envelope([REDACTED])")
    }
}
fn validate(bytes: &[u8]) -> Result<usize, CryptoError> {
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(CryptoError::InputLimit);
    }
    if bytes.len() < HEADER_BYTES + TAG_BYTES || &bytes[..4] != MAGIC {
        return Err(CryptoError::InvalidEnvelope);
    }
    if bytes[4] != 1 {
        return Err(CryptoError::UnsupportedVersion);
    }
    if bytes[5] != 1 {
        return Err(CryptoError::UnsupportedAlgorithm);
    }
    if bytes[6..22] == [0; 16] {
        return Err(CryptoError::InvalidEnvelope);
    }
    let len = u32::from_le_bytes(
        bytes[46..50]
            .try_into()
            .map_err(|_| CryptoError::InvalidEnvelope)?,
    ) as usize;
    if len != bytes.len() - HEADER_BYTES - TAG_BYTES {
        return Err(CryptoError::InvalidEnvelope);
    }
    Ok(len)
}
fn aad(header: &[u8], context: &EnvelopeContext) -> Vec<u8> {
    let mut out = b"codex-accounts/envelope/v1\0".to_vec();
    out.extend_from_slice(header);
    out.extend_from_slice(&context.encoded());
    out
}
impl RootKey {
    /// Every call obtains a fresh 24-byte nonce directly from OS randomness.
    pub fn seal(
        &self,
        context: &EnvelopeContext,
        plaintext: &[u8],
    ) -> Result<Envelope, CryptoError> {
        self.seal_using(context, plaintext, &mut SystemEntropy)
    }
    fn seal_using(
        &self,
        context: &EnvelopeContext,
        plaintext: &[u8],
        rng: &mut impl Entropy,
    ) -> Result<Envelope, CryptoError> {
        context.check_length(plaintext.len())?;
        let mut nonce = [0; 24];
        random::fill(rng, &mut nonce)?;
        let mut header = [0; HEADER_BYTES];
        header[..4].copy_from_slice(MAGIC);
        header[4..6].copy_from_slice(&[1, 1]);
        header[6..22].copy_from_slice(&self.id);
        header[22..46].copy_from_slice(&nonce);
        header[46..50].copy_from_slice(&(plaintext.len() as u32).to_le_bytes());
        let key = self.derive(ENCRYPTION_INFO)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
        let mut data = Zeroizing::new(plaintext.to_vec());
        let tag = cipher
            .encrypt_in_place_detached(
                XNonce::from_slice(&nonce),
                &aad(&header, context),
                &mut data,
            )
            .map_err(|_| CryptoError::AuthenticationFailed)?;
        let mut encoded = Vec::with_capacity(HEADER_BYTES + data.len() + TAG_BYTES);
        encoded.extend_from_slice(&header);
        encoded.extend_from_slice(&data);
        encoded.extend_from_slice(&tag);
        Ok(Envelope(encoded))
    }
    pub fn open(
        &self,
        context: &EnvelopeContext,
        envelope: &Envelope,
    ) -> Result<SecretBytes, CryptoError> {
        let bytes = envelope.as_bytes();
        let len = validate(bytes)?;
        context.check_length(len)?;
        if bytes[6..22] != self.id {
            return Err(CryptoError::AuthenticationFailed);
        }
        let key = self.derive(ENCRYPTION_INFO)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
        let mut data = Zeroizing::new(bytes[HEADER_BYTES..HEADER_BYTES + len].to_vec());
        cipher
            .decrypt_in_place_detached(
                XNonce::from_slice(&bytes[22..46]),
                &aad(&bytes[..HEADER_BYTES], context),
                &mut data,
                Tag::from_slice(&bytes[HEADER_BYTES + len..]),
            )
            .map_err(|_| CryptoError::AuthenticationFailed)?;
        Ok(SecretBytes(data))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::FailingEntropy;
    #[test]
    fn nonce_failure_never_returns_an_envelope() {
        let key = RootKey::generate().unwrap();
        let context =
            EnvelopeContext::new(Purpose::Generation, 1, [0x41; 16], [0x42; 16], 0).unwrap();
        assert!(matches!(
            key.seal_using(&context, b"SYNTHETIC", &mut FailingEntropy),
            Err(CryptoError::RandomnessUnavailable)
        ));
    }
    #[test]
    fn every_seal_requests_exact_nonce_size() {
        struct Recorded(Vec<usize>);
        impl Entropy for Recorded {
            fn fill(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
                self.0.push(out.len());
                out.fill(self.0.len() as u8);
                Ok(())
            }
        }
        let root = RootKey::generate().unwrap();
        let context =
            EnvelopeContext::new(Purpose::Generation, 1, [0x41; 16], [0x42; 16], 0).unwrap();
        let mut rng = Recorded(vec![]);
        let first = root.seal_using(&context, b"SYNTHETIC", &mut rng).unwrap();
        let second = root.seal_using(&context, b"SYNTHETIC", &mut rng).unwrap();
        assert_eq!(rng.0, [24, 24]);
        assert_ne!(&first.as_bytes()[22..46], &second.as_bytes()[22..46]);
        assert!(root.open(&context, &first).unwrap().expose() == b"SYNTHETIC");
    }
    #[test]
    fn independent_libsodium_envelope_vector_and_same_id_wrong_key() {
        // Independently produced with Python HKDF-SHA256 and libsodium XChaCha20-Poly1305.
        // All inputs are conspicuously synthetic. Production has no raw-key/RNG injection API.
        struct Fixed;
        impl Entropy for Fixed {
            fn fill(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
                out.fill(0x43);
                Ok(())
            }
        }
        let root = RootKey {
            secret: Zeroizing::new([0x53; 32]),
            id: [0x41; 16],
        };
        let ctx = EnvelopeContext::new(Purpose::Generation, 1, [0x41; 16], [0x42; 16], 0).unwrap();
        let sealed = root
            .seal_using(&ctx, b"CA03B_SYNTHETIC_NO_CREDENTIALS", &mut Fixed)
            .unwrap();
        let expected: &[u8] = &[
            67, 65, 69, 78, 1, 1, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65,
            67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67, 67,
            67, 67, 30, 0, 0, 0, 39, 156, 132, 184, 66, 56, 10, 93, 167, 138, 175, 61, 249, 201,
            144, 47, 153, 153, 212, 212, 93, 226, 248, 23, 198, 216, 67, 126, 119, 191, 0, 228,
            141, 18, 53, 245, 248, 226, 174, 148, 163, 120, 106, 154, 79, 199,
        ];
        assert!(sealed.as_bytes() == expected);
        let wrong = RootKey {
            secret: Zeroizing::new([0x54; 32]),
            id: root.id,
        };
        assert!(matches!(
            wrong.open(&ctx, &sealed),
            Err(CryptoError::AuthenticationFailed)
        ));
    }
}
