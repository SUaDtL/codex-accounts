# Session handoff: registered runtime provenance and schema preparation

As of 2026-10-15, PRs #14 through #18 are merged into main
`b734f0c0f02010e59c736d1abc758172cd9e171b`. The active branch is
`codex/runtime-publisher-evidence`. Read
[the current packet](../next-pr.md) and
[Windows storage compatibility](../windows-storage-compatibility.md) first.
The owner authorized continued direct development outside engine approval bookkeeping;
the audit log retains that instruction. Preserve pending engine records without
claiming acceptance. Security, qualification and exact-head CI remain required.

PR #17 preserves the reader's no-execution boundary while adding a retained,
explicit nominated-image observation. PR #18 supervises native test process
families and retains bounded failure output; it does not explain away the two
earlier main restart-test failures. See the validation record for exact CI scope.
The [local Q0 experiment](../runtime-provenance-observation.md) records public
package/image equality and one contained original-image version result. The
separately reviewed schema collector then admitted 317 generated files from one
contained original invocation with exit and cleanup verified. CA-05F owns private
handshake/account-read decoding from the pinned schema; full Q0 and production authority
remain open. Never substitute the private experiment for a shipped adapter.

The following review-stack checkpoint is historical.

## Live-state checkpoint

PR #13 is owner-merged as `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
Its final source `0ab969c4b05ffc25820f2f6c538ff510bcfa34ae` passed all six jobs in
run 36186587877, with tree `d58404dfcf67aa3f3c9b59aae36fed708f46c5d6`.
Those results are historical, not the current slice's proof. PR #14's head is
`e4dbe1242dd0f3238c323570dcf274fbb655c89b`; PR #15's preceding head is
`caeb46c42efe4ff22926c3d7ac817a580556c59e` on `feat/ca-05e-synthetic-driver`.
Resume the live stack, with PR #14 preceding PR #15 and fresh base/head checks
after any prerequisite merge.
`../next-pr.md` owns packet routing; `../implementation-plan.html` is the only plan.

CA-05C source `db7fcfced20c3fab1f602991fd6e56712ec016b6` passed run 36192783799.
CA-05D extends that same open PR #14; this historical pass is not the new head's
proof. `session_login.rs` adds bounded login IDs, early completion and a separate
cancellation lane using the existing request allocator and sent/response checks.
Thirty new Rust cases and four Python boundary checks join normal CI; old tests
remain intact. Read `../ca-05d-login-sequencing.md` before extending the driver.

CA-05E adds the private in-memory transport pump, not native pipes or an official
helper. This follow-up fixes two test-proof gaps: full-queue release/wakeup with
exact bytes, and cancellation/timeout after partial output. Five isolated fault
fixtures fail for the intended reasons. Local debug and release each pass 118
ordinary runtime cases and five compile-fail doctests. See
`../ca-05e-transport-driver.md` for the preceding exact-head CI and current proof.

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

The codeArbiter scaffold remains uninitialized: contained context-scout support on
this host is tracked in [codeArbiter #950](https://github.com/arbiterForge/codeArbiter/issues/950).
The owner authorized a logged missing-context preflight override only for this
PR's test/documentation follow-up, commit and update. No context sentinel,
provenance or approval receipt was invented; numeric maturity/coverage is unassessed.
The override does not waive application qualification, security or CI gates.

Retain the synthetic transport boundary and real deadline/IO scheduling, including
bounded stderr discard and cancellation. Official schema, browser opener, owned-helper stop/reap,
newest-generation capture and recovery integration remain separate reviewed work.
Do not expose test constructors, treat normalized values as proof or change the
unchanged native inventory. Only publish this review branch non-forced; verify the
full combined diff and return its actual head/checks. The owner subsequently
authorized merge only after completion and green gates. Production qualification
and live account actions remain separately gated.
