//! Managed-login schema subset. No production construction, browser or IO.
#![forbid(unsafe_code)]
use super::*;
use serde::ser::{Serialize, SerializeMap, Serializer};

pub(in super::super) enum Presence<T> {
    Missing,
    Null,
    Present(T),
}
impl<T> fmt::Debug for Presence<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Presence([REDACTED])")
    }
}
fn presence<T>(value: Option<Option<T>>) -> Presence<T> {
    match value {
        None => Presence::Missing,
        Some(None) => Presence::Null,
        Some(Some(v)) => Presence::Present(v),
    }
}
fn valid_id(value: &Secret) -> bool {
    !value.as_str().is_empty() && value.as_str().len() <= 256
}
fn login_id(value: &Secret, session: &ProtocolSession) -> LoginId {
    LoginId {
        owner: session.owner.clone(),
        bytes: Zeroizing::new(value.as_str().as_bytes().to_vec()),
    }
}
pub(super) struct StartReply {
    pub(super) id: Secret,
    pub(super) auth_url: Secret,
}
impl<'de> Deserialize<'de> for StartReply {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = StartReply;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("managed start result")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let (mut kind, mut id, mut url) = (None, None, None);
                while let Some(key) = m.next_key::<Secret>()? {
                    match key.as_str() {
                        "type" => unique(&mut kind, m.next_value::<Secret>()?)?,
                        "loginId" => unique(&mut id, m.next_value::<Secret>()?)?,
                        "authUrl" => unique(&mut url, m.next_value::<Secret>()?)?,
                        _ => return Err(invalid()),
                    }
                }
                let kind: Secret = required(kind)?;
                let id: Secret = required(id)?;
                if kind.as_str() != "chatgpt" || !valid_id(&id) {
                    return Err(invalid());
                }
                Ok(StartReply {
                    id,
                    auth_url: required(url)?,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum CancelStatus {
    Canceled,
    NotFound,
}
impl<'de> Deserialize<'de> for CancelStatus {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = CancelStatus;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("cancel status result")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let mut status = None;
                while let Some(key) = m.next_key::<Secret>()? {
                    if key.as_str() != "status" {
                        return Err(invalid());
                    }
                    unique(&mut status, m.next_value::<Secret>()?)?;
                }
                let status: Secret = required(status)?;
                match status.as_str() {
                    "canceled" => Ok(CancelStatus::Canceled),
                    "notFound" => Ok(CancelStatus::NotFound),
                    _ => Err(invalid()),
                }
            }
        }
        d.deserialize_map(Fields)
    }
}
pub(super) struct Completed {
    id: Secret,
    success: bool,
    error: Presence<Secret>,
    onboarding: Presence<&'static str>,
}
impl<'de> Deserialize<'de> for Completed {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = Completed;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("managed completion notification")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let (mut id, mut success, mut error, mut onboarding) = (None, None, None, None);
                while let Some(key) = m.next_key::<Secret>()? {
                    match key.as_str() {
                        "loginId" => unique(&mut id, m.next_value::<Option<Secret>>()?)?,
                        "success" => unique(&mut success, m.next_value::<bool>()?)?,
                        "error" => unique(&mut error, m.next_value::<Option<Secret>>()?)?,
                        "onboardingEntrypoint" => {
                            unique(&mut onboarding, m.next_value::<Option<Secret>>()?)?
                        }
                        _ => return Err(invalid()),
                    }
                }
                let id: Secret = id.flatten().ok_or_else(invalid)?;
                if !valid_id(&id) {
                    return Err(invalid());
                }
                let onboarding = match presence(onboarding) {
                    Presence::Missing => Presence::Missing,
                    Presence::Null => Presence::Null,
                    Presence::Present(value) if value.as_str() == "life_sciences" => {
                        Presence::Present("life_sciences")
                    }
                    _ => return Err(invalid()),
                };
                Ok(Completed {
                    id,
                    success: required(success)?,
                    error: presence(error),
                    onboarding,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}
pub(in super::super) struct AccountUpdate {
    pub(in super::super) auth_mode: Presence<&'static str>,
    pub(in super::super) plan: Presence<&'static str>,
}
impl<'de> Deserialize<'de> for AccountUpdate {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = AccountUpdate;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("account change notification")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let (mut mode, mut tier) = (None, None);
                while let Some(key) = m.next_key::<Secret>()? {
                    match key.as_str() {
                        "authMode" => unique(&mut mode, m.next_value::<Option<Secret>>()?)?,
                        "planType" => unique(&mut tier, m.next_value::<Option<Secret>>()?)?,
                        _ => return Err(invalid()),
                    }
                }
                let auth_mode = match presence(mode) {
                    Presence::Missing => Presence::Missing,
                    Presence::Null => Presence::Null,
                    Presence::Present(value) if value.as_str() == "chatgpt" => {
                        Presence::Present("chatgpt")
                    }
                    _ => return Err(invalid()),
                };
                let tier = match presence(tier) {
                    Presence::Missing => Presence::Missing,
                    Presence::Null => Presence::Null,
                    Presence::Present(value) => {
                        Presence::Present(plan(value.as_str()).ok_or_else(invalid)?)
                    }
                };
                Ok(AccountUpdate {
                    auth_mode,
                    plan: tier,
                })
            }
        }
        d.deserialize_map(Fields)
    }
}
enum Notification {
    Completed(Completed),
    Updated(AccountUpdate),
}
#[derive(Clone, Copy)]
enum Kind {
    Completed,
    Updated,
}
struct ParamsSeed(Kind);
impl<'de> DeserializeSeed<'de> for ParamsSeed {
    type Value = Notification;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        match self.0 {
            Kind::Completed => Completed::deserialize(d).map(Notification::Completed),
            Kind::Updated => AccountUpdate::deserialize(d).map(Notification::Updated),
        }
    }
}
struct NotificationSeed(Kind);
impl<'de> DeserializeSeed<'de> for NotificationSeed {
    type Value = Notification;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        struct Fields(Kind);
        impl<'de> Visitor<'de> for Fields {
            type Value = Notification;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("typed notification envelope")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let mut params = None;
                while let Some(key) = m.next_key::<Secret>()? {
                    match key.as_str() {
                        "method" => {
                            m.next_value::<Secret>()?;
                        }
                        "params" => unique(&mut params, m.next_value_seed(ParamsSeed(self.0))?)?,
                        "emittedAtMs" => {
                            m.next_value::<i64>()?;
                        }
                        _ => return Err(invalid()),
                    }
                }
                required(params)
            }
        }
        d.deserialize_map(Fields(self.0))
    }
}
struct CancelWire<'a> {
    sequence: u64,
    target: &'a str,
}
impl Serialize for CancelWire<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        struct Params<'a>(&'a str);
        impl Serialize for Params<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("loginId", self.0)?;
                map.end()
            }
        }
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("id", &self.sequence)?;
        map.serialize_entry("method", "account/login/cancel")?;
        map.serialize_entry("params", &Params(self.target))?;
        map.end()
    }
}
impl Codec {
    pub(in super::super) fn start_login(
        &self,
        s: &mut ProtocolSession,
        d: &mut TransportDriver,
    ) -> Result<IssuedRequest, DriverError> {
        self.gate(s, d)?;
        let ticket = s
            .start_login(Instant::now())
            .map_err(|_| d.fail(s, DriverFault::Session))?;
        let sequence = ticket.id.sequence;
        let bytes = format!("{{\"id\":{sequence},\"method\":\"account/login/start\",\"params\":{{\"type\":\"chatgpt\"}}}}\n");
        d.queue_request(s, &ticket, Zeroizing::new(bytes.into_bytes()))?;
        Ok(ticket)
    }
    pub(in super::super) fn cancel_login(
        &self,
        s: &mut ProtocolSession,
        d: &mut TransportDriver,
    ) -> Result<Option<IssuedRequest>, DriverError> {
        self.gate(s, d)?;
        let Some(ticket) = s
            .reserve_login_cancel(Instant::now())
            .map_err(|reason| self.fail(s, d, reason))?
        else {
            return Ok(None);
        };
        let Some(target) = s.cancellation_target().cloned() else {
            return Err(self.fail(s, d, SessionError::Correlation));
        };
        let target = std::str::from_utf8(&target.bytes)
            .map_err(|_| self.fail(s, d, SessionError::LoginIdInvalid))?;
        let mut bytes = Zeroizing::new(
            serde_json::to_vec(&CancelWire {
                sequence: ticket.id.sequence,
                target,
            })
            .map_err(|_| self.fail(s, d, SessionError::InvalidMessage))?,
        );
        bytes.push(b'\n');
        d.queue_request(s, &ticket, bytes)?;
        Ok(Some(ticket))
    }
    pub(super) fn notification(
        &self,
        s: &mut ProtocolSession,
        d: &mut TransportDriver,
        bytes: &[u8],
        method: &str,
    ) -> Result<CodecEvent, DriverError> {
        let kind = match method {
            "account/login/completed" => Kind::Completed,
            "account/updated" => Kind::Updated,
            _ => return Err(self.fail(s, d, SessionError::MethodUnavailable)),
        };
        let mut decoder = serde_json::Deserializer::from_slice(bytes);
        let notification = NotificationSeed(kind)
            .deserialize(&mut decoder)
            .and_then(|value| {
                decoder.end()?;
                Ok(value)
            })
            .map_err(|_| self.fail(s, d, SessionError::InvalidMessage))?;
        self.gate(s, d)?;
        match notification {
            Notification::Updated(update) => {
                s.account_notification(Instant::now())
                    .map_err(|_| d.fail(s, DriverFault::Session))?;
                Ok(CodecEvent::AccountUpdated(update))
            }
            Notification::Completed(value) => {
                let id = login_id(&value.id, s);
                let early = s.login.as_ref().is_some_and(|flow| flow.id.is_none());
                let cancelling = s.phase() == SessionPhase::LoginCancelling;
                let outcome = if value.success {
                    LoginCompletion::Success
                } else {
                    LoginCompletion::Failure
                };
                s.login_completed(&id, outcome, Instant::now())
                    .map_err(|_| d.fail(s, DriverFault::Session))?;
                if cancelling {
                    Ok(CodecEvent::CancellationProgress)
                } else if early {
                    Ok(CodecEvent::LoginCompletionBuffered)
                } else {
                    Ok(CodecEvent::LoginCompletion {
                        outcome,
                        error: value.error,
                        onboarding: value.onboarding,
                    })
                }
            }
        }
    }
    pub(super) fn login_response(
        &self,
        s: &mut ProtocolSession,
        d: &mut TransportDriver,
        bytes: &[u8],
        id: &RequestId,
        method: Method,
    ) -> Result<CodecEvent, DriverError> {
        struct ResultOnly<T>(T);
        impl<'de, T: Deserialize<'de>> Deserialize<'de> for ResultOnly<T> {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct Fields<T>(std::marker::PhantomData<T>);
                impl<'de, T: Deserialize<'de>> Visitor<'de> for Fields<T> {
                    type Value = ResultOnly<T>;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("typed login response")
                    }
                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut m: A,
                    ) -> Result<Self::Value, A::Error> {
                        let mut value = None;
                        while let Some(key) = m.next_key::<Secret>()? {
                            match key.as_str() {
                                "id" => {
                                    m.next_value::<u64>()?;
                                }
                                "result" => unique(&mut value, m.next_value::<T>()?)?,
                                _ => return Err(invalid()),
                            }
                        }
                        Ok(ResultOnly(required(value)?))
                    }
                }
                d.deserialize_map(Fields(std::marker::PhantomData))
            }
        }
        match method {
            Method::LoginStart => {
                let ResultOnly(value): ResultOnly<StartReply> = serde_json::from_slice(bytes)
                    .map_err(|_| self.fail(s, d, SessionError::InvalidMessage))?;
                self.gate(s, d)?;
                let login_id = login_id(&value.id, s);
                let cancelling = s.phase() == SessionPhase::LoginCancelling;
                s.login_start_response(id, &login_id, Instant::now())
                    .map_err(|_| d.fail(s, DriverFault::Session))?;
                if cancelling {
                    Ok(CodecEvent::CancellationReady)
                } else {
                    Ok(CodecEvent::LoginStarted {
                        auth_url: value.auth_url,
                    })
                }
            }
            Method::LoginCancel => {
                let ResultOnly(status): ResultOnly<CancelStatus> = serde_json::from_slice(bytes)
                    .map_err(|_| self.fail(s, d, SessionError::InvalidMessage))?;
                self.gate(s, d)?;
                let _ = s.login_cancel_response(id, ResponseOutcome::Success, Instant::now());
                let terminal = d.fail(s, DriverFault::Session);
                if s.cancellation_state() == CancellationState::Acknowledged {
                    Ok(CodecEvent::CancellationAcknowledged {
                        status,
                        primary: terminal.primary,
                    })
                } else {
                    Err(terminal)
                }
            }
            _ => Err(self.fail(s, d, SessionError::MethodUnavailable)),
        }
    }
}
