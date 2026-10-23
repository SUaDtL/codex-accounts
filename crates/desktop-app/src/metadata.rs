//! Only bounded, typed public metadata crosses the renderer boundary.
use codex_accounts_discovery::OfficialInstallation;
use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultState {
    Unopened,
    Open,
    RecoveryRequired,
}

#[derive(Serialize)]
pub struct ProfileList {
    pub state: VaultState,
    pub vault_exists: Option<bool>,
    pub protection: Option<&'static str>,
    pub last_known_active_profile: Option<String>,
    pub profiles: Vec<Profile>,
    pub operations: Vec<Operation>,
    pub recovery: &'static str,
    pub health: Option<Health>,
    pub reauthentication: Option<Reauthentication>,
    pub diagnostics_exported: Option<bool>,
}

#[derive(Serialize)]
pub struct Reauthentication {
    pub profile: String,
    pub installation: Option<Operation>,
}

#[derive(Clone, Serialize)]
pub struct Health {
    pub source: &'static str,
    pub profile: String,
    pub acceptance: &'static str,
    pub checked_at: u64,
    pub available_resets: Option<u32>,
    pub primary: Option<QuotaWindow>,
    pub secondary: Option<QuotaWindow>,
}

#[derive(Clone, Serialize)]
pub struct QuotaWindow {
    pub used_percent: i32,
    pub duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Serialize)]
pub struct Operation {
    pub id: String,
    pub phase: &'static str,
    pub installed_profile: Option<String>,
    pub credential_acceptance: &'static str,
    pub desktop_launch: &'static str,
    pub primary_error: Option<&'static str>,
    pub restoration_error: Option<&'static str>,
    pub can_recover: bool,
    pub can_restore: bool,
}

#[derive(Serialize)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub domain: String,
    pub generation: String,
    pub captured_at: u64,
}

pub fn unopened_profiles(vault_exists: Option<bool>) -> ProfileList {
    ProfileList {
        state: VaultState::Unopened,
        vault_exists,
        protection: None,
        last_known_active_profile: None,
        profiles: Vec::new(),
        operations: Vec::new(),
        recovery: "none",
        health: None,
        reauthentication: None,
        diagnostics_exported: None,
    }
}

#[derive(Serialize)]
pub struct Installation {
    pub version: [u16; 4],
    pub architecture: Architecture,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X64,
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Inventory {
    Observed { installations: Vec<Installation> },
    Unavailable { code: &'static str },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    NotChecked,
}

#[derive(Serialize)]
pub struct Status {
    pub inventory: Inventory,
    pub compatibility: Compatibility,
}

pub fn inspect() -> Status {
    // Discover once. This selects the registered official x64 package and binds
    // its images, but does not inspect a Codex home or establish backend policy.
    let inventory = match OfficialInstallation::discover() {
        Ok(official) => selected_inventory(official.version()),
        Err(error) => Inventory::Unavailable { code: error.code() },
    };
    Status {
        inventory,
        compatibility: Compatibility::NotChecked,
    }
}

fn selected_inventory(version: [u16; 4]) -> Inventory {
    Inventory::Observed {
        installations: vec![Installation {
            version,
            architecture: Architecture::X64,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renderer_projection_excludes_raw_discovery_fields() {
        let output = serde_json::to_value(selected_inventory([1, 2, 3, 4])).unwrap();
        assert_eq!(
            output,
            json!({"state":"observed", "installations":[{ "version": [1,2,3,4], "architecture": "x64" }]})
        );
    }
}
