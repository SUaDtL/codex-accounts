use serde_json::Value;
use std::process::Command;

#[test]
fn registered_metadata_help_and_invalid_combinations_are_redacted() {
    let binary = env!("CARGO_BIN_EXE_codex-accounts-discovery");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("--registered-metadata"));
    for tail in [
        vec![],
        vec!["--candidate-home", "SYNTHETIC_SECRET_CANARY"],
        vec!["--runtime-relative", "synthetic.exe"],
    ] {
        let mut args = vec!["--registered-metadata".to_owned()];
        if !tail.is_empty() {
            args.extend(["--candidate".into(), "a".repeat(64)]);
        }
        args.extend(tail.into_iter().map(str::to_owned));
        let output = Command::new(binary).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("SYNTHETIC"));
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["error"], "E_USAGE");
        assert_eq!(report["qualified"], false);
        assert_eq!(report["credential_mutation_enabled"], false);
    }
}

#[test]
fn rejected_arguments_do_not_echo_paths_or_secret_canaries() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-accounts-discovery"))
        .args(["--qualified", "SYNTHETIC_SECRET_CANARY"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("SYNTHETIC"));
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["credential_mutation_enabled"], false);
    assert_eq!(value["qualified"], false);
}

#[test]
fn local_only_output_cannot_be_redirected_into_a_report() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-accounts-discovery"))
        .arg("--show-local-paths")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"], "E_USAGE");
    assert_eq!(value["qualified"], false);
    assert!(value.get("local_display_only_do_not_share").is_none());
}

#[cfg(not(windows))]
#[test]
fn other_hosts_are_not_windows_or_macos_qualification() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-accounts-discovery"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"], "E_PLATFORM_UNSUPPORTED");
    assert_eq!(value["credential_mutation_enabled"], false);
}
