//! Read-only usage observation. No helper, token refresh, live auth writes or
//! account-association mutations. Only access tokens reach the fixed transport.
use super::{runtime_context, target::Target, usage_http};
use crate::{auth_capture, AccountHealth, OperationError, ProfileId, StorageError, Vault};
use codex_accounts_core::CredentialAcceptance;
use codex_accounts_runtime::{Observed, RateLimitSummary, RateLimitWindow};
use codex_accounts_vault::{validate_json_object, ResourceId};
use serde::Deserialize;

impl Vault {
    pub fn refresh_usage(
        &self,
        requested: Option<ProfileId>,
    ) -> Result<AccountHealth, OperationError> {
        self.storage.ensure_operation_idle()?;
        runtime_context::check_environment()?;
        let live = (|| {
            // Sharing permits Desktop's refresh/atomic replacement. All handles
            // are dropped before HTTP; no locks are held across network waits.
            let home = runtime_context::selected_home()?;
            let target = Target::open_file_backend(&home)?;
            let bytes = target.read_usage_auth()?;
            let credentials = auth_capture::usage_credentials(&bytes)?;
            Ok::<_, OperationError>(credentials)
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
            // An outside sign-in may have changed Desktop's current identity.
            // A requested saved profile remains observable without overwriting
            // live credentials or attributing another account's usage to it.
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
        let account_id = identity.components().2;
        let bytes = usage_http::fetch(&token, account_id).map_err(OperationError)?;
        let (quotas, available_resets) = parse_usage(&bytes, account_id).map_err(OperationError)?;
        Ok(AccountHealth {
            profile,
            // A usage response is not an official-runtime credential check.
            acceptance: CredentialAcceptance::Unknown,
            quotas: Some(quotas),
            available_resets,
            checked_at: crate::records::now()?,
        })
    }
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
}
