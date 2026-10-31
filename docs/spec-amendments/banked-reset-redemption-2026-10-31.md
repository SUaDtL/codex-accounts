# Owner-approved manual redemption of banked Codex resets

Decision: 2026-10-31. The owner explicitly requested implementing an in-app action to
use an available earned reset without visiting the Codex web/Desktop usage UI.
This adds a **new explicit user action**; existing background usage polling remains read-only.

## Scope

- An opened, unlocked vault permits a user to select **Use reset** for a saved
  account with a positively observed available reset count, including inactive accounts.
- Require a confirmation identifying the selected account. No automatic redemption,
  scheduling, account rotation, credential CLI, or external HTTP/credential API.
- Native vault worker owns account selection and outgoing transport. The renderer
  supplies only a validated vault profile ID, never tokens, paths, headers, backend
  credit identifiers or idempotency keys.
- Validate selected profile-to-credential identity before use. Use only that profile's
  existing access token and fixed ChatGPT account ID. Do not refresh OAuth tokens,
  write credentials, or change the installed Codex Desktop account.
- Immediately before redemption, read individual available credit details using a
  fixed first-party endpoint. Require the expected reset type and available status,
  with a bounded opaque ID. Prefer the earliest-expiring credit. No verified details
  means no POST; a count alone never authorizes consumption.
- Send exactly one bounded, user-confirmed POST to the fixed endpoint used by the
  official Codex backend client. Never follow redirects, send cookies, automatically
  retry, or accept arbitrary request URLs.
- Derive a stable SHA-256 idempotency key from a scoped namespace, saved profile ID,
  and the chosen backend credit ID. Retrying that same credit across launches uses
  the same key, but never silently switch to another credit to recover a lost reply.
- Accept only documented outcomes: reset, already_redeemed, nothing_to_reset,
  no_credit. Report unknown or ambiguous outcomes truthfully; do not infer success.
- Serialize with native vault operations and respect unfinished recovery blockers.
- No credentials, reset-credit identifiers, headers or raw service bodies enter UI
  IPC, logs, telemetry or diagnostics. Tests use synthetic data only.
- Routine usage polling remains a read-only GET and never consumes credits.

## Compatibility and limitations

The first-party endpoints /backend-api/wham/rate-limit-reset-credits and
/backend-api/wham/rate-limit-reset-credits/consume are used by official Codex
implementations but are not guaranteed public APIs. Fail closed when changed.

Only the service determines if a usage window can be reset. A lost POST reply may
mean a credit was consumed. The UI blocks further attempts on that profile for the
current app session following an uncertain outcome and tells the user to review
official usage. This initial implementation does not include a durable journal of
uncertain requests: restarting the app is not evidence the previous request failed.
This limitation needs acceptance/review before a stable release.

## Validation gates

- Synthetic GET/POST parsing, credit selection, idempotency, unknown-outcome tests.
- UI no-command-before-confirmation tests for account menu and tray.
- Native Windows checks for account binding, expired tokens, HTTP timeout/429/401,
  redirects, bounded responses, app-close race and no installed-auth mutation.
- Real-account consumption must be separately approved; neither CI nor this PR
  should consume a user's saved reset.
