use codex_accounts_vault::*;

fn slot(n: u8) -> ResourceId {
    ResourceId::new(n).unwrap()
}
fn generation(n: u8) -> GenerationId {
    GenerationId::from_bytes([n; 16]).unwrap()
}
fn profile(n: u8) -> ProfileId {
    ProfileId::from_bytes([n; 16]).unwrap()
}
fn identity(subject: &str, workspace: &str) -> Identity {
    Identity::new("SYNTHETIC_ISSUER".into(), subject.into(), workspace.into()).unwrap()
}
fn one(bytes: &[u8]) -> Result<CredentialSet, DataError> {
    CredentialSet::new(
        &[(slot(0), ResourceShape::JsonObject, true)],
        vec![Resource::present(slot(0), bytes.to_vec())?],
    )
}

#[test]
fn exact_bytes_and_unknown_fields_survive_validation() {
    let bytes = b" \r\n{\"SYNTHETIC_TOKEN\" : \"never-real\", \"future\": [true, null, -3, 1.5, {\"x\":1}]}\n";
    let set = one(bytes).unwrap();
    assert_eq!(set.resource(slot(0)).unwrap().as_bytes(), Some(&bytes[..]));
}

#[test]
fn malformed_utf8_empty_truncated_scalar_and_trailing_data_are_rejected() {
    for bytes in [
        &b""[..],
        b"\xff",
        b"{",
        b"{}{}",
        b"{} garbage",
        b"[]",
        b"null",
        b"\"SYNTHETIC\"",
        b"{\"x\":NaN}",
        b"{\"x\":\"\x01\"}",
    ] {
        assert_eq!(one(bytes).unwrap_err(), DataError::InvalidJson);
    }
}

#[test]
fn duplicate_keys_at_every_level_are_rejected_after_decoding() {
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"nested":{"a":1,"a":2}}"#,
        r#"{"array":[{"a":1,"\u0061":2}]}"#,
        r#"{"é":1,"\u00e9":2}"#,
    ] {
        assert_eq!(one(input.as_bytes()).unwrap_err(), DataError::DuplicateKey);
    }
}

#[test]
fn independent_objects_can_reuse_a_key() {
    one(br#"{"a":{"key":1},"b":{"key":2}}"#).unwrap();
}

#[test]
fn depth_limit_counts_containers_and_does_not_disable_serde_limit() {
    let bounded = format!("{{\"x\":{}0{}}}", "[".repeat(63), "]".repeat(63));
    one(bounded.as_bytes()).unwrap();
    let deep = format!("{{\"x\":{}0{}}}", "[".repeat(64), "]".repeat(64));
    assert_eq!(one(deep.as_bytes()).unwrap_err(), DataError::DepthLimit);
}

#[test]
fn per_resource_exact_limit_and_one_extra() {
    let mut input = b"{\"x\":\"".to_vec();
    input.extend(vec![b'x'; MAX_RESOURCE_BYTES - 8]);
    input.extend_from_slice(b"\"}");
    assert_eq!(input.len(), MAX_RESOURCE_BYTES);
    one(&input).unwrap();
    input.push(b' ');
    assert_eq!(one(&input).unwrap_err(), DataError::InputLimit);
}

#[test]
fn full_set_limit_is_aggregate_not_just_per_resource() {
    let rules: Vec<_> = (0..4).map(|n| (slot(n), ResourceShape::Opaque, true)).collect();
    let resources = (0..4)
        .map(|n| Resource::present(slot(n), vec![b'x'; MAX_RESOURCE_BYTES]).unwrap())
        .collect();
    CredentialSet::new(&rules, resources).unwrap();
    let rules: Vec<_> = (0..5).map(|n| (slot(n), ResourceShape::Opaque, true)).collect();
    let resources = (0..5)
        .map(|n| {
            let size = if n == 4 { 1 } else { MAX_RESOURCE_BYTES };
            Resource::present(slot(n), vec![b'x'; size]).unwrap()
        })
        .collect();
    assert_eq!(CredentialSet::new(&rules, resources).unwrap_err(), DataError::InputLimit);
}

#[test]
fn absence_is_not_empty_and_optional_absence_is_preserved() {
    let set = CredentialSet::new(
        &[(slot(0), ResourceShape::Opaque, false), (slot(1), ResourceShape::Opaque, true)],
        vec![Resource::absent(slot(0)), Resource::present(slot(1), Vec::new()).unwrap()],
    ).unwrap();
    assert_eq!(set.resource(slot(0)).unwrap().as_bytes(), None);
    assert_eq!(set.resource(slot(1)).unwrap().as_bytes(), Some(&[][..]));
    assert!(set.resource(slot(2)).is_none());
}

#[test]
fn omitted_extra_duplicate_and_required_absent_resources_fail() {
    let rule = [(slot(0), ResourceShape::Opaque, true)];
    assert_eq!(CredentialSet::new(&rule, vec![]).unwrap_err(), DataError::ResourceSetMismatch);
    assert_eq!(CredentialSet::new(&rule, vec![Resource::absent(slot(1))]).unwrap_err(), DataError::ResourceSetMismatch);
    assert_eq!(CredentialSet::new(&rule, vec![Resource::absent(slot(0)), Resource::absent(slot(0))]).unwrap_err(), DataError::DuplicateResource);
    assert_eq!(CredentialSet::new(&rule, vec![Resource::absent(slot(0))]).unwrap_err(), DataError::RequiredResourceAbsent);
    assert_eq!(CredentialSet::new(&[rule[0], rule[0]], vec![]).unwrap_err(), DataError::DuplicateResource);
    assert_eq!(ResourceId::new(16), Err(DataError::InvalidId));
}

#[test]
fn raw_resource_cannot_bypass_set_json_validation() {
    let bad = Resource::present(slot(0), b"invalid SYNTHETIC".to_vec()).unwrap();
    assert_eq!(CredentialSet::new(&[(slot(0), ResourceShape::JsonObject, true)], vec![bad]).unwrap_err(), DataError::InvalidJson);
}

#[test]
fn identity_compares_issuer_subject_and_workspace_not_email() {
    let a = identity("SYNTHETIC_SUBJECT", "SYNTHETIC_WORKSPACE");
    assert_eq!(a, identity("SYNTHETIC_SUBJECT", "SYNTHETIC_WORKSPACE"));
    assert_ne!(a, identity("OTHER_SUBJECT", "SYNTHETIC_WORKSPACE"));
    assert_ne!(a, identity("SYNTHETIC_SUBJECT", "OTHER_WORKSPACE"));
    assert_ne!(a, Identity::new("OTHER_ISSUER".into(), "SYNTHETIC_SUBJECT".into(), "SYNTHETIC_WORKSPACE".into()).unwrap());
    // Email is deliberately not part of this comparison model or constructor.
}

#[test]
fn empty_ambiguous_and_unbounded_identity_is_rejected() {
    for value in ["".to_owned(), " x".into(), "x ".into(), "x\n".into(), "x".repeat(2049)] {
        assert_eq!(Identity::new("SYNTHETIC".into(), value, "SYNTHETIC".into()).unwrap_err(), DataError::InvalidIdentity);
    }
    assert_eq!(ProfileId::from_bytes([0; 16]), Err(DataError::InvalidId));
    assert_eq!(GenerationId::from_bytes([0; 16]), Err(DataError::InvalidId));
}

#[test]
fn newest_generation_is_not_the_original_snapshot() {
    let who = identity("SYNTHETIC_A", "SYNTHETIC_SPACE");
    let mut index = GenerationIndex::new(profile(1), identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1));
    index.append(profile(1), &who, generation(1), generation(2)).unwrap();
    assert_eq!(index.latest(), generation(2));
    assert_eq!(index.append(profile(1), &who, generation(1), generation(3)), Err(DataError::StaleParent));
    assert_eq!(index.latest(), generation(2));
}

#[test]
fn mismatched_profile_or_identity_cannot_replace_a_generation() {
    let mut index = GenerationIndex::new(profile(1), identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1));
    assert_eq!(index.append(profile(2), &identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1), generation(2)), Err(DataError::IdentityMismatch));
    assert_eq!(index.append(profile(1), &identity("SYNTHETIC_A", "OTHER_SPACE"), generation(1), generation(2)), Err(DataError::IdentityMismatch));
    assert_eq!(index.latest(), generation(1));
}

#[test]
fn generation_identifiers_cannot_be_reused() {
    let who = identity("SYNTHETIC_A", "SYNTHETIC_SPACE");
    let mut index = GenerationIndex::new(profile(1), identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1));
    assert_eq!(index.append(profile(1), &who, generation(1), generation(1)), Err(DataError::DuplicateGeneration));
    assert_eq!(index.latest(), generation(1));
}

#[test]
fn retention_decision_keeps_latest_and_all_supplied_unresolved_references() {
    let who = identity("SYNTHETIC_A", "SYNTHETIC_SPACE");
    let mut index = GenerationIndex::new(profile(1), identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1));
    index.append(profile(1), &who, generation(1), generation(2)).unwrap();
    index.append(profile(1), &who, generation(2), generation(3)).unwrap();
    assert_eq!(index.unreferenced_candidates(&[generation(1), generation(1)]).unwrap(), vec![generation(2)]);
    assert!(index.unreferenced_candidates(&[generation(1), generation(2)]).unwrap().is_empty());
    assert_eq!(index.unreferenced_candidates(&[generation(9)]), Err(DataError::UnknownGeneration));
    assert_eq!(index.latest(), generation(3));
}

#[test]
fn history_bound_refuses_growth_without_evicting_recovery_state() {
    let who = identity("SYNTHETIC_A", "SYNTHETIC_SPACE");
    let mut index = GenerationIndex::new(profile(1), identity("SYNTHETIC_A", "SYNTHETIC_SPACE"), generation(1));
    for n in 2..=MAX_GENERATIONS as u8 {
        index.append(profile(1), &who, generation(n - 1), generation(n)).unwrap();
    }
    assert_eq!(index.append(profile(1), &who, generation(128), generation(129)), Err(DataError::InputLimit));
    assert_eq!(index.latest(), generation(128));
    assert_eq!(index.unreferenced_candidates(&vec![generation(1); MAX_GENERATIONS + 1]), Err(DataError::InputLimit));
}

#[test]
fn secrets_do_not_appear_in_debug_or_error_categories() {
    let canary = "SYNTHETIC_SECRET_CANARY";
    let who = identity(canary, canary);
    let set = one(format!("{{\"token\":\"{canary}\"}}").as_bytes()).unwrap();
    let index = GenerationIndex::new(profile(1), who, generation(1));
    let text = format!("{set:?} {:?} {index:?}", set.resource(slot(0)).unwrap());
    assert!(!text.contains(canary));
    let error = one(format!("{{\"{canary}\":1,\"{canary}\":2}}").as_bytes()).unwrap_err();
    assert!(!format!("{error} {error:?}").contains(canary));
}
