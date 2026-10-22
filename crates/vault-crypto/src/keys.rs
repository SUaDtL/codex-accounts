#![forbid(unsafe_code)]
use crate::random::{self, Entropy, SystemEntropy};
use crate::{CryptoError, EnvelopeContext};
#[cfg(any(test, all(windows, target_arch = "x86_64")))]
use argon2::{Algorithm, Argon2, Block, Params, Version};
#[cfg(any(test, all(windows, target_arch = "x86_64")))]
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    Key, Tag, XChaCha20Poly1305, XNonce,
};
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
const HARDENED_HEADER_BYTES: usize = 62;
const HARDENED_TAG_BYTES: usize = 16;
const HARDENED_M_COST: u32 = 64 * 1024;
const HARDENED_T_COST: u32 = 3;
const HARDENED_P_COST: u32 = 1;
#[cfg(any(test, all(windows, target_arch = "x86_64")))]
const MIN_PASSPHRASE_BYTES: usize = 12;
#[cfg(any(test, all(windows, target_arch = "x86_64")))]
const MAX_PASSPHRASE_BYTES: usize = 1024;
#[cfg(any(test, all(windows, target_arch = "x86_64")))]
const HARDENED_AAD: &[u8] = b"codex-accounts/protected-root/v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootProtection {
    WindowsUser,
    Passphrase,
}

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
    pub fn protect_with_passphrase(
        &self,
        passphrase: &[u8],
    ) -> Result<ProtectedRootKey, CryptoError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            crate::dpapi::protect(self)?.harden(passphrase)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            let _ = passphrase;
            Err(CryptoError::UnsupportedPlatform)
        }
    }
    pub fn unprotect_with_passphrase(
        protected: &ProtectedRootKey,
        passphrase: &[u8],
    ) -> Result<Self, CryptoError> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        {
            crate::dpapi::unprotect(&protected.unharden(passphrase)?)
        }
        #[cfg(not(all(windows, target_arch = "x86_64")))]
        {
            let _ = (protected, passphrase);
            Err(CryptoError::UnsupportedPlatform)
        }
    }
}

pub struct Fingerprint(Zeroizing<[u8; 32]>);
impl Fingerprint {
    /// Fixed-size storage encoding. The caller must encrypt this value; it is
    /// never suitable for UI, diagnostics, logs, or a portable credential export.
    pub fn storage_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    /// Parse only a tag, not trusted evidence. Verify against independently
    /// selected context and bytes before using a recovered fingerprint.
    pub fn from_storage_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidEnvelope);
        }
        let mut tag = Zeroizing::new([0; 32]);
        tag.copy_from_slice(bytes);
        Ok(Self(tag))
    }
}

/// Versioned ciphertext, not proof of its origin, user scope, or storage safety.
pub struct ProtectedRootKey(Vec<u8>);
impl ProtectedRootKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() > MAX_PROTECTED_BYTES {
            return Err(CryptoError::InputLimit);
        }
        if bytes.len() < 10 || &bytes[..4] != PROTECTED_MAGIC {
            return Err(CryptoError::InvalidEnvelope);
        }
        if bytes[4] != 1 {
            return Err(CryptoError::UnsupportedVersion);
        }
        match bytes[5] {
            1 => {
                let len = encoded_len(&bytes[6..10])?;
                if len == 0 || len != bytes.len() - 10 {
                    return Err(CryptoError::InvalidEnvelope);
                }
            }
            2 => {
                if bytes.len() < HARDENED_HEADER_BYTES + HARDENED_TAG_BYTES
                    || encoded_u32(&bytes[6..10])? != HARDENED_M_COST
                    || encoded_u32(&bytes[10..14])? != HARDENED_T_COST
                    || encoded_u32(&bytes[14..18])? != HARDENED_P_COST
                    || bytes[18..34] == [0; 16]
                    || bytes[34..58] == [0; 24]
                {
                    return Err(CryptoError::InvalidEnvelope);
                }
                let len = encoded_len(&bytes[58..62])?;
                if len == 0 || len != bytes.len() - HARDENED_HEADER_BYTES - HARDENED_TAG_BYTES {
                    return Err(CryptoError::InvalidEnvelope);
                }
            }
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        }
        Ok(Self(bytes.to_vec()))
    }
    pub fn protection(&self) -> RootProtection {
        if self.0[5] == 2 {
            RootProtection::Passphrase
        } else {
            RootProtection::WindowsUser
        }
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

    #[cfg(all(windows, target_arch = "x86_64"))]
    fn harden(&self, passphrase: &[u8]) -> Result<Self, CryptoError> {
        self.harden_using(passphrase, &mut SystemEntropy)
    }

    #[cfg(any(test, all(windows, target_arch = "x86_64")))]
    fn harden_using(&self, passphrase: &[u8], rng: &mut impl Entropy) -> Result<Self, CryptoError> {
        if self.protection() != RootProtection::WindowsUser {
            return Err(CryptoError::UnsupportedAlgorithm);
        }
        check_passphrase(passphrase)?;
        let mut salt = [0; 16];
        let mut nonce = [0; 24];
        random::fill(rng, &mut salt)?;
        random::fill(rng, &mut nonce)?;
        let mut header = [0; HARDENED_HEADER_BYTES];
        header[..4].copy_from_slice(PROTECTED_MAGIC);
        header[4..6].copy_from_slice(&[1, 2]);
        header[6..10].copy_from_slice(&HARDENED_M_COST.to_le_bytes());
        header[10..14].copy_from_slice(&HARDENED_T_COST.to_le_bytes());
        header[14..18].copy_from_slice(&HARDENED_P_COST.to_le_bytes());
        header[18..34].copy_from_slice(&salt);
        header[34..58].copy_from_slice(&nonce);
        header[58..62].copy_from_slice(&(self.0.len() as u32).to_le_bytes());
        let key = hardened_key(passphrase, &salt)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
        let mut data = Zeroizing::new(self.0.clone());
        let tag = cipher
            .encrypt_in_place_detached(
                XNonce::from_slice(&nonce),
                &[HARDENED_AAD, header.as_slice()].concat(),
                &mut data,
            )
            .map_err(|_| CryptoError::AuthenticationFailed)?;
        let mut bytes = Vec::with_capacity(header.len() + data.len() + tag.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(&tag);
        Self::from_bytes(&bytes)
    }

    #[cfg(any(test, all(windows, target_arch = "x86_64")))]
    fn unharden(&self, passphrase: &[u8]) -> Result<Self, CryptoError> {
        if self.protection() != RootProtection::Passphrase {
            return Err(CryptoError::UnsupportedAlgorithm);
        }
        check_passphrase(passphrase)?;
        let len = encoded_len(&self.0[58..62])?;
        let key = hardened_key(passphrase, &self.0[18..34])?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
        let mut data =
            Zeroizing::new(self.0[HARDENED_HEADER_BYTES..HARDENED_HEADER_BYTES + len].to_vec());
        cipher
            .decrypt_in_place_detached(
                XNonce::from_slice(&self.0[34..58]),
                &[HARDENED_AAD, &self.0[..HARDENED_HEADER_BYTES]].concat(),
                &mut data,
                Tag::from_slice(&self.0[HARDENED_HEADER_BYTES + len..]),
            )
            .map_err(|_| CryptoError::AuthenticationFailed)?;
        let inner = Self::from_bytes(&data)?;
        if inner.protection() != RootProtection::WindowsUser {
            return Err(CryptoError::InvalidEnvelope);
        }
        Ok(inner)
    }
}

fn encoded_u32(bytes: &[u8]) -> Result<u32, CryptoError> {
    Ok(u32::from_le_bytes(
        bytes.try_into().map_err(|_| CryptoError::InvalidEnvelope)?,
    ))
}

fn encoded_len(bytes: &[u8]) -> Result<usize, CryptoError> {
    Ok(encoded_u32(bytes)? as usize)
}

#[cfg(any(test, all(windows, target_arch = "x86_64")))]
fn check_passphrase(passphrase: &[u8]) -> Result<(), CryptoError> {
    if !(MIN_PASSPHRASE_BYTES..=MAX_PASSPHRASE_BYTES).contains(&passphrase.len()) {
        return Err(CryptoError::InputLimit);
    }
    Ok(())
}

#[cfg(any(test, all(windows, target_arch = "x86_64")))]
fn hardened_key(passphrase: &[u8], salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let params = Params::new(HARDENED_M_COST, HARDENED_T_COST, HARDENED_P_COST, Some(32))
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(HARDENED_M_COST as usize)
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    memory.resize(HARDENED_M_COST as usize, Block::default());
    let mut blocks = Zeroizing::new(memory);
    let mut key = Zeroizing::new([0; 32]);
    argon
        .hash_password_into_with_memory(passphrase, salt, key.as_mut(), blocks.as_mut_slice())
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    Ok(key)
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
    #[test]
    fn hardened_wrapper_requires_the_same_strong_passphrase() {
        struct Recorded(u8);
        impl Entropy for Recorded {
            fn fill(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
                self.0 = self.0.wrapping_add(1);
                out.fill(self.0);
                Ok(())
            }
        }
        let inner = ProtectedRootKey::from_bytes(&[b'C', b'A', b'K', b'P', 1, 1, 1, 0, 0, 0, 0x53])
            .unwrap();
        let hardened = inner
            .harden_using(b"correct horse battery staple", &mut Recorded(0))
            .unwrap();
        assert_eq!(hardened.protection(), RootProtection::Passphrase);
        assert_eq!(
            hardened
                .unharden(b"correct horse battery staple")
                .unwrap()
                .as_bytes(),
            inner.as_bytes()
        );
        assert!(matches!(
            hardened.unharden(b"wrong passphrase but long enough"),
            Err(CryptoError::AuthenticationFailed)
        ));
    }

    #[test]
    fn hardened_wrapper_rejects_parameter_corruption_and_authenticated_mutations() {
        struct Recorded(u8);
        impl Entropy for Recorded {
            fn fill(&mut self, out: &mut [u8]) -> Result<(), CryptoError> {
                self.0 = self.0.wrapping_add(1);
                out.fill(self.0);
                Ok(())
            }
        }
        let passphrase = b"correct horse battery staple";
        let inner = ProtectedRootKey::from_bytes(&[b'C', b'A', b'K', b'P', 1, 1, 1, 0, 0, 0, 0x53])
            .unwrap();
        let hardened = inner.harden_using(passphrase, &mut Recorded(0)).unwrap();

        for index in [6, 10, 14] {
            let mut changed = hardened.as_bytes().to_vec();
            changed[index] ^= 1;
            assert!(matches!(
                ProtectedRootKey::from_bytes(&changed),
                Err(CryptoError::InvalidEnvelope)
            ));
        }
        for index in [18, 34, HARDENED_HEADER_BYTES, hardened.as_bytes().len() - 1] {
            let mut changed = hardened.as_bytes().to_vec();
            changed[index] ^= 1;
            let changed = ProtectedRootKey::from_bytes(&changed).unwrap();
            assert!(matches!(
                changed.unharden(passphrase),
                Err(CryptoError::AuthenticationFailed)
            ));
        }
        assert!(check_passphrase(&[b'a'; MIN_PASSPHRASE_BYTES]).is_ok());
        assert!(check_passphrase(&[b'a'; MAX_PASSPHRASE_BYTES]).is_ok());
        assert!(matches!(
            check_passphrase(&[b'a'; MIN_PASSPHRASE_BYTES - 1]),
            Err(CryptoError::InputLimit)
        ));
        assert!(matches!(
            check_passphrase(&[b'a'; MAX_PASSPHRASE_BYTES + 1]),
            Err(CryptoError::InputLimit)
        ));
    }
}
