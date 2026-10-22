//! Transient observations from a current-account check, not persisted identity
//! assertions or inferred health for other saved profiles.
#![forbid(unsafe_code)]

use crate::ProfileId;
use codex_accounts_core::CredentialAcceptance;
use codex_accounts_runtime::RateLimitSummary;

pub struct AccountHealth {
    pub profile: ProfileId,
    pub acceptance: CredentialAcceptance,
    pub quotas: Option<RateLimitSummary>,
    /// Banked resets reported by the service; missing is not zero.
    pub available_resets: Option<u32>,
    /// Completed observation time as Unix seconds, separate from capture time.
    pub checked_at: u64,
}
impl std::fmt::Debug for AccountHealth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AccountHealth([REDACTED])")
    }
}
