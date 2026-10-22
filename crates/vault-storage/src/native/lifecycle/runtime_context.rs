//! One operation's current installation, home and protocol observations.
//! Image versions identify an operation; they are never an approval allowlist.
use super::super::*;
use super::{launch::RuntimeConnection, process::Inventory};
use crate::journal::Failure;
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};
use codex_accounts_discovery::OfficialInstallation;
use codex_accounts_runtime::{
    AccountPresence, ChatGptEndpoint, ConfigSettings, CredentialStore, LoginMethod, Observed,
    PipeEvent, PipeSession, PolicySettings, Provider,
};
use sha2::{Digest, Sha256};
use std::os::windows::process::CommandExt;

pub(super) struct RuntimeContext {
    installation: OfficialInstallation,
    home: PathBuf,
    binding: [u8; 32],
    observer: Option<RuntimeConnection>,
    helper_exit_unproven: bool,
    configuration_checked: bool,
    selection_files: Vec<(PathBuf, Option<(File, Stamp)>)>,
    registered_core: bool,
    switch_cancellation: Option<crate::SwitchCancellation>,
}

impl RuntimeContext {
    /// Select only. No helper or authentication IO occurs until the caller has
    /// quiesced clients and acquired the target's retained home lock.
    pub(super) fn discover() -> Result<Self, Failure> {
        check_environment()?;
        let installation = OfficialInstallation::discover().map_err(|_| Failure::Binding)?;
        let home = selected_home()?;
        validate_path(&home).map_err(|_| Failure::Binding)?;
        let mut hash = Sha256::new();
        hash.update(b"CodexAccounts/OperationBinding/v1\0");
        // Recovery must survive an official Store update between app runs.
        // Retained installation handles separately pin this operation's images.
        hash.update(b"OpenAI.Codex_2p2nqsd0c76g0/native/file");
        for unit in home.as_os_str().encode_wide() {
            hash.update(unit.to_le_bytes());
        }
        Ok(Self {
            installation,
            home,
            binding: hash.finalize().into(),
            observer: None,
            helper_exit_unproven: false,
            configuration_checked: false,
            selection_files: Vec::new(),
            registered_core: false,
            switch_cancellation: None,
        })
    }
    pub(super) fn home(&self) -> &Path {
        &self.home
    }
    pub(super) fn installation(&self) -> &OfficialInstallation {
        &self.installation
    }
    pub(super) fn registered_core(&self) -> bool {
        self.registered_core
    }
    pub(super) fn switch_cancellation(&mut self, control: crate::SwitchCancellation) {
        self.switch_cancellation = Some(control);
    }
    pub(super) fn binding(&self) -> [u8; 32] {
        self.binding
    }
    pub(super) fn revalidate(&self) -> Result<(), Failure> {
        check_environment()?;
        if selected_home()? != self.home {
            return Err(Failure::Binding);
        }
        for (path, pinned) in &self.selection_files {
            match (
                pinned,
                open_file(
                    path,
                    GENERIC_READ | READ_CONTROL,
                    FILE_SHARE_READ,
                    OPEN_EXISTING,
                    None,
                ),
            ) {
                (None, Err(StorageError::Missing)) => {}
                (Some((file, original)), Ok(current))
                    if stamp(file).ok().as_ref() == Some(original)
                        && stamp(&current).ok().as_ref() == Some(original) => {}
                _ => return Err(Failure::ExternalChange),
            }
        }
        self.installation.revalidate().map_err(|_| Failure::Binding)
    }
    pub(super) fn check_selection(&mut self) -> Result<(), Failure> {
        self.quiesce()?;
        let config = self.selection_file("config.toml")?.unwrap_or_default();
        let global = self.selection_file(".codex-global-state.json")?;
        let parsed = match global
            .as_deref()
            .map(|v| crate::desktop_selection::parse_global(v))
        {
            Some(Ok(value)) => Some(value),
            _ => self
                .selection_file(".codex-global-state.json.bak")?
                .as_deref()
                .map(|v| crate::desktop_selection::parse_global(v))
                .transpose()
                .map_err(|_| Failure::Binding)?,
        };
        let selection =
            crate::desktop_selection::from_global(&config, parsed).map_err(|_| Failure::Binding)?;
        if selection.wsl {
            return Err(Failure::Policy);
        }
        // Explicit registered-core selection is reproduced for the helper; the
        // packaged launcher remains inside the owned job with its descendants.
        self.registered_core = self.installation.contained_core()
            && (selection.core_framework || self.installation.provisioned_core());
        self.revalidate()
    }
    fn selection_file(
        &mut self,
        leaf: &str,
    ) -> Result<Option<zeroize::Zeroizing<Vec<u8>>>, Failure> {
        let max_bytes = match leaf {
            ".codex-global-state.json" | ".codex-global-state.json.bak" => {
                crate::desktop_selection::MAX_DESKTOP_STATE_BYTES
            }
            _ => codex_accounts_vault::MAX_RESOURCE_BYTES,
        } as u64;
        let path = self.home.join(leaf);
        let mut file = match open_file(
            &path,
            GENERIC_READ | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => file,
            Err(StorageError::Missing) => {
                self.selection_files.push((path, None));
                return Ok(None);
            }
            Err(_) => return Err(Failure::Binding),
        };
        check_object(&file, false).map_err(|_| Failure::Binding)?;
        let original = stamp(&file).map_err(|_| Failure::Binding)?;
        if original.size > max_bytes {
            return Err(Failure::Binding);
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        (&mut file)
            .take(max_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Failure::Binding)?;
        if bytes.len() as u64 != original.size
            || stamp(&file).map_err(|_| Failure::Binding)? != original
        {
            return Err(Failure::ExternalChange);
        }
        self.selection_files.push((path, Some((file, original))));
        Ok(Some(bytes))
    }
    pub(super) fn quiesce(&self) -> Result<(), Failure> {
        let inventory = Inventory::capture_codex().map_err(|_| Failure::Writers)?;
        if inventory.has_live_clients().map_err(|_| Failure::Writers)? {
            return Err(Failure::Writers);
        }
        Ok(())
    }
    fn session(&mut self) -> Result<&mut PipeSession, Failure> {
        self.observer
            .as_mut()
            .and_then(|v| v.session.as_mut())
            .ok_or(Failure::Interrupted)
    }
    fn open(&mut self) -> Result<(), Failure> {
        self.revalidate()?;
        self.quiesce()?;
        if self.observer.is_some() {
            return Err(Failure::Busy);
        }
        self.observer = Some(
            RuntimeConnection::open(&self.installation, &self.home, false, self.registered_core)
                .map_err(|error| {
                    if error == StorageError::RecoveryRequired {
                        self.helper_exit_unproven = true;
                        Failure::HelperStuck
                    } else {
                        Failure::Binding
                    }
                })?,
        );
        if let (Some(control), Some(observer)) = (&self.switch_cancellation, &self.observer) {
            control
                .attach(observer.cancellation.clone())
                .map_err(|_| Failure::Cancelled)?;
        }
        self.session()?.initialize().map_err(|_| Failure::Binding)?;
        loop {
            match self.session()?.next_event().map_err(|_| Failure::Binding)? {
                PipeEvent::Initialized {
                    home,
                    platform_os,
                    platform_family,
                    ..
                } => {
                    if platform_os.as_str() != "windows"
                        || platform_family.as_str() != "windows"
                        || !super::same_path(Path::new(home.as_str()), &self.home)
                    {
                        return Err(Failure::Binding);
                    }
                    break;
                }
                PipeEvent::Written(_)
                | PipeEvent::IgnoredNotification
                | PipeEvent::AccountUpdated
                | PipeEvent::RateLimitsUpdated(_) => {}
                _ => return Err(Failure::Binding),
            }
        }
        self.session()?
            .initialized()
            .map_err(|_| Failure::Binding)?;
        // Flush the initialized notification before issuing the first request.
        loop {
            match self.session()?.next_event().map_err(|_| Failure::Binding)? {
                PipeEvent::Written(_) => return Ok(()),
                PipeEvent::IgnoredNotification
                | PipeEvent::AccountUpdated
                | PipeEvent::RateLimitsUpdated(_) => {}
                _ => return Err(Failure::Binding),
            }
        }
    }
    fn configuration(&mut self) -> Result<(), Failure> {
        self.session()?
            .read_config_requirements()
            .map_err(|_| Failure::Policy)?;
        let policy = loop {
            match self.session()?.next_event().map_err(|_| Failure::Policy)? {
                PipeEvent::Requirements(value) => break value.requirements,
                PipeEvent::Written(_)
                | PipeEvent::AccountUpdated
                | PipeEvent::IgnoredNotification
                | PipeEvent::RateLimitsUpdated(_) => {}
                _ => return Err(Failure::Policy),
            }
        };
        self.session()?.read_config().map_err(|_| Failure::Policy)?;
        let config = loop {
            match self.session()?.next_event().map_err(|_| Failure::Policy)? {
                PipeEvent::Config(value) => break value.config,
                PipeEvent::Written(_)
                | PipeEvent::AccountUpdated
                | PipeEvent::IgnoredNotification
                | PipeEvent::RateLimitsUpdated(_) => {}
                _ => return Err(Failure::Policy),
            }
        };
        compatible(config, policy)?;
        self.configuration_checked = true;
        Ok(())
    }
    pub(super) fn check_configuration(&mut self) -> Result<(), Failure> {
        self.check_selection()?;
        let result = self.open().and_then(|()| self.configuration());
        let cleanup = self.stop_observer_and_wait();
        cleanup.and(result)
    }
    pub(super) fn start_observer(&mut self) -> Result<(), Failure> {
        self.open()?;
        self.configuration()
    }
    pub(super) fn observe_account(&mut self) -> Result<CredentialAcceptance, Failure> {
        if !self.configuration_checked {
            return Err(Failure::Policy);
        }
        self.session()?
            .read_account()
            .map_err(|_| Failure::Interrupted)?;
        let present = loop {
            match self.session()?.next_event() {
                Ok(PipeEvent::Account {
                    presence,
                    requires_openai_auth,
                    ..
                }) => {
                    if !requires_openai_auth {
                        return Err(Failure::Policy);
                    }
                    break presence == AccountPresence::Managed;
                }
                Ok(
                    PipeEvent::Written(_)
                    | PipeEvent::AccountUpdated
                    | PipeEvent::IgnoredNotification
                    | PipeEvent::RateLimitsUpdated(_),
                ) => {}
                _ => return Ok(CredentialAcceptance::ObservationUnavailable),
            }
        };
        if !present {
            return Ok(CredentialAcceptance::Rejected);
        }
        // account/read is a local metadata read. A successful quota request is
        // the separate online observation; offline/protocol errors are unknown.
        self.session()?
            .read_rate_limits()
            .map_err(|_| Failure::Interrupted)?;
        loop {
            match self.session()?.next_event() {
                Ok(PipeEvent::RateLimits(_)) => {
                    return Ok(CredentialAcceptance::Accepted);
                }
                Ok(PipeEvent::RateLimitsUpdated(_)) => {}
                Ok(
                    PipeEvent::Written(_)
                    | PipeEvent::AccountUpdated
                    | PipeEvent::IgnoredNotification,
                ) => {}
                _ => return Ok(CredentialAcceptance::ObservationUnavailable),
            }
        }
    }
    pub(super) fn stop_observer_and_wait(&mut self) -> Result<(), Failure> {
        if self.helper_exit_unproven {
            return Err(Failure::HelperStuck);
        }
        if let Some(observer) = self.observer.as_mut() {
            observer
                .close_and_wait()
                .map_err(|_| Failure::HelperStuck)?;
        }
        self.observer.take();
        if let Some(control) = &self.switch_cancellation {
            control.detach();
        }
        Ok(())
    }
    pub(super) fn launch_desktop(&mut self) -> Result<DesktopLaunch, Failure> {
        if self.observer.is_some() {
            return Err(Failure::HelperStuck);
        }
        self.revalidate()?;
        self.quiesce()?;
        self.selection_files.clear();
        // A normal new Desktop launch, with no credential argument, custom CLI,
        // process-kill permission, or claims about which account its UI shows.
        let child = std::process::Command::new(self.installation.desktop_path())
            .current_dir(
                self.installation
                    .desktop_path()
                    .parent()
                    .ok_or(Failure::Binding)?,
            )
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|_| Failure::Launch)?;
        drop(child);
        Ok(DesktopLaunch::Opened)
    }
}

pub(super) fn selected_home() -> Result<PathBuf, Failure> {
    match std::env::var_os("CODEX_HOME") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        Some(_) => Err(Failure::Binding),
        None => known_folder(&FOLDERID_Profile)
            .map(|p| p.join(".codex"))
            .map_err(|_| Failure::Binding),
    }
}
pub(super) fn check_environment() -> Result<(), Failure> {
    for name in [
        "CODEX_CLI_PATH",
        "CODEX_API_KEY",
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "NODE_OPTIONS",
        "CODEX_USE_WSL",
        "WSL_DISTRO_NAME",
        "CODEX_CA_ROOT_CERT",
        "CODEX_INSECURE_NO_TLS_VERIFY",
        "CODEX_APP_SERVER_LOGIN_ISSUER",
        "CODEX_APP_SERVER_LOGIN_CLIENT_ID",
        "CODEX_REFRESH_TOKEN_URL_OVERRIDE",
        "CODEX_REVOKE_TOKEN_URL_OVERRIDE",
    ] {
        if std::env::var_os(name).is_some_and(|v| !v.is_empty()) {
            return Err(Failure::Policy);
        }
    }
    Ok(())
}
fn absent<T>(value: Observed<T>) -> bool {
    matches!(value, Observed::Missing | Observed::Null)
}
fn default_provider(value: Observed<Provider>) -> bool {
    absent(value) || value == Observed::Value(Provider::OpenAi)
}
fn official_chatgpt_endpoint(value: Observed<ChatGptEndpoint>) -> bool {
    absent(value) || value == Observed::Value(ChatGptEndpoint::Official)
}
pub(super) fn compatible(
    config: ConfigSettings,
    requirements: Observed<PolicySettings>,
) -> Result<(), Failure> {
    if !default_provider(config.provider)
        || !official_chatgpt_endpoint(config.chatgpt_base_url)
        || !absent(config.base_url)
        || matches!(config.provider_definitions, Observed::Value(v) if v.entries != 0)
        || !absent(config.workspace_restriction)
        || !(absent(config.forced_login)
            || config.forced_login == Observed::Value(LoginMethod::ChatGpt))
    {
        return Err(Failure::Policy);
    }
    let store = if let Observed::Value(policy) = requirements {
        if !default_provider(policy.provider)
            || !official_chatgpt_endpoint(policy.chatgpt_base_url)
            || matches!(policy.provider_definitions, Observed::Value(v) if v.entries != 0)
            || matches!(policy.allowed_login_methods, Observed::Value(v) if !v.chatgpt || v.unknown)
        {
            return Err(Failure::Policy);
        }
        if absent(policy.credential_store) {
            config.credential_store
        } else {
            policy.credential_store
        }
    } else {
        config.credential_store
    };
    // Official core defaults omitted/null cli_auth_credentials_store to File.
    // This is used only after current config and managed requirements both reply.
    if absent(store) || store == Observed::Value(CredentialStore::File) {
        Ok(())
    } else {
        Err(Failure::Policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_chatgpt_default_is_supported_but_overrides_remain_refused() {
        for (endpoint, accepted) in [
            ("https://chatgpt.com/backend-api/", true),
            ("https://chatgpt.com/backend-api", true),
            ("https://synthetic.invalid/backend-api/", false),
            ("https://chatgpt.com.synthetic.invalid/backend-api/", false),
            ("https://chatgpt.com@synthetic.invalid/backend-api/", false),
            ("http://chatgpt.com/backend-api/", false),
            ("https://chatgpt.com/backend-api/other", false),
            ("https://chatgpt.com/backend-api/?redirect=synthetic", false),
            ("https://chatgpt.com/backend-api/#synthetic", false),
            ("https://chatgpt.com:8443/backend-api/", false),
            ("", false),
        ] {
            let config: ConfigSettings = serde_json::from_value(serde_json::json!({
                "cli_auth_credentials_store": "file", "chatgpt_base_url": endpoint
            }))
            .unwrap();
            assert_eq!(compatible(config, Observed::Null).is_ok(), accepted);
            let policy: PolicySettings = serde_json::from_value(serde_json::json!({
                "chatgptBaseUrl": endpoint
            }))
            .unwrap();
            let default_config: ConfigSettings = serde_json::from_str("{}").unwrap();
            assert_eq!(
                compatible(default_config, Observed::Value(policy)).is_ok(),
                accepted
            );
            assert!(!format!("{config:?} {policy:?}").contains(endpoint) || endpoint.is_empty());
        }
        let config: ConfigSettings = serde_json::from_str(
            r#"{"chatgpt_base_url":"https://chatgpt.com/backend-api/","cli_auth_credentials_store":"keyring"}"#
        ).unwrap();
        assert_eq!(compatible(config, Observed::Null), Err(Failure::Policy));
        let config: ConfigSettings = serde_json::from_str(
            r#"{"chatgpt_base_url":"https://chatgpt.com/backend-api/","forced_login_method":"api"}"#
        ).unwrap();
        assert_eq!(compatible(config, Observed::Null), Err(Failure::Policy));
    }
}
