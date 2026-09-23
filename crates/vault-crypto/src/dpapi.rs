//! Only application-authored unsafe boundary in this crate. See the scoped review.
use crate::{CryptoError, ProtectedRootKey, RootKey};
use hkdf::hmac::{Hmac, Mac};
use sha2::Sha256;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8; 8] = b"CAROOT01";
const ENTROPY: &[u8] = b"codex-accounts/current-user-root/v1";
const CHECK_INFO: &[u8] = b"codex-accounts/v1/root-integrity/hmac-sha256";
const INNER_BYTES: usize = 88;

// Own exactly one OS allocation; no Clone, Debug or public pointer access.
struct NativeBlob(CRYPT_INTEGER_BLOB);
impl Drop for NativeBlob {
    fn drop(&mut self) {
        if !self.0.pbData.is_null() {
            // SAFETY: DPAPI allocated cbData initialized bytes at pbData. This
            // owner is created immediately after the call, never aliases a Rust
            // allocation, and outlives all borrows. OS allocator metadata is trusted.
            unsafe {
                std::slice::from_raw_parts_mut(self.0.pbData, self.0.cbData as usize).zeroize();
                LocalFree(self.0.pbData.cast());
            }
        }
    }
}
fn transform(data: &[u8], encrypt: bool) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if data.is_empty() || data.len() > 4096 {
        return Err(CryptoError::InputLimit);
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr().cast_mut(),
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: ENTROPY.len() as u32,
        pbData: ENTROPY.as_ptr().cast_mut(),
    };
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    // SAFETY: bounded live input slices are documented [in] parameters. No input
    // is mutated by these APIs. Output begins empty. No prompt or description
    // allocation is requested. Only UI_FORBIDDEN is passed, never machine scope.
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                null(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        } else {
            CryptUnprotectData(
                &input,
                null_mut(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        }
    };
    let owned = NativeBlob(out);
    if success == 0 || owned.0.pbData.is_null() || owned.0.cbData == 0 || owned.0.cbData > 4096 {
        return Err(CryptoError::KeyProtectionUnavailable);
    }
    // SAFETY: successful DPAPI returned cbData initialized bytes, bounded above.
    // The slice cannot escape NativeBlob; the owned copy is zeroizing as well.
    Ok(Zeroizing::new(unsafe {
        std::slice::from_raw_parts(owned.0.pbData, owned.0.cbData as usize).to_vec()
    }))
}
fn integrity(key: &RootKey, data: &[u8]) -> Result<Hmac<Sha256>, CryptoError> {
    let derived = key.derive(CHECK_INFO)?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(derived.as_ref())
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    mac.update(data);
    Ok(mac)
}
pub(crate) fn protect(key: &RootKey) -> Result<ProtectedRootKey, CryptoError> {
    let mut inner = Zeroizing::new(Vec::with_capacity(INNER_BYTES));
    inner.extend_from_slice(MAGIC);
    inner.extend_from_slice(&key.id);
    inner.extend_from_slice(key.secret.as_ref());
    let mut tag = integrity(key, &inner)?.finalize().into_bytes();
    inner.extend_from_slice(&tag);
    tag.as_mut_slice().zeroize();
    let blob = transform(&inner, true)?;
    ProtectedRootKey::wrap(&blob)
}
pub(crate) fn unprotect(key: &ProtectedRootKey) -> Result<RootKey, CryptoError> {
    let inner = transform(key.blob(), false)?;
    if inner.len() != INNER_BYTES || &inner[..8] != MAGIC {
        return Err(CryptoError::KeyProtectionUnavailable);
    }
    let mut secret = Zeroizing::new([0; 32]);
    secret.copy_from_slice(&inner[24..56]);
    let id: [u8; 16] = inner[8..24]
        .try_into()
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    if *secret == [0; 32] || id == [0; 16] {
        return Err(CryptoError::KeyProtectionUnavailable);
    }
    let root = RootKey { secret, id };
    // Validate the whole recovered structure, even if DPAPI returns damaged data
    // without an error. This is supplemental validation, not a DPAPI replacement.
    integrity(&root, &inner[..56])?
        .verify_slice(&inner[56..])
        .map_err(|_| CryptoError::KeyProtectionUnavailable)?;
    Ok(root)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_dpapi_roundtrip_keeps_root_and_id() {
        let root = RootKey::generate().unwrap();
        let protected = protect(&root).unwrap();
        let restored = unprotect(&protected).unwrap();
        assert!(*restored.secret == *root.secret && restored.id == root.id);
    }
    #[test]
    fn native_dpapi_corruption_never_becomes_a_key() {
        let root = RootKey::generate().unwrap();
        let protected = protect(&root).unwrap();
        for index in [
            10,
            protected.as_bytes().len() / 2,
            protected.as_bytes().len() - 1,
        ] {
            let mut bytes = protected.as_bytes().to_vec();
            bytes[index] ^= 0x80;
            let bad = ProtectedRootKey::from_bytes(&bytes).unwrap();
            assert!(unprotect(&bad).is_err());
        }
    }
    #[test]
    fn native_dpapi_rejects_wrong_inner_shape_and_integrity() {
        for len in [1, INNER_BYTES - 1, INNER_BYTES, INNER_BYTES + 1] {
            let mut data = Zeroizing::new(vec![0x53; len]);
            if len >= 8 {
                data[..8].copy_from_slice(MAGIC);
            }
            let blob = transform(&data, true).unwrap();
            assert!(unprotect(&ProtectedRootKey::wrap(&blob).unwrap()).is_err());
        }
    }
    #[test]
    fn native_dpapi_unavailable_or_invalid_never_falls_back() {
        assert!(transform(b"SYNTHETIC NOT A DPAPI BLOB", false).is_err());
        assert!(matches!(
            transform(&[], false),
            Err(CryptoError::InputLimit)
        ));
        assert!(matches!(
            transform(&vec![0; 4097], false),
            Err(CryptoError::InputLimit)
        ));
    }
}
