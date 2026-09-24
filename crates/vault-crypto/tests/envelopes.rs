use codex_accounts_vault_crypto::{
    CryptoError, Envelope, EnvelopeContext, ProtectedRootKey, Purpose, RootKey, MAX_ENVELOPE_BYTES,
    MAX_PLAINTEXT_BYTES,
};

fn context(purpose: Purpose) -> EnvelopeContext {
    EnvelopeContext::new(purpose, 1, [0x41; 16], [0x42; 16], 0).unwrap()
}

#[test]
fn opaque_bytes_roundtrip_for_every_purpose() {
    let root = RootKey::generate().unwrap();
    let plaintext = b"SYNTHETIC_ONLY\r\n\0\xff unknown fields are not normalized";
    for purpose in [
        Purpose::Registry,
        Purpose::Identity,
        Purpose::Generation,
        Purpose::Journal,
        Purpose::CredentialResource,
    ] {
        let ctx = context(purpose);
        let sealed = root.seal(&ctx, plaintext).unwrap();
        let parsed = Envelope::from_bytes(sealed.as_bytes()).unwrap();
        assert!(root.open(&ctx, &parsed).unwrap().expose() == plaintext);
        assert!(!sealed
            .as_bytes()
            .windows(plaintext.len())
            .any(|w| w == plaintext));
    }
}

#[test]
fn empty_plaintext_has_an_authenticated_envelope() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let sealed = root.seal(&ctx, &[]).unwrap();
    assert_eq!(sealed.as_bytes().len(), 66);
    assert!(root.open(&ctx, &sealed).unwrap().expose().is_empty());
}

#[test]
fn every_context_field_and_purpose_is_authenticated() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::CredentialResource);
    let sealed = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    let wrong = [
        context(Purpose::Identity),
        context(Purpose::Registry),
        context(Purpose::Generation),
        context(Purpose::Journal),
        EnvelopeContext::new(Purpose::CredentialResource, 2, [0x41; 16], [0x42; 16], 0).unwrap(),
        EnvelopeContext::new(Purpose::CredentialResource, 1, [0x43; 16], [0x42; 16], 0).unwrap(),
        EnvelopeContext::new(Purpose::CredentialResource, 1, [0x41; 16], [0x43; 16], 0).unwrap(),
        EnvelopeContext::new(Purpose::CredentialResource, 1, [0x41; 16], [0x42; 16], 1).unwrap(),
    ];
    for other in wrong {
        assert!(matches!(
            root.open(&other, &sealed),
            Err(CryptoError::AuthenticationFailed)
        ));
    }
}

#[test]
fn every_single_byte_mutation_is_rejected() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let sealed = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    for index in 0..sealed.as_bytes().len() {
        let mut bytes = sealed.as_bytes().to_vec();
        bytes[index] ^= 1;
        if let Ok(bad) = Envelope::from_bytes(&bytes) {
            assert!(root.open(&ctx, &bad).is_err(), "corrupt envelope accepted");
        }
    }
    // Failure does not invalidate or mutate the original object.
    assert!(root.open(&ctx, &sealed).unwrap().expose() == b"SYNTHETIC_ONLY");
}

#[test]
fn truncated_extended_overflowing_or_unknown_format_is_refused() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let sealed = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    for size in 0..sealed.as_bytes().len() {
        assert!(Envelope::from_bytes(&sealed.as_bytes()[..size]).is_err());
    }
    let mut bytes = sealed.as_bytes().to_vec();
    bytes.push(0);
    assert!(Envelope::from_bytes(&bytes).is_err());
    for (offset, expected) in [
        (4, CryptoError::UnsupportedVersion),
        (5, CryptoError::UnsupportedAlgorithm),
    ] {
        let mut bytes = sealed.as_bytes().to_vec();
        bytes[offset] = 255;
        assert!(matches!(Envelope::from_bytes(&bytes), Err(e) if e == expected));
    }
    let mut bytes = sealed.as_bytes().to_vec();
    bytes[46..50].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Envelope::from_bytes(&bytes).is_err());
    assert!(matches!(
        Envelope::from_bytes(&vec![0; MAX_ENVELOPE_BYTES + 1]),
        Err(CryptoError::InputLimit)
    ));
}

#[test]
fn wrong_root_key_and_stale_context_never_decrypt() {
    let first = RootKey::generate().unwrap();
    let second = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let sealed = first.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    assert!(matches!(
        second.open(&ctx, &sealed),
        Err(CryptoError::AuthenticationFailed)
    ));
    let later = EnvelopeContext::new(Purpose::Generation, 1, [0x41; 16], [0x44; 16], 0).unwrap();
    assert!(matches!(
        first.open(&later, &sealed),
        Err(CryptoError::AuthenticationFailed)
    ));
    // Deliberately: AEAD does not prove freshness if the caller selects the old context.
    assert!(first.open(&ctx, &sealed).is_ok());
}

#[test]
fn os_nonces_are_fresh_and_have_the_declared_length() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let first = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    let second = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    assert_eq!(&first.as_bytes()[..6], b"CAEN\x01\x01");
    assert_ne!(&first.as_bytes()[22..46], &second.as_bytes()[22..46]);
}

#[test]
fn exact_limits_succeed_and_one_extra_fails_before_encryption() {
    let root = RootKey::generate().unwrap();
    for (purpose, limit) in [
        (Purpose::CredentialResource, 1024 * 1024),
        (Purpose::Generation, MAX_PLAINTEXT_BYTES),
    ] {
        let ctx = context(purpose);
        let bytes = vec![0x53; limit];
        let sealed = root.seal(&ctx, &bytes).unwrap();
        assert!(root.open(&ctx, &sealed).unwrap().expose() == bytes);
        assert!(matches!(
            root.seal(&ctx, &vec![0x53; limit + 1]),
            Err(CryptoError::InputLimit)
        ));
    }
    let oversized_resource = root
        .seal(&context(Purpose::Generation), &vec![0x53; 1024 * 1024 + 1])
        .unwrap();
    assert!(matches!(
        root.open(&context(Purpose::CredentialResource), &oversized_resource),
        Err(CryptoError::InputLimit)
    ));
}

#[test]
fn fingerprints_bind_presence_bytes_context_and_key() {
    let root = RootKey::generate().unwrap();
    let other = RootKey::generate().unwrap();
    let ctx = context(Purpose::CredentialResource);
    let absent = root.fingerprint(&ctx, None).unwrap();
    let present = root.fingerprint(&ctx, Some(b"SYNTHETIC_ONLY")).unwrap();
    assert!(root.verify_fingerprint(&absent, &ctx, None).is_ok());
    assert!(root.verify_fingerprint(&absent, &ctx, Some(&[])).is_err());
    assert!(root
        .verify_fingerprint(&present, &ctx, Some(b"SYNTHETIC_ONLY"))
        .is_ok());
    assert!(root
        .verify_fingerprint(&present, &ctx, Some(b"SYNTHETIC_OTHER"))
        .is_err());
    assert!(root
        .verify_fingerprint(
            &present,
            &context(Purpose::Identity),
            Some(b"SYNTHETIC_ONLY")
        )
        .is_err());
    assert!(other
        .verify_fingerprint(&present, &ctx, Some(b"SYNTHETIC_ONLY"))
        .is_err());
    assert!(matches!(
        root.fingerprint(&ctx, Some(&vec![0; 1024 * 1024 + 1])),
        Err(CryptoError::InputLimit)
    ));
}

#[test]
fn ambiguous_context_and_unbounded_protected_key_inputs_fail() {
    for (schema, profile, generation, slot) in [
        (0, [1; 16], [2; 16], 0),
        (1, [0; 16], [2; 16], 0),
        (1, [1; 16], [0; 16], 0),
        (1, [1; 16], [2; 16], 16),
    ] {
        assert!(matches!(
            EnvelopeContext::new(
                Purpose::CredentialResource,
                schema,
                profile,
                generation,
                slot
            ),
            Err(CryptoError::InvalidContext)
        ));
    }
    assert!(EnvelopeContext::new(Purpose::Registry, 1, [1; 16], [2; 16], 1).is_err());
    for size in 0..11 {
        assert!(ProtectedRootKey::from_bytes(&vec![0; size]).is_err());
    }
    assert!(matches!(
        ProtectedRootKey::from_bytes(&vec![0; 4097]),
        Err(CryptoError::InputLimit)
    ));
    let base = b"CAKP\x01\x01\x01\x00\x00\x00S";
    for (offset, expected) in [
        (4, CryptoError::UnsupportedVersion),
        (5, CryptoError::UnsupportedAlgorithm),
    ] {
        let mut bytes = base.to_vec();
        bytes[offset] = 255;
        assert!(matches!(ProtectedRootKey::from_bytes(&bytes), Err(e) if e == expected));
    }
    let mut bytes = base.to_vec();
    bytes[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(ProtectedRootKey::from_bytes(&bytes).is_err());
}

#[test]
fn secret_types_and_errors_only_emit_fixed_debug_categories() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::CredentialResource);
    let bytes = b"CA03B_SYNTHETIC_SECRET_CANARY_NOT_REAL_AUTH";
    let sealed = root.seal(&ctx, bytes).unwrap();
    let opened = root.open(&ctx, &sealed).unwrap();
    let fingerprint = root.fingerprint(&ctx, Some(bytes)).unwrap();
    assert_eq!(format!("{root:?}"), "RootKey([REDACTED])");
    assert_eq!(format!("{ctx:?}"), "EnvelopeContext([REDACTED])");
    assert_eq!(format!("{sealed:?}"), "Envelope([REDACTED])");
    assert_eq!(format!("{opened:?}"), "SecretBytes([REDACTED])");
    assert_eq!(format!("{fingerprint:?}"), "Fingerprint([REDACTED])");
    let error = CryptoError::AuthenticationFailed;
    assert_eq!(format!("{error}"), "encrypted data authentication failed");
    assert_eq!(format!("{error:?}"), "AuthenticationFailed");
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn protected_root_reopens_the_same_envelope_without_plaintext_fallback() {
    let root = RootKey::generate().unwrap();
    let ctx = context(Purpose::Generation);
    let sealed = root.seal(&ctx, b"SYNTHETIC_ONLY").unwrap();
    let wrapped = root.protect().unwrap();
    assert_eq!(format!("{wrapped:?}"), "ProtectedRootKey([REDACTED])");
    let parsed = ProtectedRootKey::from_bytes(wrapped.as_bytes()).unwrap();
    let reopened = RootKey::unprotect(&parsed).unwrap();
    assert!(reopened.open(&ctx, &sealed).unwrap().expose() == b"SYNTHETIC_ONLY");
}

#[cfg(not(all(windows, target_arch = "x86_64")))]
#[test]
fn other_hosts_do_not_get_a_plaintext_or_native_store_fallback() {
    let root = RootKey::generate().unwrap();
    assert!(matches!(
        root.protect(),
        Err(CryptoError::UnsupportedPlatform)
    ));
    let wrapped = ProtectedRootKey::from_bytes(b"CAKP\x01\x01\x01\x00\x00\x00S").unwrap();
    assert!(matches!(
        RootKey::unprotect(&wrapped),
        Err(CryptoError::UnsupportedPlatform)
    ));
}
