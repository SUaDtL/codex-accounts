//! Minimal projections of the captured official config/read and
//! configRequirements/read schemas. No paths, URLs, provider definitions or
//! workspace identifiers are retained. Missing/null are never filled by defaults.
#![forbid(unsafe_code)]
use super::*;
use serde::de::SeqAccess;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observed<T> {
    Missing,
    Null,
    Value(T),
}
fn observed<T>(value: Option<Option<T>>) -> Observed<T> {
    match value {
        None => Observed::Missing,
        Some(None) => Observed::Null,
        Some(Some(value)) => Observed::Value(value),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialStore {
    File,
    Keyring,
    Auto,
    Ephemeral,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginMethod {
    ChatGpt,
    Api,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    OpenAi,
    Other,
}

macro_rules! classification {
    ($name:ident, $value:ident, $body:expr) => {
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = Secret::deserialize(d)?;
                let $value = text.as_str();
                Ok($body)
            }
        }
    };
}
classification!(
    CredentialStore,
    value,
    match value {
        "file" => Self::File,
        "keyring" => Self::Keyring,
        "auto" => Self::Auto,
        "ephemeral" => Self::Ephemeral,
        _ => Self::Unknown,
    }
);
classification!(
    LoginMethod,
    value,
    match value {
        "chatgpt" => Self::ChatGpt,
        "api" => Self::Api,
        _ => Self::Unknown,
    }
);
classification!(
    Provider,
    value,
    if value == "openai" {
        Self::OpenAi
    } else {
        Self::Other
    }
);

/// Classifies only the official production default, discarding the URL itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatGptEndpoint {
    Official,
    Other,
}
classification!(
    ChatGptEndpoint,
    value,
    match value {
        // openai/codex codex-rs/config/defaults.toml. Accept the equivalent
        // spelling without a final slash, never a prefix or normalized host.
        "https://chatgpt.com/backend-api/" | "https://chatgpt.com/backend-api" => Self::Official,
        _ => Self::Other,
    }
);

/// The value was an explicit string override; its contents are discarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringOverride;
impl<'de> Deserialize<'de> for StringOverride {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let _ = Secret::deserialize(d)?;
        Ok(Self)
    }
}

/// Count only; an empty object is distinct from an omitted/null setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectSummary {
    pub entries: usize,
}
impl<'de> Deserialize<'de> for ObjectSummary {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = ObjectSummary;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = 0;
                while map.next_key::<Secret>()?.is_some() {
                    if entries == 256 {
                        return Err(invalid());
                    }
                    entries += 1;
                    map.next_value::<IgnoredAny>()?;
                }
                Ok(ObjectSummary { entries })
            }
        }
        d.deserialize_map(Fields)
    }
}

/// Actual restriction shape, with identities discarded immediately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRestriction {
    Single,
    List { entries: usize },
}
impl<'de> Deserialize<'de> for WorkspaceRestriction {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Shape;
        impl<'de> Visitor<'de> for Shape {
            type Value = WorkspaceRestriction;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("workspace restriction")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value.len() > TEXT_LIMIT || value.contains('\0') {
                    return Err(invalid());
                }
                Ok(WorkspaceRestriction::Single)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = 0;
                while sequence.next_element::<Secret>()?.is_some() {
                    if entries == 64 {
                        return Err(invalid());
                    }
                    entries += 1;
                }
                Ok(WorkspaceRestriction::List { entries })
            }
        }
        d.deserialize_any(Shape)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllowedLoginMethods {
    pub chatgpt: bool,
    pub api: bool,
    pub unknown: bool,
}
impl<'de> Deserialize<'de> for AllowedLoginMethods {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Items;
        impl<'de> Visitor<'de> for Items {
            type Value = AllowedLoginMethods;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("allowed login methods")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut result = AllowedLoginMethods {
                    chatgpt: false,
                    api: false,
                    unknown: false,
                };
                let mut count = 0;
                while let Some(method) = sequence.next_element::<LoginMethod>()? {
                    if count == 64 {
                        return Err(invalid());
                    }
                    count += 1;
                    match method {
                        LoginMethod::ChatGpt => result.chatgpt = true,
                        LoginMethod::Api => result.api = true,
                        LoginMethod::Unknown => result.unknown = true,
                    }
                }
                Ok(result)
            }
        }
        d.deserialize_seq(Items)
    }
}

macro_rules! settings {
    ($name:ident { $($field:ident : $ty:ty => $key:literal),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name { $(pub $field: Observed<$ty>,)* }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct Fields;
                impl<'de> Visitor<'de> for Fields {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("configuration observation") }
                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                        $(let mut $field = None;)*
                        while let Some(key) = map.next_key::<Secret>()? {
                            match key.as_str() {
                                $($key => unique(&mut $field, map.next_value::<Option<$ty>>()?)?,)*
                                _ => { map.next_value::<IgnoredAny>()?; }
                            }
                        }
                        Ok($name { $($field: observed($field),)* })
                    }
                }
                d.deserialize_map(Fields)
            }
        }
    };
}

settings!(ConfigSettings {
    credential_store: CredentialStore => "cli_auth_credentials_store",
    forced_login: LoginMethod => "forced_login_method",
    workspace_restriction: WorkspaceRestriction => "forced_chatgpt_workspace_id",
    provider: Provider => "model_provider",
    provider_definitions: ObjectSummary => "model_providers",
    chatgpt_base_url: ChatGptEndpoint => "chatgpt_base_url",
    base_url: StringOverride => "base_url",
});

settings!(PolicySettings {
    credential_store: CredentialStore => "cliAuthCredentialsStore",
    allowed_login_methods: AllowedLoginMethods => "allowedLoginMethods",
    provider: Provider => "modelProvider",
    provider_definitions: ObjectSummary => "modelProviders",
    chatgpt_base_url: ChatGptEndpoint => "chatgptBaseUrl",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigObservation {
    pub config: ConfigSettings,
}
impl<'de> Deserialize<'de> for ConfigObservation {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = ConfigObservation;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("config read result")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut config, mut origins, mut layers) = (None, None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "config" => unique(&mut config, map.next_value::<ConfigSettings>()?)?,
                        "origins" => unique(&mut origins, map.next_value::<ObjectSummary>()?)?,
                        "layers" => {
                            unique(&mut layers, map.next_value::<Option<Vec<IgnoredAny>>>()?)?
                        }
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                required(origins)?;
                Ok(ConfigObservation {
                    config: required(config)?,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}

settings!(RequirementsObservation { requirements: PolicySettings => "requirements" });
