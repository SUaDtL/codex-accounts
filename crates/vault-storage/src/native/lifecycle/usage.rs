//! Read-only usage observation. No helper, token refresh, live auth writes or
//! account-association mutations. Only access tokens reach the fixed transport.
use super::{runtime_context, target::Target, usage_http};
use crate::{auth_capture, AccountHealth, OperationError, ProfileId, StorageError, Vault};
use codex_accounts_core::CredentialAcceptance;
use codex_accounts_runtime::{Observed, RateLimitSummary, RateLimitWindow};
use codex_accounts_vault::{validate_json_object, ResourceId};
use serde::Deserialize;

impl Vault {
    // Bind all outbound requests to the vault's selected profile. An external
    // Desktop sign-in must not redirect a request for a saved inactive account.
    // Only the access token is read; neither refresh nor auth file writes occur.
    fn usage_credentials_for(
        &self,
        requested: Option<ProfileId>,
    ) -> Result<(zeroize::Zeroizing<String>, String, ProfileId), OperationError> {
        self.storage.ensure_operation_idle()?;
        runtime_context::check_environment()?;
        let live = (|| {
            let home = runtime_context::selected_home()?;
            let target = Target::open_file_backend(&home)?;
            let bytes = target.read_usage_auth()?;
            auth_capture::usage_credentials(&bytes).map_err(OperationError::from)
        })();
        let use_live = match (&live, requested) {
            (Ok((_, identity)), Some(profile)) => self
                .storage
                .current_reference(identity)
                .is_ok_and(|reference| reference.profile == profile.to_bytes()),
            (_, None) => true,
            _ => false,
        };
        let (token, identity) = if use_live {
            live?
        } else {
            drop(live);
            let profile = requested.ok_or(OperationError("E_USAGE_UNAVAILABLE"))?;
            let saved = self.storage.read_latest(&self.root, profile)?;
            let bytes = saved
                .resource(ResourceId::new(0).map_err(StorageError::from)?)
                .and_then(|resource| resource.as_bytes())
                .ok_or(OperationError("E_USAGE_UNAVAILABLE"))?;
            auth_capture::usage_credentials(bytes)?
        };
        let reference = self
            .storage
            .current_reference(&identity)
            .map_err(|_| OperationError("E_CAPTURE_CURRENT_FIRST"))?;
        let profile = ProfileId::from_bytes(reference.profile).map_err(StorageError::from)?;
        if requested.is_some_and(|expected| expected != profile) {
            return Err(OperationError("E_EXTERNAL_CHANGE"));
        }
        Ok((token, identity.components().2.to_owned(), profile))
    }

    pub fn refresh_usage(
        &self,
        requested: Option<ProfileId>,
    ) -> Result<AccountHealth, OperationError> {
        let (token, account_id, profile) = self.usage_credentials_for(requested)?;
        let bytes = usage_http::fetch(&token, &account_id).map_err(OperationError)?;
        let (quotas, available_resets) =
            parse_usage(&bytes, &account_id).map_err(OperationError)?;
        Ok(AccountHealth {
            profile,
            acceptance: CredentialAcceptance::Unknown,
            quotas: Some(quotas),
            available_resets,
            checked_at: crate::records::now()?,
        })
    }

    /// Explicit, account-bound, single-credit redemption. This intentionally
    /// leaves the installed Codex home and the encrypted vault unchanged.
    /// Never retry an uncertain POST automatically.
    pub fn redeem_rate_limit_reset(
        &self,
        requested: ProfileId,
    ) -> Result<crate::ResetRedemption, OperationError> {
        let (token, account_id, profile) = self.usage_credentials_for(Some(requested))?;
        let details = usage_http::fetch_credits(&token, &account_id)
            .map_err(|error| match error {
                "E_USAGE_EXPIRED" => OperationError("E_RESET_EXPIRED"),
                "E_USAGE_THROTTLED" => OperationError("E_RESET_THROTTLED"),
                _ => OperationError("E_RESET_UNAVAILABLE"),
            })?;
        let credit = selected_reset_credit(&details).map_err(OperationError)?;
        // A stable per-profile/per-credit key makes a retry of the very same
        // earned reset idempotent across application restarts.
        let key = redemption_key(profile, &credit.id);
        let body = serde_json::to_vec(&serde_json::json!({
            "redeem_request_id": key,
            "credit_id": credit.id,
        }))
        .map_err(|_| OperationError("E_RESET_UNAVAILABLE"))?;
        let response = usage_http::consume_credit(&token, &account_id, &body)
            .map_err(|error| match error {
                "E_USAGE_EXPIRED" => OperationError("E_RESET_EXPIRED"),
                _ => OperationError("E_RESET_OUTCOME_UNKNOWN"),
            })?;
        let (outcome, windows_reset) = parse_reset_response(&response)
            .map_err(|_| OperationError("E_RESET_OUTCOME_UNKNOWN"))?;
        Ok(crate::ResetRedemption {
            profile,
            outcome,
            windows_reset,
        })
    }
}

#[derive(Deserialize)]
struct CreditDetails {
    available_count: i64,
    credits: Vec<ResetCredit>,
}

#[derive(Deserialize)]
struct ResetCredit {
    id: String,
    reset_type: String,
    status: String,
    expires_at: Option<String>,
}

fn selected_reset_credit(bytes: &[u8]) -> Result<ResetCredit, &'static str> {
    validate_json_object(bytes).map_err(|_| "E_RESET_UNAVAILABLE")?;
    let details: CreditDetails =
        serde_json::from_slice(bytes).map_err(|_| "E_RESET_UNAVAILABLE")?;
    if details.available_count <= 0 {
        return Err("E_RESET_NO_CREDIT");
    }
    if details.available_count > u32::MAX as i64 || details.credits.len() > 128 {
        return Err("E_RESET_UNAVAILABLE");
    }
    // The backend can cap the detailed rows. Never consume an unspecified
    // credit if we cannot identify a supported available one.
    let credit = details
        .credits
        .into_iter()
        .filter(|credit| {
            credit.status == "available"
                && credit.reset_type == "codex_rate_limits"
                && !credit.id.is_empty()
                && credit.id.len() <= 256
                && credit.id.bytes().all(|byte| byte.is_ascii_graphic())
        })
        .min_by(|a, b| match (a.expires_at.as_deref(), b.expires_at.as_deref()) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .ok_or("E_RESET_UNAVAILABLE")?;
    Ok(credit)
}

fn redemption_key(profile: ProfileId, credit_id: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let mut hash = Sha256::new();
    hash.update(b"CodexAccounts:rate-limit-reset:v1");
    hash.update(profile.to_bytes());
    hash.update(credit_id.as_bytes());
    let mut key = String::from("ca-");
    for byte in hash.finalize() {
        let _ = write!(&mut key, "{byte:02x}");
    }
    key
}

#[derive(Deserialize)]
struct ResetResponse {
    code: String,
    #[serde(default)]
    windows_reset: i64,
}

fn parse_reset_response(bytes: &[u8]) -> Result<(&'static str, u32), &'static str> {
    validate_json_object(bytes).map_err(|_| "E_RESET_OUTCOME_UNKNOWN")?;
    let response: ResetResponse =
        serde_json::from_slice(bytes).map_err(|_| "E_RESET_OUTCOME_UNKNOWN")?;
    let outcome = match response.code.as_str() {
        "reset" => "reset",
        "already_redeemed" => "already_redeemed",
        "nothing_to_reset" => "nothing_to_reset",
        "no_credit" => "no_credit",
        _ => return Err("E_RESET_OUTCOME_UNKNOWN"),
    };
    let windows_reset =
        u32::try_from(response.windows_reset).map_err(|_| "E_RESET_OUTCOME_UNKNOWN")?;
    Ok((outcome, windows_reset))
}

#[derive(Deserialize)]
struct Usage {
    account_id: Option<String>,
    rate_limit: Option<Limits>,
    // Optional display metadata must not make valid quota windows unavailable.
    rate_limit_reset_credits: Option<serde_json::Value>,
}
#[derive(Deserialize)]
struct Limits {
    primary_window: Option<Window>,
    secondary_window: Option<Window>,
}
#[derive(Deserialize)]
struct Window {
    used_percent: i32,
    limit_window_seconds: i64,
    reset_at: i64,
}
fn parse_usage(
    bytes: &[u8],
    expected_account: &str,
) -> Result<(RateLimitSummary, Option<u32>), &'static str> {
    const INVALID: &str = "E_USAGE_UNAVAILABLE";
    validate_json_object(bytes).map_err(|_| INVALID)?;
    let usage: Usage = serde_json::from_slice(bytes).map_err(|_| INVALID)?;
    if usage
        .account_id
        .as_deref()
        .is_some_and(|id| id != expected_account)
    {
        return Err("E_EXTERNAL_CHANGE");
    }
    fn window(value: Option<Window>) -> Result<Observed<RateLimitWindow>, &'static str> {
        let Some(value) = value else {
            return Ok(Observed::Missing);
        };
        if !(0..=100).contains(&value.used_percent)
            || value.limit_window_seconds <= 0
            || value.reset_at <= 0
        {
            return Err("E_USAGE_UNAVAILABLE");
        }
        Ok(Observed::Value(RateLimitWindow {
            used_percent: value.used_percent,
            duration_mins: if value.limit_window_seconds % 60 == 0 {
                Observed::Value(value.limit_window_seconds / 60)
            } else {
                Observed::Missing
            },
            resets_at: Observed::Value(value.reset_at),
        }))
    }
    let (primary, secondary) = match usage.rate_limit {
        Some(limits) => (
            window(limits.primary_window)?,
            window(limits.secondary_window)?,
        ),
        None => (Observed::Missing, Observed::Missing),
    };
    let available_resets = usage
        .rate_limit_reset_credits
        .as_ref()
        .and_then(|value| value.get("available_count"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    Ok((
        RateLimitSummary {
            primary,
            secondary,
            plan: Observed::Missing,
        },
        available_resets,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usage_never_invents_missing_windows_or_accepts_wrong_identity() {
        let (absent, resets) = parse_usage(br#"{"rate_limit":null}"#, "synthetic").unwrap();
        assert_eq!(resets, None);
        assert_eq!(absent.primary, Observed::Missing);
        assert!(parse_usage(br#"{"account_id":"other","rate_limit":null}"#, "synthetic").is_err());
        assert!(parse_usage(br#"{"rate_limit":null,"rate_limit":{}}"#, "synthetic").is_err());
        let (full, _) = parse_usage(br#"{"account_id":"synthetic","rate_limit":{"primary_window":{"used_percent":37,"limit_window_seconds":18000,"reset_at":1900000000},"secondary_window":null}}"#, "synthetic").unwrap();
        assert_eq!(
            full.primary,
            Observed::Value(RateLimitWindow {
                used_percent: 37,
                duration_mins: Observed::Value(300),
                resets_at: Observed::Value(1900000000)
            })
        );
        assert_eq!(full.secondary, Observed::Missing);
        assert!(parse_usage(br#"{"rate_limit":{"primary_window":{"used_percent":101,"limit_window_seconds":0,"reset_at":-1}}}"#, "synthetic").is_err());
    }

    #[test]
    fn banked_resets_preserve_observed_zero_and_ignore_invalid_optional_metadata() {
        for (metadata, expected) in [
            (r#"{"available_count":2}"#, Some(2)),
            (r#"{"available_count":0}"#, Some(0)),
            (r#"{"available_count":-1}"#, None),
            (r#"{"available_count":4294967296}"#, None),
            (r#"{"available_count":"2"}"#, None),
            (r#"{"available_count":2.5}"#, None),
            (r#"{"available_count":null}"#, None),
            (r#"{"credits":[{},{}]}"#, None),
            ("null", None),
        ] {
            let body = format!(r#"{{"rate_limit":null,"rate_limit_reset_credits":{metadata}}}"#);
            assert_eq!(
                parse_usage(body.as_bytes(), "synthetic").unwrap().1,
                expected
            );
        }
    }

    #[test]
    fn picks_supported_available_earliest_expiring_credit() {
        let fixture = br#"{
            "available_count": 3,
            "credits": [
                {"id":"newer","reset_type":"codex_rate_limits","status":"available","expires_at":"2026-12-01T00:00:00Z"},
                {"id":"used","reset_type":"codex_rate_limits","status":"redeemed","expires_at":null},
                {"id":"earlier","reset_type":"codex_rate_limits","status":"available","expires_at":"2026-11-01T00:00:00Z"}
            ]
        }"#;
        assert_eq!(selected_reset_credit(fixture).unwrap().id, "earlier");
        assert_eq!(
            selected_reset_credit(br#"{"available_count":0,"credits":[]}"#).err(),
            Some("E_RESET_NO_CREDIT")
        );
        assert_eq!(
            selected_reset_credit(br#"{"available_count":1,"credits":[]}"#).err(),
            Some("E_RESET_UNAVAILABLE")
        );
    }

    #[test]
    fn reset_result_is_strict_and_idempotency_is_credit_bound() {
        assert_eq!(
            parse_reset_response(br#"{"code":"reset","windows_reset":2}"#).unwrap(),
            ("reset", 2)
        );
        assert_eq!(
            parse_reset_response(br#"{"code":"already_redeemed"}"#).unwrap(),
            ("already_redeemed", 0)
        );
        assert!(parse_reset_response(br#"{"code":"other"}"#).is_err());
        assert!(parse_reset_response(br#"{"code":"reset","windows_reset":-1}"#).is_err());
        let profile = ProfileId::from_bytes([1; 16]).unwrap();
        assert_eq!(redemption_key(profile, "credit-a"), redemption_key(profile, "credit-a"));
        assert_ne!(redemption_key(profile, "credit-a"), redemption_key(profile, "credit-b"));
    }
}
