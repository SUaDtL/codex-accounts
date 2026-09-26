# CA-05C: bounded non-login session sequencing

CA-05D now extends this same PR with a separately selected sealed login lane;
see `ca-05d-login-sequencing.md`. The non-login contract below remains unchanged.

## Scope and construction

Continue from owner-merged PR #13, `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
That PR is closed; this one bounded follow-on retains its recovery, CI, framing,
strict JSON and static/macOS preparation work. No dependencies, manifests, lockfile,
workflow, native code, shared storage interfaces or qualification records change.

`runtime/src/session.rs` implements actual request/handshake/deadline transitions.
`ProtocolSession` has no production constructor, Default, Clone, deserializer or
reset. Its sole constructor is cfg(test)-only; a compile-fail doctest enforces that
boundary. The methods are compiled in normal builds but cannot be reached through
a production instance. A later reviewed adapter must establish construction and
integration prerequisites, not turn a test flag or event into authority.

The input is a normalized internal event, not raw JSON. CA-05B validates object
syntax; it does not validate an official message schema. This slice deliberately
chooses no wire-ID representation, extracts no field from a helper, and launches no
process. Local request tickets bind an issuing session lifetime and a never-reused
sequence. They are not wire IDs, credential IDs, cryptographic receipts or proof
that another process sent a response. Mapping exact wire IDs to these tickets and
binding each reader to its original private pipe remain explicit integration work.

## Implemented behavior

An initialize request must be reserved, acknowledged as sent and matched with a
successful normalized response. Account requests still cannot proceed until the
future driver acknowledges the initialized notification write. Duplicate initialize,
early/repeated initialized, unsent responses, duplicate send, unknown/duplicate
responses and another session's tickets stop the session. Multiple account/rate-
limit requests can complete in any order without consuming each other's tickets.

Only Initialize, AccountRead and RateLimitsRead requests are modeled. A RateLimitsRead
ticket is not consent to contact a provider. LoginStart, LoginCancel and Initialized
as a request refuse. Login IDs, early login completion, the ten-minute login budget
and cancellation acknowledgement require the separately bounded CA-05D slice.

Account-change notifications are content-free signals, not identity observations.
Early signals after initialize is sent are buffered without completing the handshake.
At most 32 may be pending; consumption returns credit. Signals, partial send timing,
new requests and successful responses do not extend any absolute deadline.

All event entry points check time before processing. Initialization is bounded by
10 seconds through the initialized write, requests by 15 seconds from reservation,
and the entire non-login session by 30 seconds INCLUDING initialization. The latter
is a conservative packet choice; it does not grant a fresh 30 seconds after ready.
At exact equality a deadline expires. Earliest-expiry polling includes unsent work.
An expired older request blocks accepting a timely newer reply. Backwards clock
observations fail closed. Checked arithmetic avoids silently saturating a reversed
clock or panicking on an unrepresentable deadline.

Eight simultaneous requests and 256 issued requests per session (including
initialize) are additional conservative implementation bounds, not amendments to
the product specification or claims about the official server's capacity. Completed
requests release pending credit but never reset IDs or the lifetime cap.

Cancellation, malformed input, transport failure, unexpected server requests and
normalized remote errors stop the session, clear outstanding state and preserve the
first fixed error. No error is interpreted as revoked credentials or Desktop status.
Clean protocol EOF requires completed handshake, no outstanding request and drained
signals. It is terminal/idempotent, but is NOT owned-helper exit, safe credential
restoration, captured generations, successful login or Desktop confirmation.

## Scheduler and clock boundary

There is no thread, timer task, sleep or blocking IO in this state machine. A future
driver must schedule the returned earliest deadline, use fresh clock readings on
every event, bound each read/write and stop/reap/capture through the existing
coordinator after any possible helper write. A silent blocked reader cannot be
fixed by a state object which is never polled. No fixture proves scheduling or exit.

The Rust Instant contract warns that suspend accounting varies by platform/version
and checked_duration_since can expose reversed/nonmonotonic observations. This
implementation uses that check and checked_add, with the repository-pinned 1.90.0
toolchain. Suspend/resume handling is a future native-driver qualification gate,
not an assertion of wall-clock enforcement on the owner's machine.
Primary API reference: https://doc.rust-lang.org/std/time/struct.Instant.html

## Evidence and reproduction

Twenty-four deterministic Rust tests execute the real state machine without sleeps:
all six orders of three responses, every incomplete handshake boundary, duplicate/
foreign/unsent IDs, pending/lifetime limits, early/flooded signals, exact deadline
edges, oldest-request expiry, total-budget starvation, every timed entry point,
clock regression/jump, transport/cancellation/error precedence and terminal EOF.
Four Python source-boundary regressions supplement, not replace, those executions.
All 34 previous runtime tests remain; the runtime now has 58 ordinary cases and two
compile-fail doctests. The existing workspace, release and isolated Linux lanes
collect them automatically. No native-case partition or blocking CI check changes.

From the reviewed checkout, after the unchanged dependency review:

```text
python tools/check_dependencies.py
cargo test -p codex-accounts-runtime --locked --offline
cargo test -p codex-accounts-runtime --release --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
python -m unittest discover -s tests/python -v
```

The final PR identifies its exact source/tested tree and inspected completed checks.
The local runtime tests establish model behavior only. Hosted native tests still
must run; ordinary-user/two-user, independent macOS, exact Desktop Q0 and power-loss
qualification remain separate. No whole T-24/T-25/T-26/T-31 scenario is closed.
No real account, token, login URL, user path, helper output or support export is an
input to this slice. Public Debug output contains only fixed redacted type names.
