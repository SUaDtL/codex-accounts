use super::*;

#[test]
fn onboarding_record_authenticates_cleanup_and_recovers_unpublished_torn_write() {
    let sandbox = super::super::super::super::tests::Sandbox::new();
    let namespace = Namespace::open(&sandbox.0, true).unwrap().unwrap();
    let root = RootKey::generate().unwrap();
    // Crash during intent publication happens before any scratch home exists.
    let mut pending = open_file(
        &namespace.path.join(PENDING),
        GENERIC_WRITE,
        0,
        CREATE_NEW,
        Some(&Security::new().unwrap()),
    )
    .unwrap();
    pending.write_all(b"TORN_SYNTHETIC_ENVELOPE").unwrap();
    pending.sync_all().unwrap();
    drop(pending);
    namespace.cleanup_all(&root).unwrap();
    assert!(!has_unpublished(&namespace).unwrap());

    let record = Onboarding::begin(
        &namespace,
        &root,
        Some(ProfileId::from_bytes([7; 16]).unwrap()),
    )
    .unwrap();
    let home = record.home(&namespace);
    namespace.create_home(&home).unwrap();
    std::fs::write(home.join("auth.json"), b"SYNTHETIC_NO_REAL_CREDENTIAL").unwrap();
    drop(record);
    let wrong = RootKey::generate().unwrap();
    assert!(namespace.cleanup_all(&wrong).is_err());
    assert!(home.join("auth.json").exists());
    // A well-formed random directory name alone never authorizes its deletion.
    let orphan = namespace
        .path
        .join("login-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    create_directory(&orphan, &namespace.security).unwrap();
    assert!(namespace.cleanup_all(&root).is_err());
    assert!(orphan.exists());
    assert!(home.exists());
    std::fs::remove_dir(&orphan).unwrap();
    let record = Onboarding::read(&namespace, &root).unwrap().unwrap();
    assert_eq!(record.home(&namespace), home);
    namespace.cleanup_registered(&record).unwrap();
    drop(record);
    assert!(!home.exists());
    assert!(Onboarding::read(&namespace, &root).unwrap().is_none());
}
