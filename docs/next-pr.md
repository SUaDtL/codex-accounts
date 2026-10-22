# Current packet: finish the Windows application

## Active delivery: PR #25

[PR #25](https://github.com/SUaDtL/codex-accounts/pull/25) contains the integrated Windows
application, documentation and CI delivery. Current hosted results belong to its current
head, not the historical records below. The first run exposed duplicate Trivy SARIF
categories and Windows-only helpers compiled into portable builds; both were corrected.
Hosted Trivy/OpenGrep and UI jobs passed on the intervening run. The final combined run
must pass before merge. No merge, signing or release is authorized by this record.

2026-10-21: the owner requested a functioning application with focused checks and
explicitly rejected exact-build approval. Normal Desktop updates must work after
runtime, storage, policy and recovery checks. The canonical specification records
that direction. The owner personally completed the A -> B -> A trial.

Working checkout: `.worktrees/native-signer182`. The optimized desktop application
builds with native capture, isolated login, health/quota, switch and recovery wired.
Current helper startup/configuration/exit passed in an empty synthetic home.
The owner confirmed successful Add B, A -> B -> A and an additional return to B.
The current candidate is dist/windows-x64-refined, implementing the approved
UI feedback plus read-only current/saved-account usage polling, expiry and resets.
See spec-amendments/usage-observation-2026-10-21.md for the scoped polling change.
The prior build's real account round trip remains confirmed. The refreshed candidate
has passed build, focused native and synthetic browser checks; the owner accepted the UI.
The active packet now adds documentation, parallel CI, security scans and binary delivery
without a version bump. See ci.md for artifact retention and release boundaries.
Release qualification, signing/publication and macOS remain separate. Preserve the
vault and all dirty worker changes. No signing/publication has occurred.

## Historical crash packet

Main run 38815869659 at `9b3cc2dffc2be77f47d868918b9d9a2c5b42acee` failed its
debug discovery test process with STATUS_ACCESS_VIOLATION. Release complement,
supervisor proof and all 108 named native executions passed; the aggregate is red.
Continue `codex/fix-discovery-factory-lifetime`. Five focused cases passed alone,
but the controlled serial complement crashed on its second hashing case after
apartment teardown. Preserve that failing evidence and test fresh-process
teardown/reinitialization before and after the fix. The original hosted crash
stack remains unobserved; a source lifetime hazard is not that missing stack.

Use registered, uncached WinRT activation with interfaces scoped before apartment
shutdown, preserving SHA-256, errors, bounds and all qualification refusals. Do
not suppress teardown, leak an apartment, serialize away the full-suite failure,
skip tests, retry CI until green or introduce a new cryptographic primitive.
Require focused independent security review and final combined real gates. The
local repair and current-call-site review now pass; this does not qualify Desktop.

## Historical static runtime-selection investigation

PRs #24 and #23 are merged. Main is
`9b3cc2dffc2be77f47d868918b9d9a2c5b42acee`; their six-job PR runs passed,
and #24's separate main run passed. #23's post-merge failure is the current
repair above; refresh live refs. Preserve `codex/desktop-runtime-binding-evidence`
for later continuation. Preserve the
original unknown PR #22 failure cause in [validation](validation.md).

The new exact-build ancestor READ_CONTROL-only observation also returned access
denied. Do not retry through another security API, elevate, alter ACLs or infer
all-principal protection from another observation. One public-package metadata
attempt refused without retaining its specific cause; do not label it network
failure or silently select a newer build.

Inspect only separately reviewed, bounded static Desktop bootstrap resources to
identify runtime selection/copy/cache rules and home/policy routing. Treat source
as untrusted data and execute none of it. Retain exact source/hash/registration
bindings and unknown ancestry/distribution correspondence; no configuration,
auth, history or process-memory reads are authorized by this packet. Implement
only rules established by actual evidence, keeping production authority disabled.

## Historical CA-02B packet

PRs #14 through #22 are merged. Main is
`6ffc3d4568364caae1dc5dc3a2d6402ef5cfee3b`; PR #22 passed all six jobs in
run 38806743149, with all 108 native executions verified. Its separate post-merge
run must also be inspected. Continue `codex/cached-primary-signer`, preserving its
reviewed source and refreshing live PR/head/checks before publication or merge.

[CA-02B](ca-02b-cached-primary-signer.md) adds bounded primary leaf observation
from live successful cached WinTrust state, exactly-once cleanup and distinct
unknown/failure states. Its additive feature review preserves all eight older
records and the unchanged 407-package lock. One exact-build read-only collection
succeeded; all installation authority remains disabled. Full combined-head hosted
gates are still required, including every existing native case in both profiles.

The next genuine Q0 gap is independently reviewed official distribution/signer/
runtime binding and protected installation ancestry. Earlier read-only diagnostics
refused registered ancestor index two with access denied, even for attribute-only
and zero-desired-access directory opens. Preserve that evidence; do not change ACLs,
elevate or infer all-principal protection from a current-token AccessCheck.
Review a bounded diagnostic/evidence route before adding a large ACL parser.
Effective Desktop home/backend/policy, resource and lifecycle semantics remain
unresolved. No repeated version/schema command or synthetic codec closes them.
The standing engine workflow override and conditional green merge authority
continue; security and qualification gates remain intact.

## Historical CA-06B packet

PRs #14 through #21 are merged. Main is
`e11471f6840502e90b33f5686dccd5fd2d0f452f`; PR #21 and its post-merge run passed
all six jobs. Continue `codex/windows-static-preview`, inspecting its live PR and
checks before publication or merge. The owner authorized direct development and
conditional green merges under the standing engine workflow override. Application
security, qualification and hosted CI remain mandatory.

This packet adds a Windows-only Tauri developer shell for the three existing
disabled synthetic pages. Read [its scope and checks](ca-06b-static-preview.md) and
the additive [dependency review](ca-06b-dependencies.json). No application commands,
credential/native adapter or installer is enabled. Full Q0 through Q5 remain open.
After its exact-head gates pass, the next bounded chunk is cached primary signer
observation in the existing read-only discovery reader: preserve held-image
identity, cache-only trust, structured unknowns and all qualification refusals.
That chunk needs its own additive dependency feature review and final combined CI.
It must not execute the nominated image, infer publisher binding from a leaf hash,
or read real auth/configuration/history.

## Historical CA-05H packet

PRs #14 through #20 are merged; current main is
`67c6f157993ed786025fc6b57305a3eb433fa7f5`. Continue on
`codex/private-rate-limit-codec-integration`, preserving the existing implementation.
The older review-stack instructions below are historical and do not reopen those
PRs. [Windows storage compatibility](windows-storage-compatibility.md) records the
current storage behavior, verification limits, owner-authorized engine workflow
override and implemented registered-metadata boundary. PR #17 added a separate
explicitly nominated image observation; PR #18 contains native test families and
preserves failure diagnostics without changing application behavior or deadlines.
The current [partial Q0 record](runtime-provenance-observation.md) binds the selected
package's image bytes to the official download and records one contained original
`--version` result. It does not close publisher protection, effective home/policy,
resource/lifecycle or qualification obligations.

That separate collector has now passed review and one original-image invocation:
317 JSON files / 3,590,402 bytes, exit 0 and cleanup verified. The public manifest
pins all names/sizes/hashes. CA-05F published fourteen exact generated files; CA-05G
added five files from that same admitted collection. CA-05H selects three more
rate-limit files: twenty-two selected files / 457,451 bytes, with a new selection
pin and unchanged complete inventory. No producer command is repeated. The reader still
does not execute candidates. CA-05F now implements the private decoder over the
existing driver for initialize/initialized and account/read with refreshToken:false,
using actual required/null shapes and original reader/session/request ownership.
Bounded strict JSON, written receipts and absolute deadlines remain intact.

Merged CA-05G extends that private codec for fixed managed ChatGPT start/cancel requests
and matching login-completed/account-updated notifications. Use the existing
specialized login sequencer, exact decoded IDs and bounded early-completion queue.
Cancellation acknowledgements retain their exact status and original failure;
they never prove helper exit. During cancellation, a schema-valid account-change
signal may consume existing shared notification credit without changing phase,
primary error or the fixed deadline. Expiry/quota and late-event tests are required.
Late start replies during cancellation must discard the URL. Notification
emittedAtMs is informational, never clock authority. CA-05H adds fixed
account/rateLimits/read encoding with excludeResetCreditDetails:true and no
supportsLunaReserve field, plus typed read/update observations. Preserve every
missing/null/present state, full signed schema numeric widths and independent
single/map values. Bound bucket maps and passive credit rows to 64 each. Skip only
the schema-untyped upsell field after strict whole-frame validation; never retain
or interpret its payload. Reuse the shared 32 notification credits and cancellation
deadline. No cache merging, refetch, polling, reset-credit operation or renderer
command belongs in this slice. Actual read/startup side effects remain an F-019
qualification obligation; a method name or omitted capability proves no purity.
Production construction stays disabled. No actual server,
login, real configuration/auth/history read,
qualified binding or credential operation is enabled by this packet. Keep the
shipped discovery reader's no-execution boundary and strict flag refusals intact.
No application qualification or security gate is waived.
See the [implemented CA-05G boundary and verification](ca-05g-managed-login-codec.md).
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
