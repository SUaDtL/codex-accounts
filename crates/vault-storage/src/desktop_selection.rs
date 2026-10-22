//! Pure projection of Desktop selection settings from caller-owned byte slices.
//! The caller controls bounded file reads and any global-state backup fallback.
//! Explicit flags are not evidence that a runtime has been provisioned.
#![forbid(unsafe_code)]

use crate::StorageError;
use codex_accounts_vault::{validate_json_object_with_limit, MAX_RESOURCE_BYTES};

// Desktop keeps unrelated UI/session state in this file. It is not a credential
// resource; retain a separate finite cap without changing vault resource limits.
pub(crate) const MAX_DESKTOP_STATE_BYTES: usize = 16 * 1024 * 1024;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Selection {
    pub wsl: bool,
    pub core_framework: bool,
    pub primary_framework: bool,
    pub prefer_mxc: bool,
}

/// Known root global-state flags only. Missing flags default false; malformed
/// present values (including null) refuse, while unrelated metadata is skipped.
#[derive(Clone, Copy, Default, Deserialize)]
pub(crate) struct GlobalSelection {
    #[serde(default, rename = "runCodexInWindowsSubsystemForLinux")]
    wsl: bool,
    #[serde(default, rename = "electron-windows-core-runtime-frameworks-enabled")]
    core_framework: bool,
    #[serde(
        default,
        rename = "electron-windows-primary-runtime-frameworks-enabled"
    )]
    primary_framework: bool,
    #[serde(default, rename = "electron-prefer-mxc-enabled")]
    prefer_mxc: bool,
}

#[derive(Default, Deserialize)]
struct Desktop {
    #[serde(rename = "runCodexInWindowsSubsystemForLinux")]
    wsl: Option<bool>,
}
#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    desktop: Desktop,
}

pub(crate) fn parse(
    config_toml: &[u8],
    global_json: Option<&[u8]>,
) -> Result<Selection, StorageError> {
    from_global(config_toml, global_json.map(parse_global).transpose()?)
}

/// Separate projection lets the caller distinguish corrupt primary global state
/// from invalid TOML before applying Desktop's explicit .bak fallback policy.
pub(crate) fn parse_global(bytes: &[u8]) -> Result<GlobalSelection, StorageError> {
    if bytes.len() > MAX_DESKTOP_STATE_BYTES {
        return Err(StorageError::InputLimit);
    }
    validate_json_object_with_limit(bytes, MAX_DESKTOP_STATE_BYTES)?;
    serde_json::from_slice(bytes).map_err(|_| StorageError::InvalidData)
}

/// TOML's explicit true OR false takes precedence over the legacy global flag.
/// Absent sources are represented by empty TOML and None, not malformed bytes.
pub(crate) fn from_global(
    config_toml: &[u8],
    global: Option<GlobalSelection>,
) -> Result<Selection, StorageError> {
    bounded(config_toml)?;
    let text = std::str::from_utf8(config_toml).map_err(|_| StorageError::InvalidData)?;
    let config: Config = toml::from_str(text).map_err(|_| StorageError::InvalidData)?;
    let global = global.unwrap_or_default();
    Ok(Selection {
        wsl: config.desktop.wsl.unwrap_or(global.wsl),
        core_framework: global.core_framework,
        primary_framework: global.primary_framework,
        prefer_mxc: global.prefer_mxc,
    })
}

fn bounded(bytes: &[u8]) -> Result<(), StorageError> {
    if bytes.len() > MAX_RESOURCE_BYTES {
        Err(StorageError::InputLimit)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_desktop_state_does_not_use_credential_size_limit() {
        let mut global =
            br#"{"electron-windows-core-runtime-frameworks-enabled":true,"unrelated":""#.to_vec();
        global.extend(std::iter::repeat_n(b'x', 2_500_000));
        global.extend_from_slice(br#""}"#);
        assert!(parse(b"", Some(&global)).unwrap().core_framework);
        // Raising the state limit must not admit large credential resources.
        assert!(matches!(
            codex_accounts_vault::validate_json_object(&global),
            Err(codex_accounts_vault::DataError::InputLimit)
        ));
        assert!(matches!(
            parse_global(&vec![b' '; 16 * 1024 * 1024 + 1]),
            Err(StorageError::InputLimit)
        ));
    }

    #[test]
    fn explicit_toml_false_overrides_legacy_true_and_flags_remain_separate() {
        let global = br#"{"runCodexInWindowsSubsystemForLinux":true,"electron-windows-core-runtime-frameworks-enabled":true,"electron-windows-primary-runtime-frameworks-enabled":false,"electron-prefer-mxc-enabled":true,"unrelated":{"value":[1,2,3]}}"#;
        assert!(parse(b"", Some(global)).unwrap().wsl);
        assert_eq!(
            parse(
                b"[desktop]\nrunCodexInWindowsSubsystemForLinux = false\nother = 'ignored'",
                Some(global)
            )
            .unwrap(),
            Selection {
                wsl: false,
                core_framework: true,
                primary_framework: false,
                prefer_mxc: true
            }
        );
        assert!(
            parse(b"[desktop]\nrunCodexInWindowsSubsystemForLinux=true", None)
                .unwrap()
                .wsl
        );
        assert_eq!(parse(b"", None).unwrap(), Selection::default());
        assert_eq!(
            parse(b"[unrelated]\nvalue = 7", Some(b"{}")).unwrap(),
            Selection::default()
        );
    }

    #[test]
    fn malformed_types_duplicates_and_limits_refuse_without_backup_inference() {
        for global in [
            r#"{"runCodexInWindowsSubsystemForLinux":null}"#,
            r#"{"electron-prefer-mxc-enabled":"true"}"#,
            r#"{"electron-windows-core-runtime-frameworks-enabled":1}"#,
            r#"{"electron-windows-primary-runtime-frameworks-enabled":[]}"#,
            r#"{"unrelated":{"x":1,"\u0078":2}}"#,
            "[]",
            "{",
        ] {
            assert!(parse_global(global.as_bytes()).is_err());
        }
        for config in [
            "[desktop]\nrunCodexInWindowsSubsystemForLinux='true'",
            "[desktop]\nrunCodexInWindowsSubsystemForLinux=true\nrunCodexInWindowsSubsystemForLinux=false",
            "desktop = false",
            "[",
        ] {
            assert!(parse(config.as_bytes(), None).is_err());
        }
        assert!(matches!(
            parse(&vec![b' '; MAX_RESOURCE_BYTES + 1], None),
            Err(StorageError::InputLimit)
        ));
        assert!(matches!(
            parse_global(&vec![b' '; MAX_DESKTOP_STATE_BYTES + 1]),
            Err(StorageError::InputLimit)
        ));
        // The caller may supply its separately read backup only after a failed
        // primary parse; no path lookup or automatic fallback occurs here.
        let backup = parse_global(br#"{"electron-prefer-mxc-enabled":true}"#).unwrap();
        assert!(from_global(b"", Some(backup)).unwrap().prefer_mxc);
    }
}
