# Read-only usage and compact UI amendment

On 2026-10-21 the owner approved the feedback slice: direct Open/Create vault,
a jade plus control, compact settings, status LEDs, no em dashes, removal of
manual test acknowledgments, usage refresh with Codex open, and investigation
of monitoring inactive accounts. The owner subsequently requested clearing or
muting expired usage and monitoring reset times without clutter.

This explicit direction supersedes the earlier SPEC-PROTOCOL default against
inactive-account polling for this read-only usage feature. It does not authorize
background OAuth refresh, a service, credential writes, account switching or
an external-token runtime login. While the vault is unlocked and visible, poll
saved accounts sequentially once per minute. Closing/hiding pauses polling.
Use only the existing access token for the fixed official HTTPS usage GET.
Inactive tokens may expire; only the official sign-in flow renews them. Current
account polling can recover when Desktop itself renews its access token.

Usage percentages are observations, not official-runtime credential acceptance.
Validate identity before attribution, retain no tokens or raw responses in the
renderer, refuse redirects, cap response size/time and back off on throttling.
At reset, discard the expired window and recheck; do not manufacture 100 percent.
An expired sign-in clears visible usage, with one concise state and no watermark.

The manual I checked the account button, session acknowledgments and associated
API are removed. Existing journal wire phases remain unchanged for compatibility;
AwaitingConfirmation is presented as Account switched, not a required user task.
Recovery and actual failure outcomes remain actionable. The prior owner-reported
round trip remains historical evidence, not a UI requirement.

Sources inspected at official openai/codex revision
342ed69a174c206c1de52a59491b131e0753cfa5 (2026-10-21 EDT):
https://github.com/openai/codex/blob/342ed69a174c206c1de52a59491b131e0753cfa5/codex-rs/backend-client/src/client/rate_limit_resets.rs
https://github.com/openai/codex/blob/342ed69a174c206c1de52a59491b131e0753cfa5/codex-rs/backend-client/src/client.rs
https://github.com/openai/codex/blob/342ed69a174c206c1de52a59491b131e0753cfa5/codex-rs/codex-backend-openapi-models/src/models/rate_limit_window_snapshot.rs

This endpoint is an implementation detail of official Codex, not a promised public
API. Failure remains unavailable usage, never a reason to alter authentication.

## 2026-10-22: available banked resets

The owner requested seeing whether saved accounts have available resets. The same
read-only usage response already carries `rate_limit_reset_credits.available_count`.
Project only a validated nonnegative integer into transient renderer metadata.
Show positive counts beside the existing scheduled reset timer; missing, null,
invalid, zero and failed/expired observations add no badge. No extra request,
reset consumption, token refresh, credential write or persisted history is added.

The official client and `backend-client/src/types.rs` at the pinned revision above
establish this field on the existing usage response. Official app-server field
semantics are also documented at https://learn.chatgpt.com/docs/app-server#6-rate-limits-chatgpt.
A synthetic count proves parsing/display only; availability on the owner's account
remains an observation made by the running application.
