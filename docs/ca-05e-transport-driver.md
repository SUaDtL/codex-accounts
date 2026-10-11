# CA-05E: private synthetic transport pump

Checkpoint: **2026-10-10**. Draft [PR #15](https://github.com/SUaDtL/codex-accounts/pull/15)
extends unmerged [PR #14](https://github.com/SUaDtL/codex-accounts/pull/14).
Its reviewed code head is `caeb46c42efe4ff22926c3d7ac817a580556c59e`;
the prerequisite is `e4dbe1242dd0f3238c323570dcf274fbb655c89b`.
The existing `ProtocolSession` owns sequencing, tickets, deadlines and outcomes.
CA-05E adds scheduling and bounded in-memory IO, not another transaction engine.

## Implemented contract

`crates/runtime/src/driver.rs` pumps owned endpoints from `driver_io.rs`:

- Stdin, stdout and stderr each have 16 queued chunks of at most 4096 bytes.
  Data writes use `try_send`; a full queue reports backpressure. One coalesced
  wake channel and a separate sticky atomic bit preserve cancellation intent.
- `Instant` and `recv_timeout` enforce the sequencer's earliest absolute deadline
  during silence, partial writes and stderr activity. Activity never resets it.
  Expiry precedes simultaneous cancellation or queued input.
- A request becomes sent only after its whole frame enters the synthetic channel.
  Partial writes retain their original ticket/deadline. Channel acceptance is
  not native write/flush or helper-observation evidence.
- The existing strict decoder checks newline framing, UTF-8, bounded JSON objects
  and duplicate keys. At most one frame is leased to the reader. Endpoints and
  leases bind to the original private reader and session, not matching bytes alone.
- Raw buffers are zeroizing-owned; diagnostics contain fixed error categories.
  Stderr is discarded independently without protocol interpretation or forwarding,
  with a 4 MiB lifetime work cap. This cap is a local conservative limit.
- Stdout and stderr EOF are separate. Both streams closing still requires complete
  session state. First failures stay terminal; cancellation outcomes remain separate
  from the original reason. Protocol closure never asserts process exit.
- No production constructor, official schema, OS pipe, process launch, browser
  action, credentials, filesystem/network IO or public renderer entry point exists.
  The pump owns no worker thread; threaded test peers are scoped and joined.

The initial 28 Rust cases and five Python source checks supplement the retained
sequencer/framing/JSON suites. They cover deadlines, cancellation, backpressure,
framing bounds, redaction, binding and EOF. They do not close a full T-xx scenario.
The normative obligations remain S-011/S-012/F-017/F-019 and the related
T-24/T-25/T-26/T-31 records in the canonical specification.

## Exact-source evidence

Source tree: `a1f4f8c4ed24bcf025144a283a8bdff96e9c56a8`.
[Run 37261149762](https://github.com/SUaDtL/codex-accounts/actions/runs/37261149762)
completed successfully on the stated head. Inspected logs bind its tested merge
`d8bfbbb896f8946aa262ae476739628ac3bb10df` to the same tree and show all six jobs
passing. Source checks ran all 172 Python tests. Portable/isolated lanes passed
242 ordinary Rust cases per profile; Windows passed 280 workspace cases per
profile plus 27 separately named native behaviors per profile. All eight workspace
doctests passed. The 54 native records are distinct and explicitly state that
Desktop qualification is not established. Three two-user helpers remain NOT RUN.

The 2026-10-10 local source refresh passed dependency review (58 packages),
specification/projection/provenance, the workflow contract, macOS source consistency,
formatting and workspace/all-target Clippy with warnings denied. The Python suite
ran 172 tests: 171 passed and one POSIX-only FIFO test skipped on Windows. Runtime
debug and release each passed 116 ordinary cases and five compile-fail doctests.
These are scoped local checks, not full local workspace/native execution.
Later documentation edits have local validation only; the cited hosted run belongs
to the stated source head, not an unpublished changed tree.

## Local review and test-proof follow-up

The preceding bounded security and test review found no critical/high finding and
two medium test-proof gaps. The authorized follow-up addresses both in
`driver_tests.rs`:

1. `released_input_credit_wakes_writer_and_only_full_delivery_marks_sent` fills
   the input queue before any consumer runs, proves backpressure, drains old wakes
   and directly observes the slot-release wake. It verifies partial writes remain
   unsent, completes within a fixed test deadline and compares all delivered bytes.
2. Two added cases cancel after a partial initialize/login-start write and expire
   a blocked partial login-cancel write. They verify only the existing prefix is
   delivered, no final newline or sent event occurs, all endpoints close, failure
   stays terminal, and cancellation timeout does not replace the primary reason.
   An unsent partial login-start has no authoritative ID, so protocol cancellation
   is unavailable; the test does not manufacture one.

These were test-proof gaps, not reproduced implementation defects. Production
runtime source remains unchanged. The driver now has 30 unit cases; local runtime
debug/release each pass 118 ordinary cases and five compile-fail doctests. Five
isolated fault fixtures fail for their declared assertions: missing slot wake,
same-length corruption with framing preserved, ignored cancellation, replaced
primary reason, and input left open after failure. An initial corruption fixture
also changed the newline and was excluded from exact-byte proof before correction.
The ordinary unmodified fixture passes all 30 driver cases.

The sanctioned codeArbiter scaffold is present but remains uninitialized.
[CodeArbiter #950](https://github.com/arbiterForge/codeArbiter/issues/950) records
missing host-enforced context-scout containment. The owner explicitly selected a
logged `pr15-context-preflight` override for this test/documentation follow-up,
commit and PR update. No context documents/provenance or initialization receipts
were fabricated. Numeric maturity/coverage remains unassessed. Verification uses
the existing repository contract and pinned CI, plus the installed read-only secret
scanner; the override waives no application qualification or security gate.

The read-only discovery binary built locally and inventoried one registered
package-name candidate, reporting `E_COMPAT_UNKNOWN`, `qualified: false` and
`credential_mutation_enabled: false`. No detailed candidate selection or runtime,
home/backend/policy qualification was performed. This is local inventory evidence,
not permission to use that candidate for an account operation.

## Integration still required

Exact installed-runtime schema generation/hash binding, native private pipes,
platform clock/suspend behavior, environment policy, browser-open validation,
owned-helper stop/reap and newest-generation capture/recovery must be reviewed
before production construction. Use the existing coordinator and native lifecycle
primitives; neither EOF nor cancellation acknowledgement grants credential authority.

Q0's exact Desktop contract and Q1/Q2 native protection/durability evidence remain
open. Integrated Tauri/native consent, owner visual/accessibility checks, complete
release scenarios, signing/distribution and license decisions remain later work.
See [the open asks](maintainers/open-asks.md) and [the current packet](next-pr.md).
