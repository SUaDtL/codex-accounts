# Session handoff: CA-05C/CA-05D on PR #14

## Live-state checkpoint

PR #13 is owner-merged as `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
Its final source `0ab969c4b05ffc25820f2f6c538ff510bcfa34ae` passed all six jobs in
run 36186587877, with tree `d58404dfcf67aa3f3c9b59aae36fed708f46c5d6`.
Those results are historical, not the current slice's proof. Resume branch
`feat/ca-05c-session-correlation` and its live review, not the merged branch.
`../next-pr.md` owns packet routing; `../implementation-plan.html` is the only plan.

CA-05C source `db7fcfced20c3fab1f602991fd6e56712ec016b6` passed run 36192783799.
CA-05D extends that same open PR #14; this historical pass is not the new head's
proof. `session_login.rs` adds bounded login IDs, early completion and a separate
cancellation lane using the existing request allocator and sent/response checks.
Thirty new Rust cases and four Python boundary checks join normal CI; old tests
remain intact. Read `../ca-05d-login-sequencing.md` before extending the driver.

## Existing code and this increment

Preserve the merged discovery, protected vault, immutable generations, encrypted
storage, write-ahead coordinator, CA-04B/C native primitives and CA-04D explicit
staging repair. CAREG003 and its prior-format compatibility are unchanged.
The compiled workspace/native CI partition and two-worker verifier are unchanged.
CA-05A framing, CA-05B strict JSON, CA-06A static pages and macOS source preparation
are retained. Do not replace any of them with a second foundation or older archive.

CA-05C adds `runtime/src/session.rs` and `session_tests.rs`: real bounded non-login
state transitions with test-only construction, local session-owned request tickets,
monotonic deadline checks and fixed terminal errors. They are normalized internal
events, not parsed official messages. See `../ca-05c-session-sequencing.md` for exact
contracts, bounds and the scheduler/clock/IO limitations. No production construction,
credential IO, helper launch, login or qualification is enabled.

## Validation and handoff

Use pinned Rust 1.90.0, unchanged dependency review and offline builds. New runtime
cases join ordinary debug/release suites and isolated Linux automatically. All 27
named Windows behaviors remain separately required in each profile. A workspace
complement alone is not complete native evidence. Three two-user helpers remain
NOT RUN. macOS compilation is not Keychain/Desktop qualification. Browser/keyboard/
zoom/screen-reader, exact Desktop and power-loss trials remain separate.

The current PR owns exact final source/test merge/tree/run and inspected logs;
`../validation.md` preserves evidence boundaries. No pass from PR #13 transfers.
No new dependencies, workflow skip, native binding or changed shared-source digest
is part of CA-05C. The macOS v2 baseline remains unchanged and still source-only.

After review, separately selected CA-05E can implement the private synthetic
transport-driver boundary and real deadline/IO scheduling, including bounded stderr
discard and cancellation. Official schema, browser opener, owned-helper stop/reap,
newest-generation capture and recovery integration remain separate reviewed work.
Do not expose test constructors, treat normalized values as proof or change the
unchanged native inventory. Only publish this review branch non-forced; verify the
full combined diff and return its actual head/checks. No merge or live account action.
