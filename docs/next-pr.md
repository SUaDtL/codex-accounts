# Current packet: explicit registered image observation

PRs #14, #15 and #16 are merged; current main is
`9389c33b7bc8d41fee92f14046eebd546c5c86f2`. Continue on
`codex/runtime-contract-observation`, preserving the existing implementation.
The older review-stack instructions below are historical and do not reopen those
PRs. [Windows storage compatibility](windows-storage-compatibility.md) records the
current storage behavior, verification limits, owner-authorized engine workflow
override and implemented registered-metadata boundary. The next Q0 increment adds
one explicitly nominated package-relative executable image to that read-only
observation. It must retain root/internal/image handles through registration and
identity revalidation, keep declared images separate, and preserve unknown
publisher/runtime/home/backend/policy and ancestry. No candidate execution,
configuration/auth/history read, qualified binding or credential operation is
enabled. Keep strict inspection and existing metadata flag refusals intact.
No application qualification or security gate is waived.
See [the selected observation contract](registered-image-observation.md) and the
updated [Q0 collection procedure](q0-qualification.md).

## Historical CA-05E review packet

Resume PR #15, `feat/ca-05e-synthetic-driver`, stacked on PR #14,
`feat/ca-05c-session-correlation`. The prerequisite head is
`e4dbe1242dd0f3238c323570dcf274fbb655c89b`; CA-05E's preceding head is
`caeb46c42efe4ff22926c3d7ac817a580556c59e`. Main remains owner-merged PR #13,
`85aff5e32e6619d7250bf7d4dbfcffefbc016043`. Preserve the matching branch, inspect
actual head/reviews/checks and use non-force publication. Do not open a duplicate.

## Implemented increment

The prerequisite CA-05D extends the existing sealed sequencer with bounded login-ID matching, early
completion buffering and all completion/cancel orderings. The start reply uses the
existing request deadline; interactive login has one 600-second deadline. Cancel
has a separate fixed five-second protocol-cleanup bound and distinct acknowledgement,
error and timeout evidence. Neither a success report nor cancel acknowledgement
means helper exit, credential capture or committed onboarding. Constructors stay
test-only; no actual login, official schema, URL or credential IO is introduced.

Read live AGENTS.md, SPEC-PROTOCOL, F-017/F-019, S-011/S-012, T-24/T-25/T-26/T-31,
`ca-05d-login-sequencing.md`, `ca-05e-transport-driver.md`, `runtime/src/session.rs`,
`session_login.rs`, `driver.rs`, `driver_io.rs` and `driver_tests.rs`.
Reuse CA-05C allocation/ownership and the existing coordinator. No dependency,
native, shared storage, workflow or qualification-catalog changes belong here.

## Finish this review

CA-05E is implemented as a private synthetic pump. This follow-up strengthens
full-queue credit/wakeup and exact-byte assertions, adds partial initialize/login
cancellation and blocked login-cancel timeout coverage, and preserves production
source. Five isolated fault fixtures fail for the intended assertions. The local
runtime passes 118 ordinary cases and five compile-fail doctests in each profile;
final combined-head hosted evidence is still required before merge.

Brownfield scaffolding is present but uninitialized. CodeArbiter
[issue #950](https://github.com/arbiterForge/codeArbiter/issues/950) tracks the missing
contained scout profile on this host. The owner explicitly selected a logged
missing-context preflight override for this PR's test/documentation follow-up;
numeric maturity/coverage remains unassessed. No application qualification or
security gate is waived, and no initialization receipt is fabricated.

Verify pinned formatting, runtime/workspace debug and release, doctests, optimized
build, Clippy, source/Python/dependency/projection checks and all inherited hosted
jobs, including 54 exact-name Windows native executions and isolated Linux. Review
cancel races, ID bounds, original-error retention, no deadline resets and no fake
helper/credential receipts. Only the final head/tested tree and completed logs can
establish current execution. All owner/native qualification omissions remain open.

## Next integration gate

Retain CA-05E's independently bounded input/output and discarded stderr, real
scheduling/cancellation behavior, private reader ownership and refusal/EOF tests
using owned synthetic endpoints.
Keep production construction disabled; do not fabricate an official schema or use
real auth. Exact version decoding, native helper ownership, stop/reap/capture and
onboarding recovery integration require their own reviewed dependencies and Q0/Q2
evidence. The native evidence collection steps remain in `maintainers/open-asks.md`.
Merge order remains PR #14 before PR #15, with base reconciliation and fresh
exact-head checks. The owner's conditional merge instruction does not waive that
dependency or any gate. No automatic phase advance, release, permission change
or live account action.
