use std::process::Command;
use serde_json::Value;

#[test]
fn rejected_arguments_do_not_echo_paths_or_secret_canaries() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-accounts-discovery"))
        .args(["--qualified", "SYNTHETIC_SECRET_CANARY"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("SYNTHETIC"));
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["credential_mutation_enabled"], false);
    assert_eq!(value["qualified"], false);
}

#[cfg(not(windows))]
#[test]
fn other_hosts_are_not_windows_or_macos_qualification() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-accounts-discovery"))
        .output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"], "E_PLATFORM_UNSUPPORTED");
    assert_eq!(value["credential_mutation_enabled"], false);
}
