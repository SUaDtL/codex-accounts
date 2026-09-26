# CA-05D: bounded login correlation and cancellation

## Review and scope

This increment continues CA-05C on the same PR #14 and branch
`feat/ca-05c-session-correlation`. Its observed parent is
`db7fcfced20c3fab1f602991fd6e56712ec016b6`; main remains the owner-merged PR #13.
No dependencies, manifests, lockfile, native code, storage formats, specification,
workflow, native-test inventory or qualified catalog records change.

`session_login.rs` extends the existing ProtocolSession, pending-request allocator,
request-ID ownership, sent checks and sticky-error implementation. It is not a
second transport or credential transaction. Both normal and login construction
remain test-only. LoginId has no public constructor or byte getter; compile-fail
tests enforce these boundaries. Normal `reserve` still refuses login enum values.
A method name, successful unit test or caller flag cannot enable production login.

## Matching and reordering

A dedicated sealed login lane permits one login-start reservation only after the
existing handshake completes, with no pending requests or undrained account signals.
The start request must be acknowledged as sent before a start reply or early
completion can be accepted. The correlated start reply establishes the expected
login ID. A completion received earlier is buffered, not reported as success.
Every buffered ID must match that reply; foreign-session, wrong, duplicate and
conflicting IDs refuse. Start and cancel responses must use their original sent
request tickets; the generic response method cannot bypass specialized checks.

IDs are opaque exact bytes with a session/reader owner and a conservative 1..256-byte
local bound. They are zeroizing-owned and redacted. This is not an asserted official
wire type, upstream limit, account identity or cryptographic origin proof. The later
version-pinned decoder must validate and bind actual messages to their original
private reader before creating these internal values. No raw message parsing,
credentials or login URL handling is added by this slice.

Account signals and early login completions share the existing maximum of 32 pending
notifications. Draining account signals returns credit; wrong IDs cannot be dropped
silently to turn a mixed batch into a successful match. After a matching reported
success, account requests and another login remain unavailable. Protocol close
requires drained signals. A report is not credential acceptance, helper exit,
captured generation, completed onboarding, or Desktop confirmation.

## Time and cancellation

The start reply keeps the existing 15-second request bound, capped by the original
30-second pre-login session budget. Interactive waiting receives one absolute
600-second budget starting at login reservation. Progress does not reset it; the
30-second non-login limit is not incorrectly applied to the browser-wait phase.
At exact equality timeout wins over a simultaneous completion.

Cancel/interactive-timeout records the first primary reason and enters a distinct
shutdown-only lane. Ordinary operations keep returning that reason. If a sent start
has not supplied its ID, cancellation may wait only until the earlier of its original
reply deadline or five seconds from cancellation. A late matching start reply can
supply the cancel target but cannot enable login success. Before a start is sent,
or after completion was already reported, matching cancellation is unavailable;
the session stops without inventing an ID.

Once known, the matching target can produce at most one cancel ticket through the
same bounded request allocator. That ticket must be sent before its matching reply
is accepted. The cancellation deadline is fixed at intent time, including queued
send time. Five seconds is a conservative protocol-cleanup bound, not extra time to
complete login, an upstream guarantee or a new normative timeout. It can extend
past the interactive deadline only for cancellation/cleanup; no further login or
account operation is then admitted.

Cancellation acknowledged, failed, timed out and unavailable are distinct from the
primary Cancelled/LoginTimeout reason. A late successful completion never undoes
cancellation, including all six orderings of start reply, completion and cancel.
Malformed input, transport failure, wrong acknowledgement, missing EOF acknowledgement
and clock regression retain primary and secondary outcomes separately. Repeated
cancel requests do not reset time; successful acknowledgement remains a stopped
operation, never a successful login or helper-exit receipt.

The synchronous sequencer creates no timers or IO. After a cancel/timeout error, the
future driver uses `poll_cancellation` and the fixed cancellation state, not repeated
ordinary `poll` as a supposed success. It must wake at deadlines, bound IO, handle
suspend/resume and stop/reap the owned helper whether cancellation is acknowledged
or not. After any possible helper write, the existing coordinator must preserve the
newest generation before restoration or further mutation. No native reaping,
credential capture, staging cleanup or recovery wiring is implemented here.

## Tests and validation

Thirty new deterministic Rust cases cover both completion orders, all six cancel
races, wrong/foreign/duplicate/unsent IDs, shared notification bounds, exact expiry,
late start and completion, cancellation credit/lifetime bounds, primary/secondary
errors and terminal behavior. All 58 prior runtime cases are retained. Runtime total:
88 ordinary tests and four compile-fail doctests. Four Python regressions supplement
construction, existing-sequencer reuse, shared bounds and no-native-authority checks.
No added test is ignored and no workflow check is weakened.

```text
python tools/check_dependencies.py
cargo test -p codex-accounts-runtime --locked --offline
cargo test -p codex-accounts-runtime --release --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
python -m unittest discover -s tests/python -v
```

The PR records exact final source/tested tree and completed hosted checks. All
inherited native cases still run; passing model events does not close full
T-24/T-25/T-26/T-31, exact Desktop Q0, scheduler/private-pipe integration or native
qualification. No real accounts, auth files, browser URLs or helper logs are inputs.
Next work must implement a bounded private synthetic driver/IO boundary or a
separately approved exact-schema adapter, not enable this test constructor.
