//! Captured-schema subset used by the bounded pipe driver and synthetic tests.
//! No version equality gate. Account metadata is not authenticated identity proof.
#![forbid(unsafe_code)]
use super::*;
use serde::de::{self, Deserialize, DeserializeSeed, IgnoredAny, MapAccess, Visitor};
#[path = "codec_login.rs"]
mod managed;
pub(super) use managed::{CancelStatus, Presence};
#[path = "codec_rates.rs"]
mod rates;
pub(super) use rates::{ReadObservation, Snapshot};
#[path = "codec_config.rs"]
pub(super) mod config;
use config::{ConfigObservation, RequirementsObservation};

pub(super) const RUNTIME_VERSION: &str = "0.162.0-alpha.17.2";
pub(super) const RUNTIME_IMAGE_SHA256: &str =
    "e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e";
pub(super) const SCHEMA_MANIFEST_SHA256: &str =
    "a87f649f36c3a2787182bedd125002cf1a2eff54c46383b04e7d1431a7dd758e";
const TEXT_LIMIT: usize = 4096;

/// Retained untrusted text has a zeroizing owner and no payload diagnostics.
pub(super) struct Secret(Zeroizing<String>);
impl Secret {
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
fn invalid<E: de::Error>() -> E {
    E::custom("unsupported wire shape")
}
impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = Zeroizing::new(String::deserialize(d)?);
        if value.len() > TEXT_LIMIT || value.contains('\0') {
            return Err(invalid());
        }
        Ok(Self(value))
    }
}
fn unique<T, E: de::Error>(slot: &mut Option<T>, value: T) -> Result<(), E> {
    if slot.is_some() {
        return Err(invalid());
    }
    *slot = Some(value);
    Ok(())
}
fn required<T, E: de::Error>(slot: Option<T>) -> Result<T, E> {
    slot.ok_or_else(invalid)
}

/// A declaration from this helper, never a filesystem capability/effective home.
pub(super) struct InitializeObservation {
    pub(super) codex_home: Secret,
    pub(super) platform_family: Secret,
    pub(super) platform_os: Secret,
    pub(super) user_agent: Secret,
}
impl fmt::Debug for InitializeObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InitializeObservation([REDACTED])")
    }
}
fn absolute_declaration(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\');
    let unc =
        path.starts_with("\\\\") && path.split('\\').filter(|part| !part.is_empty()).count() >= 2;
    (drive || unc || path.starts_with('/'))
        && !path.chars().any(char::is_control)
        && !path
            .split(['/', '\\'])
            .any(|part| part == "." || part == "..")
}
impl<'de> Deserialize<'de> for InitializeObservation {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = InitializeObservation;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("initialize result")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut home, mut family, mut os, mut agent) = (None, None, None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "codexHome" => unique(&mut home, map.next_value::<Secret>()?)?,
                        "platformFamily" => unique(&mut family, map.next_value::<Secret>()?)?,
                        "platformOs" => unique(&mut os, map.next_value::<Secret>()?)?,
                        "userAgent" => unique(&mut agent, map.next_value::<Secret>()?)?,
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                let home: Secret = required(home)?;
                if !absolute_declaration(home.as_str()) {
                    return Err(invalid());
                }
                Ok(InitializeObservation {
                    codex_home: home,
                    platform_family: required(family)?,
                    platform_os: required(os)?,
                    user_agent: required(agent)?,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}

/// Display metadata only. No principal, workspace, login or acceptance claim.
pub(super) struct ManagedAccount {
    pub(super) email: Option<Secret>,
    pub(super) plan: &'static str,
}
impl fmt::Debug for ManagedAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ManagedAccount([REDACTED])")
    }
}
pub(super) struct AccountObservation {
    pub(super) account: AccountPresence,
    pub(super) requires_openai_auth: bool,
    unsupported_mode: bool,
}
/// Wire presence only. Neither omission nor null proves credential absence.
pub(super) enum AccountPresence {
    Missing,
    Null,
    Managed(ManagedAccount),
}
impl fmt::Debug for AccountObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountObservation([REDACTED])")
    }
}
enum AccountMode {
    Managed(ManagedAccount),
    Unsupported,
}
fn plan(value: &str) -> Option<&'static str> {
    [
        "free",
        "go",
        "plus",
        "pro",
        "prolite",
        "promax",
        "team",
        "self_serve_business_prolite",
        "self_serve_business_usage_based",
        "business",
        "ent26",
        "enterprise_cbp_automation",
        "enterprise_cbp_usage_based",
        "enterprise",
        "edu",
        "edu_plus",
        "edu_pro",
        "unknown",
    ]
    .into_iter()
    .find(|item| *item == value)
}
impl<'de> Deserialize<'de> for AccountMode {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = AccountMode;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("account object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut kind, mut email, mut tier, mut bedrock) = (None, None, None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "type" => unique(&mut kind, map.next_value::<Secret>()?)?,
                        "email" => unique(&mut email, map.next_value::<Option<Secret>>()?)?,
                        "planType" => unique(&mut tier, map.next_value::<Secret>()?)?,
                        "usesCodexManagedCredentials" => {
                            unique(&mut bedrock, map.next_value::<bool>()?)?
                        }
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                let kind: Secret = required(kind)?;
                if kind.as_str() != "chatgpt" {
                    return Ok(AccountMode::Unsupported);
                }
                if bedrock.is_some() {
                    return Err(invalid());
                }
                let tier: Secret = required(tier)?;
                Ok(AccountMode::Managed(ManagedAccount {
                    email: required(email)?,
                    plan: plan(tier.as_str()).ok_or_else(invalid)?,
                }))
            }
        }
        d.deserialize_map(Fields)
    }
}
impl<'de> Deserialize<'de> for AccountObservation {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = AccountObservation;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("account read result")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut account, mut auth) = (None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "account" => {
                            unique(&mut account, map.next_value::<Option<AccountMode>>()?)?
                        }
                        "requiresOpenaiAuth" => unique(&mut auth, map.next_value::<bool>()?)?,
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                let (account, unsupported_mode) = match account {
                    Some(Some(AccountMode::Managed(account))) => {
                        (AccountPresence::Managed(account), false)
                    }
                    Some(Some(AccountMode::Unsupported)) => (AccountPresence::Missing, true),
                    Some(None) => (AccountPresence::Null, false),
                    None => (AccountPresence::Missing, false),
                };
                Ok(AccountObservation {
                    account,
                    requires_openai_auth: required(auth)?,
                    unsupported_mode,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}

struct RemoteError;
impl<'de> Deserialize<'de> for RemoteError {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = RemoteError;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("remote error")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut code, mut message, mut data) = (None, None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "code" => unique(&mut code, map.next_value::<i64>()?)?,
                        "message" => unique(&mut message, map.next_value::<Secret>()?)?,
                        "data" => unique(&mut data, map.next_value::<IgnoredAny>()?)?,
                        _ => return Err(invalid()),
                    }
                }
                required(code)?;
                required(message)?;
                Ok(RemoteError)
            }
        }
        d.deserialize_map(Fields)
    }
}
struct Envelope {
    id: Option<u64>,
    method: Option<Secret>,
    result: bool,
    error: bool,
}
impl<'de> Deserialize<'de> for Envelope {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = Envelope;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("response envelope")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut id, mut result, mut error, mut method, mut params, mut emitted) =
                    (None, None, None, None, None, None);
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "id" => unique(&mut id, map.next_value::<u64>()?)?,
                        "result" => unique(&mut result, map.next_value::<IgnoredAny>()?)?,
                        "error" => unique(&mut error, map.next_value::<RemoteError>()?)?,
                        "method" => unique(&mut method, map.next_value::<Secret>()?)?,
                        "params" => unique(&mut params, map.next_value::<IgnoredAny>()?)?,
                        "emittedAtMs" => unique(&mut emitted, map.next_value::<i64>()?)?,
                        _ => return Err(invalid()),
                    }
                }
                if id.is_some_and(|id| !(1..=256).contains(&id)) {
                    return Err(invalid());
                }
                if params.is_some() && method.is_none() {
                    return Err(invalid());
                }
                if emitted.is_some()
                    && (method.is_none() || id.is_some() || result.is_some() || error.is_some())
                {
                    return Err(invalid());
                }
                if method.is_some()
                    && id.is_none()
                    && (params.is_none() || result.is_some() || error.is_some())
                {
                    return Err(invalid());
                }
                Ok(Envelope {
                    id,
                    method,
                    result: result.is_some(),
                    error: error.is_some(),
                })
            }
        }
        d.deserialize_map(Fields)
    }
}
pub(super) enum CodecEvent {
    IgnoredNotification,
    Config(ConfigObservation),
    Requirements(RequirementsObservation),
    Initialized(InitializeObservation),
    Account(AccountObservation),
    RateLimits(ReadObservation),
    RateLimitsUpdated(Snapshot),
    LoginStarted {
        auth_url: Secret,
    },
    CancellationReady,
    CancellationProgress,
    LoginCompletionBuffered,
    LoginCompletion {
        outcome: LoginCompletion,
        error: Presence<Secret>,
        onboarding: Presence<&'static str>,
    },
    AccountUpdated(managed::AccountUpdate),
    CancellationAcknowledged {
        status: CancelStatus,
        primary: SessionError,
    },
}
impl fmt::Debug for CodecEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CodecEvent([REDACTED])")
    }
}
struct ResultSeed(Method);
impl<'de> DeserializeSeed<'de> for ResultSeed {
    type Value = CodecEvent;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        match self.0 {
            Method::Initialize => {
                InitializeObservation::deserialize(d).map(CodecEvent::Initialized)
            }
            Method::AccountRead => AccountObservation::deserialize(d).map(CodecEvent::Account),
            Method::RateLimitsRead => ReadObservation::deserialize(d).map(CodecEvent::RateLimits),
            Method::ConfigRead => ConfigObservation::deserialize(d).map(CodecEvent::Config),
            Method::ConfigRequirementsRead => {
                RequirementsObservation::deserialize(d).map(CodecEvent::Requirements)
            }
            _ => Err(invalid()),
        }
    }
}
struct ResponseSeed(Method);
impl<'de> DeserializeSeed<'de> for ResponseSeed {
    type Value = CodecEvent;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        struct Fields(Method);
        impl<'de> Visitor<'de> for Fields {
            type Value = CodecEvent;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("typed response")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut result = None;
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "id" => {
                            map.next_value::<u64>()?;
                        }
                        "result" => unique(&mut result, map.next_value_seed(ResultSeed(self.0))?)?,
                        _ => return Err(invalid()),
                    }
                }
                required(result)
            }
        }
        d.deserialize_map(Fields(self.0))
    }
}

/// Bound to the original reader and session; construction stays inside the driver.
pub(super) struct Codec {
    owner: Arc<()>,
    reader: Arc<()>,
    ignored_notifications: std::sync::atomic::AtomicUsize,
}
impl fmt::Debug for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Codec([REDACTED])")
    }
}
impl Codec {
    pub(super) fn new(driver: &TransportDriver, session: &ProtocolSession) -> Self {
        assert!(Arc::ptr_eq(&driver.owner, &session.owner));
        Self {
            owner: session.owner.clone(),
            reader: driver.reader.clone(),
            ignored_notifications: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    #[cfg(test)]
    pub(super) fn for_test(driver: &TransportDriver, session: &ProtocolSession) -> Self {
        Self::new(driver, session)
    }
    fn fail(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
        error: SessionError,
    ) -> DriverError {
        // Parsing must not let a simultaneous terminal deadline lose precedence.
        let _ = session.poll(Instant::now());
        let _ = session.stop::<()>(error);
        driver.fail(session, DriverFault::Session)
    }
    fn gate(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
    ) -> Result<(), DriverError> {
        if !Arc::ptr_eq(&self.owner, &session.owner) || !Arc::ptr_eq(&self.reader, &driver.reader) {
            return Err(driver.fail(session, DriverFault::Binding));
        }
        driver.gate(session).map(|_| ())
    }
    pub(super) fn queue(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
        method: Method,
    ) -> Result<IssuedRequest, DriverError> {
        self.gate(session, driver)?;
        if !matches!(
            method,
            Method::Initialize
                | Method::AccountRead
                | Method::RateLimitsRead
                | Method::ConfigRead
                | Method::ConfigRequirementsRead
        ) {
            return Err(self.fail(session, driver, SessionError::MethodUnavailable));
        }
        let request = session
            .reserve(method, Instant::now())
            .map_err(|_| driver.fail(session, DriverFault::Session))?;
        let id = request.id.sequence;
        let body = match method {
            Method::Initialize => format!("{{\"id\":{id},\"method\":\"initialize\",\"params\":{{\"clientInfo\":{{\"name\":\"local_codex_switcher\",\"version\":\"{}\"}}}}}}\n", env!("CARGO_PKG_VERSION")),
            Method::AccountRead => format!("{{\"id\":{id},\"method\":\"account/read\",\"params\":{{\"refreshToken\":false}}}}\n"),
            Method::RateLimitsRead => format!("{{\"id\":{id},\"method\":\"account/rateLimits/read\",\"params\":{{\"excludeResetCreditDetails\":true}}}}\n"),
            Method::ConfigRead => format!("{{\"id\":{id},\"method\":\"config/read\",\"params\":{{\"includeLayers\":false}}}}\n"),
            Method::ConfigRequirementsRead => format!("{{\"id\":{id},\"method\":\"configRequirements/read\",\"params\":null}}\n"),
            _ => unreachable!(),
        };
        driver.queue_request(session, &request, Zeroizing::new(body.into_bytes()))?;
        Ok(request)
    }
    pub(super) fn initialized(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
    ) -> Result<(), DriverError> {
        self.gate(session, driver)?;
        driver.queue_initialized(
            session,
            Zeroizing::new(b"{\"method\":\"initialized\"}\n".to_vec()),
        )
    }
    /// Consume the owned lease; callers cannot submit byte slices or a forged ID.
    pub(super) fn consume(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
        frame: ReaderFrame,
    ) -> Result<CodecEvent, DriverError> {
        self.gate(session, driver)?;
        let bytes = frame
            .bytes(driver, session)
            .map_err(|_| driver.fail(session, DriverFault::Binding))?;
        let envelope: Envelope = serde_json::from_slice(bytes)
            .map_err(|_| self.fail(session, driver, SessionError::InvalidMessage))?;
        if let Some(method) = &envelope.method {
            if envelope.id.is_some() {
                return Err(self.fail(session, driver, SessionError::UnexpectedServerRequest));
            }
            if method.as_str() == "account/rateLimits/updated" {
                return self.rate_notification(session, driver, bytes);
            }
            if !matches!(
                method.as_str(),
                "account/updated" | "account/login/completed"
            ) {
                // Unknown notifications are forward-compatible metadata only.
                // Keep a separate lifetime bound: they cannot acknowledge a
                // request, extend a deadline or leave an account signal queued.
                if self
                    .ignored_notifications
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    >= 32
                {
                    return Err(self.fail(session, driver, SessionError::NotificationLimit));
                }
                session
                    .account_notification(Instant::now())
                    .map_err(|_| driver.fail(session, DriverFault::Session))?;
                session.notifications -= 1;
                return Ok(CodecEvent::IgnoredNotification);
            }
            return self.notification(session, driver, bytes, method.as_str());
        }
        if envelope.result == envelope.error {
            return Err(self.fail(session, driver, SessionError::InvalidMessage));
        }
        let Some(sequence) = envelope.id else {
            return Err(self.fail(session, driver, SessionError::InvalidMessage));
        };
        let Some(pending) = session
            .pending
            .iter()
            .find(|pending| pending.request.id.sequence == sequence)
        else {
            return Err(self.fail(session, driver, SessionError::Correlation));
        };
        let id = pending.request.id.clone();
        let method = pending.request.method;
        if !pending.sent {
            return Err(self.fail(session, driver, SessionError::UnsentResponse));
        }
        if !matches!(
            method,
            Method::Initialize
                | Method::AccountRead
                | Method::RateLimitsRead
                | Method::ConfigRead
                | Method::ConfigRequirementsRead
                | Method::LoginStart
                | Method::LoginCancel
        ) {
            return Err(self.fail(session, driver, SessionError::MethodUnavailable));
        }
        if envelope.error {
            if method == Method::LoginStart {
                let _ = session.login_start_error(&id, Instant::now());
                return Err(driver.fail(session, DriverFault::Session));
            }
            if method == Method::LoginCancel {
                let _ = session.login_cancel_response(&id, ResponseOutcome::Error, Instant::now());
                return Err(driver.fail(session, DriverFault::Session));
            }
            return Err(self.fail(session, driver, SessionError::RemoteError));
        }
        if matches!(method, Method::LoginStart | Method::LoginCancel) {
            return self.login_response(session, driver, bytes, &id, method);
        }
        let mut decoder = serde_json::Deserializer::from_slice(bytes);
        let event = ResponseSeed(method)
            .deserialize(&mut decoder)
            .and_then(|event| {
                decoder.end()?;
                Ok(event)
            })
            .map_err(|_| self.fail(session, driver, SessionError::InvalidMessage))?;
        if matches!(&event, CodecEvent::Account(account) if account.unsupported_mode) {
            return Err(self.fail(session, driver, SessionError::MethodUnavailable));
        }
        drop(frame);
        self.gate(session, driver)?;
        session
            .response(&id, ResponseOutcome::Success, Instant::now())
            .map_err(|_| driver.fail(session, DriverFault::Session))?;
        Ok(event)
    }
}
