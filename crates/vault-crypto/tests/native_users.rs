//! Explicit owner-run, two-existing-user proof. Never part of a shipped CLI.
//! The only persistent output is a named, encrypted SYNTHETIC test fixture.
#![cfg(all(windows, target_arch = "x86_64"))]
use codex_accounts_vault_crypto::{CryptoError, Envelope, EnvelopeContext, ProtectedRootKey, Purpose, RootKey};
use std::io::{Read, Write};

const FILE: &str = "ca03b-synthetic.dpapi";
const MARKER: &[u8] = b"CA03B-SYNTHETIC-FIXTURE-V1\0";
const MESSAGE: &[u8] = b"SYNTHETIC_ONLY_NO_ACCOUNT_OR_CREDENTIALS";
fn context() -> EnvelopeContext {
    EnvelopeContext::new(Purpose::Generation, 1, [0x41; 16], [0x42; 16], 0).unwrap()
}
fn read_fixture() -> (ProtectedRootKey, Envelope) {
    let metadata = std::fs::symlink_metadata(FILE).unwrap_or_else(|_| panic!("synthetic fixture metadata unavailable"));
    assert!(metadata.file_type().is_file() && metadata.len() <= 8192, "invalid synthetic fixture file");
    let mut bytes = Vec::new();
    std::fs::File::open(FILE).unwrap_or_else(|_| panic!("synthetic fixture unavailable"))
        .take(8193).read_to_end(&mut bytes).unwrap_or_else(|_| panic!("synthetic fixture read failed"));
    assert!(bytes.len() <= 8192 && bytes.len() >= MARKER.len() + 4, "invalid synthetic fixture size");
    assert!(bytes.starts_with(MARKER), "not a CA-03B synthetic fixture");
    let offset = MARKER.len();
    let size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    let start = offset + 4;
    assert!(size > 0 && size <= 4096 && start + size < bytes.len(), "invalid synthetic fixture structure");
    let key = ProtectedRootKey::from_bytes(&bytes[start..start + size]).unwrap_or_else(|_| panic!("invalid protected synthetic key"));
    let envelope = Envelope::from_bytes(&bytes[start + size..]).unwrap_or_else(|_| panic!("invalid synthetic envelope"));
    (key, envelope)
}

#[test]
#[ignore = "owner-run only: creates a synthetic fixture; see docs/ca-03b-crypto.md"]
fn native_two_user_1_create_fixture() {
    let root = RootKey::generate().unwrap();
    let protected = root.protect().unwrap();
    let sealed = root.seal(&context(), MESSAGE).unwrap();
    let reopened = RootKey::unprotect(&protected).unwrap();
    assert!(reopened.open(&context(), &sealed).unwrap().expose() == MESSAGE);
    // Never overwrite a fixture or write a plaintext key/credential.
    let mut file = std::fs::OpenOptions::new().create_new(true).write(true)
        .open(FILE).unwrap_or_else(|_| panic!("create a new synthetic fixture only"));
    file.write_all(MARKER).unwrap_or_else(|_| panic!("synthetic fixture write failed"));
    file.write_all(&(protected.as_bytes().len() as u32).to_le_bytes()).unwrap_or_else(|_| panic!("synthetic fixture write failed"));
    file.write_all(protected.as_bytes()).unwrap_or_else(|_| panic!("synthetic fixture write failed"));
    file.write_all(sealed.as_bytes()).unwrap_or_else(|_| panic!("synthetic fixture write failed"));
    file.sync_all().unwrap_or_else(|_| panic!("synthetic fixture flush failed"));
}

#[test]
#[ignore = "owner-run creator control on the unchanged synthetic fixture"]
fn native_two_user_2_creator_control() {
    let (protected, sealed) = read_fixture();
    let root = RootKey::unprotect(&protected).unwrap_or_else(|_| panic!("creator must unlock the same synthetic fixture"));
    assert!(root.open(&context(), &sealed).unwrap().expose() == MESSAGE);
}

#[test]
#[ignore = "owner-run under a DIFFERENT existing ordinary OS user; unchanged fixture and creator controls required"]
fn native_two_user_3_other_user_refuses() {
    let (protected, _) = read_fixture();
    assert!(matches!(RootKey::unprotect(&protected), Err(CryptoError::KeyProtectionUnavailable)),
        "different ordinary user must not recover the synthetic key");
}
