# Session handoff: CA-05C continuation

## Live-state checkpoint

PR #13 is owner-merged as `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
Its final source `0ab969c4b05ffc25820f2f6c538ff510bcfa34ae` passed all six jobs in
run 36186587877, with tree `d58404dfcf67aa3f3c9b59aae36fed708f46c5d6`.
Those results are historical, not the current slice's proof. Resume branch
`feat/ca-05c-session-correlation` and its live review, not the merged branch.
`../next-pr.md` owns packet routing; `../implementation-plan.html` is the only plan.

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

After this review, separately selected CA-05D can implement login-correlation and
cancellation ordering without production schemas/IO. Operational integration still
requires the exact Q0/Q2 and native evidence in `open-asks.md`, actual deadline
scheduling and owned-helper reaping/capture. No fixture or caller boolean authorizes
those operations. Publish only the observed review branch non-forced; verify every
changed file and stop after the PR handoff. No merge, release or live account action.
