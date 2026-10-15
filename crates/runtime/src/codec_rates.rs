//! Private bounded rate metadata. No cache, permission, polling or credit action.
#![forbid(unsafe_code)]
use super::*;
use serde::de::{SeqAccess, Visitor};

const COLLECTION_LIMIT: usize = 64;

fn presence<T>(value: Option<Option<T>>) -> Presence<T> {
    match value {
        None => Presence::Missing,
        Some(None) => Presence::Null,
        Some(Some(value)) => Presence::Present(value),
    }
}

/// Generate strict typed object visitors, retaining missing/null independently.
/// Only schema-unconstrained fields may use the explicit ignored branch.
macro_rules! object {
    ($name:ident { required { $($rf:ident : $rt:ty => $rk:literal),* $(,)? }
      optional { $($of:ident : $ot:ty => $ok:literal),* $(,)? }
      ignored { $($ig:ident => $ik:literal),* $(,)? } }) => {
        pub(in super::super) struct $name {
            $(pub(in super::super) $rf: $rt,)*
            $(pub(in super::super) $of: Presence<$ot>,)*
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct Fields;
                impl<'de> Visitor<'de> for Fields {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("bounded rate metadata")
                    }
                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                        $(let mut $rf = None;)*
                        $(let mut $of = None;)*
                        $(let mut $ig = None;)*
                        while let Some(key) = map.next_key::<Secret>()? {
                            match key.as_str() {
                                $($rk => unique(&mut $rf, map.next_value::<$rt>()?)?,)*
                                $($ok => unique(&mut $of, map.next_value::<Option<$ot>>()?)?,)*
                                $($ik => unique(&mut $ig, map.next_value::<IgnoredAny>()?)?,)*
                                _ => return Err(invalid()),
                            }
                        }
                        Ok($name {
                            $($rf: required($rf)?,)*
                            $($of: presence($of),)*
                        })
                    }
                }
                d.deserialize_map(Fields)
            }
        }
    };
}

pub(in super::super) struct ShortText(pub(in super::super) Secret);
impl fmt::Debug for ShortText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ShortText([REDACTED])")
    }
}
impl<'de> Deserialize<'de> for ShortText {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = Secret::deserialize(d)?;
        if text.as_str().len() > 256 {
            return Err(invalid());
        }
        Ok(Self(text))
    }
}

macro_rules! labels {
    ($name:ident, [$($value:literal),+ $(,)?]) => {
        pub(in super::super) struct $name(pub(in super::super) &'static str);
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = Secret::deserialize(d)?;
                match text.as_str() {
                    $($value => Ok(Self($value)),)+
                    _ => Err(invalid()),
                }
            }
        }
    };
}
pub(in super::super) struct Plan(pub(in super::super) &'static str);
impl<'de> Deserialize<'de> for Plan {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = Secret::deserialize(d)?;
        plan(text.as_str()).map(Self).ok_or_else(invalid)
    }
}
labels!(
    Reached,
    [
        "rate_limit_reached",
        "workspace_owner_credits_depleted",
        "workspace_member_credits_depleted",
        "workspace_owner_usage_limit_reached",
        "workspace_member_usage_limit_reached",
    ]
);
labels!(ResetType, ["codexRateLimits", "unknown"]);
labels!(
    CreditStatus,
    ["available", "redeeming", "redeemed", "unknown"]
);

object!(Window {
    required { used_percent: i32 => "usedPercent" }
    optional { resets_at: i64 => "resetsAt", duration_mins: i64 => "windowDurationMins" }
    ignored {}
});
object!(Spend {
    required { limit: Secret => "limit", used: Secret => "used",
        remaining_percent: i32 => "remainingPercent", resets_at: i64 => "resetsAt" }
    optional {} ignored {}
});
object!(Credits {
    required { has_credits: bool => "hasCredits", unlimited: bool => "unlimited" }
    optional { balance: Secret => "balance" } ignored {}
});
object!(Snapshot {
    required {}
    optional { credits: Credits => "credits", individual: Spend => "individualLimit",
        limit_id: ShortText => "limitId", limit_name: Secret => "limitName",
        model: Secret => "normalModelSlug", plan: Plan => "planType",
        primary: Window => "primary", reached: Reached => "rateLimitReachedType",
        secondary: Window => "secondary", spend_reached: bool => "spendControlReached" }
    ignored {}
});
object!(CreditRow {
    required { granted_at: i64 => "grantedAt", id: ShortText => "id",
        reset_type: ResetType => "resetType", status: CreditStatus => "status" }
    optional { description: Secret => "description", expires_at: i64 => "expiresAt",
        title: Secret => "title" } ignored {}
});
pub(in super::super) struct Rows(pub(in super::super) Vec<CreditRow>);
impl<'de> Deserialize<'de> for Rows {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Rows;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded credit observations")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut rows = Vec::new();
                while let Some(row) = seq.next_element::<CreditRow>()? {
                    if rows.len() >= COLLECTION_LIMIT {
                        return Err(invalid());
                    }
                    rows.push(row);
                }
                Ok(Rows(rows))
            }
        }
        d.deserialize_seq(Entries)
    }
}
object!(ResetCredits {
    required { available_count: i64 => "availableCount" }
    optional { credits: Rows => "credits" } ignored {}
});
pub(in super::super) struct Buckets(pub(in super::super) Vec<(ShortText, Snapshot)>);
impl<'de> Deserialize<'de> for Buckets {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Buckets;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded rate buckets")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut rows = Vec::new();
                while let Some(key) = map.next_key::<ShortText>()? {
                    if rows.len() >= COLLECTION_LIMIT {
                        return Err(invalid());
                    }
                    rows.push((key, map.next_value::<Snapshot>()?));
                }
                Ok(Buckets(rows))
            }
        }
        d.deserialize_map(Entries)
    }
}
object!(ReadObservation {
    required { rate_limits: Snapshot => "rateLimits" }
    optional { account_id: ShortText => "accountId", ordinary_allowed: bool => "ordinaryUsageAllowed",
        buckets: Buckets => "rateLimitsByLimitId", reset_credits: ResetCredits => "rateLimitResetCredits" }
    ignored { upsell => "rateLimitUpsell" }
});
object!(Update {
    required { rate_limits: Snapshot => "rateLimits" }
    optional {} ignored {}
});
struct UpdateEnvelope(Update);
impl<'de> Deserialize<'de> for UpdateEnvelope {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = UpdateEnvelope;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("rate notification envelope")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut params = None;
                while let Some(key) = map.next_key::<Secret>()? {
                    match key.as_str() {
                        "method" => {
                            map.next_value::<Secret>()?;
                        }
                        "emittedAtMs" => {
                            map.next_value::<i64>()?;
                        }
                        "params" => unique(&mut params, map.next_value::<Update>()?)?,
                        _ => return Err(invalid()),
                    }
                }
                Ok(UpdateEnvelope(required(params)?))
            }
        }
        d.deserialize_map(Fields)
    }
}
impl Codec {
    pub(super) fn rate_notification(
        &self,
        session: &mut ProtocolSession,
        driver: &mut TransportDriver,
        bytes: &[u8],
    ) -> Result<CodecEvent, DriverError> {
        let UpdateEnvelope(update) = serde_json::from_slice(bytes)
            .map_err(|_| self.fail(session, driver, SessionError::InvalidMessage))?;
        self.gate(session, driver)?;
        // One shared signal credit; this never acknowledges cancellation or merges
        // a sparse delta into a cache or grants identity/usage authority.
        session
            .account_notification(Instant::now())
            .map_err(|_| driver.fail(session, DriverFault::Session))?;
        Ok(CodecEvent::RateLimitsUpdated(update.rate_limits))
    }
}
