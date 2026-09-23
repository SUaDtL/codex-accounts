//! Developer-only read-only inventory. This is not a credential-management CLI.
#![deny(unsafe_code)]

use codex_accounts_core::compatibility::{Context, Observation};
use codex_accounts_core::Backend;
use serde_json::{json, Value};
use std::fmt;
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(windows)]
#[allow(unsafe_code)]
mod native;
#[cfg(windows)]
mod windows_reader;

pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 1024 * 1024;
pub const MAX_CANDIDATES: usize = 64;
pub const MAX_PACKAGES: usize = 4096;
pub const MAX_APPLICATIONS: usize = 64;
pub const MAX_PATH_UNITS: usize = 32760;
pub const MAX_EXECUTABLE_BYTES: u64 = 1024 * 1024 * 1024;

pub const ENV_FLAGS: &[&str] = &[
    "CODEX_HOME",
    "CODEX_CLI_PATH",
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "CODEX_API_KEY",
    "NODE_OPTIONS",
    "LD_PRELOAD",
    "DYLD_INSERT_LIBRARIES",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    UnsupportedPlatform,
    EnumerationUnavailable,
    AccessDenied,
    Missing,
    UnsafePath,
    Changed,
    InputLimit,
    InvalidMetadata,
    SharingViolation,
    Io,
    Usage,
}

impl ReadError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "E_PLATFORM_UNSUPPORTED",
            Self::EnumerationUnavailable => "E_ENUMERATION_UNAVAILABLE",
            Self::AccessDenied => "E_ACCESS_DENIED",
            Self::Missing => "E_CANDIDATE_MISSING",
            Self::UnsafePath => "E_PATH_UNSAFE",
            Self::Changed => "E_EXTERNAL_CHANGE",
            Self::InputLimit => "E_INPUT_LIMIT",
            Self::InvalidMetadata => "E_METADATA_INVALID",
            Self::SharingViolation => "E_SHARING_VIOLATION",
            Self::Io => "E_INVENTORY_IO",
            Self::Usage => "E_USAGE",
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for ReadError {}

/// Raw paths remain private, are not serializable, and have redacted Debug.
#[derive(Default)]
pub struct Options {
    pub(crate) candidate: Option<String>,
    pub(crate) runtime_relative: Option<String>,
    pub(crate) home: Option<PathBuf>,
    pub(crate) local: bool,
}
impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Options([REDACTED])")
    }
}

pub fn options(args: impl IntoIterator<Item = String>) -> Result<Options, ReadError> {
    let mut result = Options::default();
    let mut args = args.into_iter();
    let mut seen = std::collections::BTreeSet::new();
    while let Some(arg) = args.next() {
        if !seen.insert(arg.clone()) {
            return Err(ReadError::Usage);
        }
        match arg.as_str() {
            "--candidate" => {
                let id = args.next().ok_or(ReadError::Usage)?;
                if id.len() != 64
                    || !id
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                {
                    return Err(ReadError::Usage);
                }
                result.candidate = Some(id);
            }
            "--runtime-relative" => {
                let value = args.next().ok_or(ReadError::Usage)?;
                validate_relative_executable(&value)?;
                result.runtime_relative = Some(value);
            }
            "--candidate-home" => {
                let value = args.next().ok_or(ReadError::Usage)?;
                if value.is_empty() || value.len() > MAX_PATH_UNITS || value.contains('\0') {
                    return Err(ReadError::Usage);
                }
                result.home = Some(value.into());
            }
            "--show-local-paths" => result.local = true,
            _ => return Err(ReadError::Usage),
        }
    }
    if result.candidate.is_none() && (result.runtime_relative.is_some() || result.home.is_some()) {
        return Err(ReadError::Usage);
    }
    Ok(result)
}

/// A package-relative executable is only a candidate, never executable authority.
/// Reject Windows path aliases before touching the filesystem, on every test OS.
pub fn validate_relative_executable(value: &str) -> Result<(), ReadError> {
    if value.len() > MAX_PATH_UNITS
        || value.is_empty()
        || value.starts_with(['/', '\\'])
        || value.contains([':', '\0', '%'])
        || !value.to_ascii_lowercase().ends_with(".exe")
    {
        return Err(ReadError::UnsafePath);
    }
    for part in value.split(['/', '\\']) {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.chars().any(|c| c.is_control())
        {
            return Err(ReadError::UnsafePath);
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err(ReadError::UnsafePath);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct DeclaredConfig {
    pub state: &'static str,
    pub backend: Observation<Backend>,
    pub restriction_present: bool,
    pub unsupported_source_present: bool,
    pub profile_layers_present: bool,
}
impl DeclaredConfig {
    pub fn unknown(state: &'static str) -> Self {
        Self {
            state,
            backend: Observation::Unknown,
            restriction_present: false,
            unsupported_source_present: false,
            profile_layers_present: false,
        }
    }
    pub fn report(&self) -> Value {
        let value = match self.backend {
            Observation::Observed(Backend::File) => "file",
            Observation::Observed(Backend::Auto) => "auto",
            Observation::Observed(Backend::Keyring) => "keyring",
            Observation::Observed(Backend::Secrets) => "secrets",
            Observation::Observed(Backend::Ephemeral) => "ephemeral",
            _ => "unknown",
        };
        json!({"state": self.state, "backend_state": self.backend.state(),
            "declared_backend": value, "provenance": "nominated_home_top_level_declaration_only",
            "managed_restriction_declaration_present": self.restriction_present,
            "unsupported_auth_or_provider_declaration_present": self.unsupported_source_present,
            "profile_layers_present": self.profile_layers_present,
            "effective_backend": "unknown", "effective_home": "unknown", "effective_policy": "unknown"})
    }
}

/// Parse a bounded config in memory; never preserve its values or error body.
/// No rule for the selected Desktop build exists yet, so this is NOT precedence.
pub fn parse_declared_config(bytes: &[u8]) -> DeclaredConfig {
    if bytes.len() > MAX_CONFIG_BYTES {
        return DeclaredConfig::unknown("input_limit");
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return DeclaredConfig::unknown("invalid_utf8");
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return DeclaredConfig::unknown("invalid_toml");
    };
    let backend = match table.get("cli_auth_credentials_store") {
        None => Observation::Absent,
        Some(toml::Value::String(s)) => match s.as_str() {
            "file" => Observation::Observed(Backend::File),
            "auto" => Observation::Observed(Backend::Auto),
            "keyring" => Observation::Observed(Backend::Keyring),
            "secrets" => Observation::Observed(Backend::Secrets),
            "ephemeral" => Observation::Observed(Backend::Ephemeral),
            _ => Observation::Unsupported,
        },
        _ => Observation::Unsupported,
    };
    DeclaredConfig {
        state: "observed",
        backend,
        restriction_present: ["forced_login_method", "forced_chatgpt_workspace_id"]
            .iter()
            .any(|k| table.contains_key(*k)),
        unsupported_source_present: table.contains_key("model_providers")
            || table
                .get("model_provider")
                .is_some_and(|v| v.as_str() != Some("openai"))
            || ["api_key", "experimental_bearer_token", "env_key"]
                .iter()
                .any(|k| table.contains_key(*k)),
        profile_layers_present: table.contains_key("profiles") || table.contains_key("profile"),
    }
}

pub fn context_report(config: &DeclaredConfig, flags: &[&str]) -> Value {
    let flags: Vec<&str> = ENV_FLAGS
        .iter()
        .copied()
        .filter(|name| flags.contains(name))
        .collect();
    let c = Context {
        declared_backend: config.backend,
        override_present: !flags.is_empty(),
        ..Context::default()
    };
    json!({"outward_code": c.refusal().as_str(), "declared_config": config.report(),
        "current_process_environment_flags": flags, "environment_provenance": "diagnostic_process_not_desktop",
        "exact_build_resolution_rule": "unknown", "effective_home": c.home.state(),
        "effective_backend": c.effective_backend.state(), "effective_policy": c.policy.state(),
        "credential_mutation_enabled": c.credential_mutation_enabled(),
        "unresolved": ["desktop_effective_home", "all_material_configuration_layers", "managed_policy_sources", "auth_resource_contract", "runtime_role_and_schema", "desktop_lifecycle_and_identity", "owner_native_evidence"]})
}

pub fn error_report(error: ReadError) -> Value {
    json!({"schema": "codex-accounts/native-discovery/v1", "error": error.code(),
        "qualified": false, "credential_mutation_enabled": false})
}

pub fn encode(report: &Value) -> Result<String, ReadError> {
    let text = serde_json::to_string_pretty(report).map_err(|_| ReadError::InvalidMetadata)?;
    if text.len() > MAX_REPORT_BYTES {
        return Err(ReadError::InputLimit);
    }
    Ok(text)
}

pub fn inspect(opts: &Options) -> Result<Value, ReadError> {
    #[cfg(windows)]
    {
        windows_reader::inspect(opts)
    }
    #[cfg(not(windows))]
    {
        let _ = opts;
        Err(ReadError::UnsupportedPlatform)
    }
}

pub fn is_candidate_hint(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("codex") || name.contains("chatgpt")
}

#[cfg(windows)]
pub(crate) fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_is_not_precedence() {
        for mode in ["file", "auto", "keyring", "secrets", "ephemeral", "custom"] {
            let config =
                parse_declared_config(format!("cli_auth_credentials_store = '{mode}'").as_bytes());
            assert_eq!(config.state, "observed");
            let output = context_report(&config, &["CODEX_HOME"]);
            assert_eq!(output["effective_backend"], "unknown");
            assert_eq!(output["effective_home"], "unknown");
            assert_eq!(output["effective_policy"], "unknown");
            assert_eq!(output["credential_mutation_enabled"], false);
        }
    }

    #[test]
    fn missing_invalid_denied_and_conflict_are_not_default_proof() {
        assert_eq!(parse_declared_config(b"").backend, Observation::Absent);
        assert_eq!(
            parse_declared_config(
                b"cli_auth_credentials_store='file'\ncli_auth_credentials_store='auto'"
            )
            .state,
            "invalid_toml"
        );
        assert_eq!(parse_declared_config(&[255]).state, "invalid_utf8");
        assert_eq!(
            parse_declared_config(&vec![b' '; MAX_CONFIG_BYTES + 1]).state,
            "input_limit"
        );
        let cfg = DeclaredConfig {
            state: "inaccessible",
            backend: Observation::Inaccessible,
            ..DeclaredConfig::unknown("unknown")
        };
        assert_eq!(cfg.report()["backend_state"], "inaccessible");
        let cfg = DeclaredConfig {
            backend: Observation::Conflicting,
            ..cfg
        };
        assert_eq!(cfg.report()["backend_state"], "conflicting");
    }

    #[test]
    fn reports_and_errors_never_include_config_or_argument_canaries() {
        let canary = "SYNTHETIC_SECRET_CANARY";
        let input = format!("cli_auth_credentials_store='file'\nforced_chatgpt_workspace_id='{canary}'\nmodel_provider='{canary}'\n[profiles.'{canary}']\nfoo='{canary}'");
        let cfg = parse_declared_config(input.as_bytes());
        assert!(
            cfg.restriction_present && cfg.unsupported_source_present && cfg.profile_layers_present
        );
        assert!(!format!("{cfg:?}").contains(canary));
        assert!(!encode(&context_report(&cfg, &[])).unwrap().contains(canary));
        let options = options(vec![
            "--candidate".into(),
            "a".repeat(64),
            "--candidate-home".into(),
            canary.into(),
        ])
        .unwrap();
        assert!(!format!("{options:?}").contains(canary));
        assert!(!encode(&error_report(ReadError::Usage))
            .unwrap()
            .contains(canary));
    }

    #[test]
    fn relative_paths_cannot_escape_package_or_name_credentials() {
        for bad in [
            "../x.exe",
            "C:\\x.exe",
            "\\\\server\\x.exe",
            "a//b.exe",
            "a/./b.exe",
            "a/../b.exe",
            "auth.json",
            "cap_sid",
            "a.exe:auth.json",
            "a. /b.exe",
            "CON.exe",
            "a/%x%.exe",
            "a/../auth.exe",
            "a\0.exe",
        ] {
            assert_eq!(
                validate_relative_executable(bad),
                Err(ReadError::UnsafePath),
                "synthetic invalid path accepted"
            );
        }
        assert!(validate_relative_executable("resources/合成 runtime.exe").is_ok());
    }

    #[test]
    fn options_do_not_offer_authority_or_arbitrary_invocation() {
        for bad in [
            "--force",
            "--qualified",
            "--rpc",
            "--execute",
            "--auth-file",
            "--publisher",
        ] {
            assert!(options(vec![bad.into(), "SYNTHETIC_CANARY".into()]).is_err());
        }
        assert!(options(vec![
            "--candidate".into(),
            "a".repeat(64),
            "--candidate".into(),
            "a".repeat(64)
        ])
        .is_err());
    }

    #[test]
    fn heuristic_is_not_publisher_trust() {
        assert!(is_candidate_hint("Untrusted.SyntheticCodex"));
        assert!(is_candidate_hint("Synthetic.ChatGPT"));
        assert!(!is_candidate_hint("Synthetic.Editor"));
        assert!(!Context::default().credential_mutation_enabled());
    }

    #[test]
    fn output_is_bounded_and_json_escaped() {
        assert!(encode(&json!({"value": "x".repeat(MAX_REPORT_BYTES)})).is_err());
        let text = encode(&json!({"value": "<script>\"\nSYNTHETIC"})).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap()["value"],
            "<script>\"\nSYNTHETIC"
        );
    }
}
