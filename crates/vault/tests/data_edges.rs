use codex_accounts_vault::*;

fn slot(n: u8) -> ResourceId {
    ResourceId::new(n).unwrap()
}

fn generation(n: u8) -> GenerationId {
    GenerationId::from_bytes([n; 16]).unwrap()
}

fn profile() -> ProfileId {
    ProfileId::from_bytes([1; 16]).unwrap()
}

fn identity() -> Identity {
    Identity::new(
        "SYNTHETIC_ISSUER".into(),
        "SYNTHETIC_SUBJECT".into(),
        "SYNTHETIC_WORKSPACE".into(),
    )
    .unwrap()
}

#[test]
fn decoded_surrogate_pairs_cannot_hide_duplicate_keys() {
    let input = r#"{"🐻":1,"\ud83d\udc3b":2}"#;
    assert_eq!(
        validate_json_object(input.as_bytes()),
        Err(DataError::DuplicateKey)
    );
    for input in [r#"{"x":"\ud800"}"#, r#"{"\udc00":1}"#] {
        assert_eq!(
            validate_json_object(input.as_bytes()),
            Err(DataError::InvalidJson)
        );
    }
}

#[test]
fn nested_objects_obey_the_same_depth_limit_as_arrays() {
    let bounded = format!("{}0{}", "{\"x\":".repeat(64), "}".repeat(64));
    assert_eq!(validate_json_object(bounded.as_bytes()), Ok(()));
    let too_deep = format!("{}0{}", "{\"x\":".repeat(65), "}".repeat(65));
    assert_eq!(
        validate_json_object(too_deep.as_bytes()),
        Err(DataError::DepthLimit)
    );
}

#[test]
fn rejected_append_does_not_consume_the_proposed_generation_id() {
    let mut index = GenerationIndex::new(profile(), identity(), generation(1));
    index
        .append(profile(), &identity(), generation(1), generation(2))
        .unwrap();
    assert_eq!(
        index.append(profile(), &identity(), generation(1), generation(3)),
        Err(DataError::StaleParent)
    );
    assert_eq!(index.latest(), generation(2));
    assert_eq!(
        index.unreferenced_candidates(&[]).unwrap(),
        vec![generation(1)]
    );
    index
        .append(profile(), &identity(), generation(2), generation(3))
        .unwrap();
    assert_eq!(index.latest(), generation(3));
}

#[test]
fn shuffled_resources_preserve_bytes_and_require_explicit_optional_absence() {
    let rules = [
        (slot(0), ResourceShape::JsonObject, true),
        (slot(1), ResourceShape::Opaque, false),
        (slot(2), ResourceShape::Opaque, true),
    ];
    let exact = b" {\"SYNTHETIC_FIELD\": 1} \r\n";
    let set = CredentialSet::new(
        &rules,
        vec![
            Resource::present(slot(2), b"SYNTHETIC_OPAQUE".to_vec()).unwrap(),
            Resource::absent(slot(1)),
            Resource::present(slot(0), exact.to_vec()).unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(set.resource(slot(0)).unwrap().as_bytes(), Some(&exact[..]));
    assert_eq!(set.resource(slot(1)).unwrap().as_bytes(), None);
    let omitted = CredentialSet::new(
        &rules,
        vec![
            Resource::present(slot(0), exact.to_vec()).unwrap(),
            Resource::present(slot(2), b"SYNTHETIC_OPAQUE".to_vec()).unwrap(),
        ],
    );
    assert_eq!(omitted.unwrap_err(), DataError::ResourceSetMismatch);
}

#[test]
fn identity_limit_counts_utf8_bytes_not_characters() {
    let exact = "é".repeat(1024);
    assert_eq!(exact.len(), 2048);
    assert!(Identity::new("SYNTHETIC".into(), exact.clone(), "SYNTHETIC".into()).is_ok());
    assert_eq!(
        Identity::new("SYNTHETIC".into(), format!("{exact}x"), "SYNTHETIC".into())
            .unwrap_err(),
        DataError::InvalidIdentity
    );
}
