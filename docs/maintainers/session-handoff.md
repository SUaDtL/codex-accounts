# Session handoff: functional Windows build

## Active delivery: PR #25

[PR #25](https://github.com/SUaDtL/codex-accounts/pull/25) contains the integrated Windows
application, documentation and CI delivery. Current hosted results belong to its current
head, not the historical records below. The first run exposed duplicate Trivy SARIF
categories and Windows-only helpers compiled into portable builds; both were corrected.
Hosted Trivy/OpenGrep and UI jobs passed on the intervening run. The final combined run
must pass before merge. No merge, signing or release is authorized by this record.

## 2026-10-22: local CI verification and hook repair

Local results: 266 Python tests passed (one platform-specific skip), including 44 CI
checks and six packaging/retention checks. Both synthetic Chromium scripts, Rust fmt,
workspace/all-target strict Clippy, Trivy secret/CVE/package-SBOM checks and eight
OpenGrep rule fixtures plus source scan passed. Independent application auth/crypto,
CI security and delivery coverage reviews reported no remaining critical/high findings
within their inspected scopes. Coverage percentages are unmeasured.

The first application commit was refused because the Git registry referenced removed
codeArbiter 0.17.4. The owner authorized in-place repair. The installed 0.20.6 registration
routine refreshed the managed entry; ca-doctor then reported zero failures and its actual
Git pre-push probe passed. Commit 1aeb635 subsequently succeeded with enforcement enabled.
No hook was disabled or session/approval receipt fabricated. Host MCP write-gate coverage
is separate from this verified Git backstop. PR and hosted checks follow below when known.

## 2026-10-22: documentation and CI delivery

The owner accepted the refined UI and requested repository documentation, fast parallel
CI, Trivy, OpenGrep and downloadable binaries. Version remains 0.1.0, pre-1.0.
The real account round trip was owner-confirmed; no new authentication is needed for CI.
Current commands and delivery policy are in docs/ci.md and CONTRIBUTING.md.
Local scanner fixtures/source scans and synthetic UI checks pass. Hosted results remain
pending until this source is committed and its PR runs. No release, signing or merge
has been performed. Earlier evidence below is retained with its original scope.

## 2026-10-22: layout, diagnostics, combined Add and banked resets

Build: `dist/windows-x64-refined/codex-accounts-desktop.exe`.
SHA-256: `0beab4dd9572e1606f0a48cb1dc335cd7b1754d8e73b643a53d96bbfce6997f6`.

The owner reported clipping during Open vault. A delayed synthetic IPC response
reproduced document height 387 in a 360px viewport, with the status ending at
367px. Flex sizing now allocates room for status text instead of appending it
below a fixed viewport-sized panel. The same regression passes at 940x360,
940x328 and 420x320, including a wrapped failure. Long lists remain scrollable.

Diagnostics shows installation version, conditional failure details, Refresh
and Export report. Duplicate inventory prose/list and duplicate vault state are
removed. Export leaves Settings open. Expanded diagnostics fits the default
window. The + form offers Save current account and Sign in to another account,
sharing the label input; save-current is removed from Settings. Form actions
remain visible at minimum window size, and keyboard focus outlines are preserved.

Available banked reset counts are parsed from the existing usage response's
rate_limit_reset_credits.available_count, projected as optional u32 metadata,
and shown only when positive with an available usage observation. Missing,
invalid and expired data never invent a count. There are no extra requests or
reset-consumption commands. See the usage amendment for the official source.

Verification: two focused usage parser tests, six desktop tests, strict scoped
all-target Clippy, formatting, specification integrity, optimized offline build,
and synthetic Chromium checks passed. The browser check exercises both Add
routes, diagnostics refresh/error/export, banked-reset visibility, recovery and
compact/narrow screens; screenshots were visually inspected. Regression script:
.review-evidence/layout-fix-check.cjs. Expanded UI script:
.review-evidence/ui-polish-check.cjs. Packaged payload and all 145 source input
hashes match. Actual reset-count availability on the owner's accounts and native
UI acceptance of this build remain for owner use. No real account operations,
publication or signing were performed during this slice.

## 2026-10-22: compact visual cleanup candidate

Build: `dist/windows-x64-ui-polish/codex-accounts-desktop.exe`.
SHA-256: `edc5681089527dde9d5d729b4cf4f71694537a1871e44939669d4905c5136ccf`.

The owner-provided screenshots establish that the preceding candidate displayed
weekly usage for both saved accounts. They do not establish every refresh/expiry
scenario. No real labels or usage values are copied here.

This slice removes the duplicate brand mark, repetitive vault/security copy,
Vault closed/unlocked footer status, redundant usage/account-list settings views,
and switch-history UI. Header refresh reloads accounts and schedules usage;
close and disclosure controls use SVG icons. Account name is empty with explicit
Examples helper text. The default window is 940x360 with scrolling for longer
lists. Only reported quota windows render, so an absent secondary window does
not produce a placeholder bar. The closed panel fills the remaining window with
balanced margins. Settings keeps vault/account actions and collapsed diagnostics.

Removed unused prune-history/current-health IPC commands and duplicate display
code. The existing verified-later-startup pruning path now runs on a clean existing
vault opening; pending operations and conflict evidence remain. A no-op pruning
pass does not create another registry generation. There is no unbounded audit log.

Checks: six desktop tests, two existing pruning tests (including native reopen),
existing durable-holds retention test, strict scoped all-target Clippy, optimized
offline build, 13 projection tests and specification integrity passed. Synthetic
Chromium checked compact two-account layout, a weekly-only response, missing-window
omission, direct vault entry, SVG controls, empty name/examples, header refresh,
completed-history absence, recovery actions and narrow layout. Screenshots are in
.review-evidence/ui-polish; its check is .review-evidence/ui-polish-check.cjs.
Payload and all 145 source input hashes match the package. This is a
local candidate, not owner acceptance or a published release. No actual account
operations were performed during development. Previous checks remain historical.

## 2026-10-22: UI feedback and read-only usage candidate

Build: `dist/windows-x64-ui-feedback/codex-accounts-desktop.exe`.
SHA-256: `68fb530dee28f09e156add08eb3cb3d11bcc14817f6f3cfa032b5a01f3d0f85f`. Close the previous Accounts window before opening it.

The approved feedback slice is implemented: unified vault entry/settings, direct
Open/Create vault CTA, jade plus matching the cog, small accessible status LEDs,
no em dashes in product copy, and no manual account-check acknowledgment UI/API.
The old journal phase remains compatible and is presented as Account switched.
An advisory metadata-only vault presence query never authorizes creation or
replaces native open/create checks. Unknown presence never means absent.

Read-only usage polling now runs sequentially once per minute with an unlocked,
visible vault. It uses the official fixed usage GET with existing access tokens,
no helper startup, token refresh, auth writes or active-association mutation.
Current credentials are read with shared handles released before HTTP. A matching
live identity supplies fresh tokens; otherwise the requested saved profile supplies
its token. Identity mismatches never attribute another account's usage. HTTP401
clears usage; other errors stay unavailable. Active-account polling retries after
expiry so Codex's own renewal can recover. Expired inactive accounts require a
manual retry or sign-in. Reset windows clear and trigger one fresh check, never
an assumed full quota. Exact reset/freshness timestamps stay in tooltips.

The user-authorized exception to earlier no-inactive-polling defaults is recorded
in spec-amendments/usage-observation-2026-10-21.md. The only native feature addition
is Win32_Networking_WinHttp on existing windows-sys 0.61.2; cached source/license
review passed, package graph and lock remain unchanged. Redirects, cookies and
automatic authentication are disabled; native waits, response headers and body
are bounded. The official endpoint is implementation-dependent, not a public API
guarantee. No live credential/network request was run during development.

Checks passed: six desktop tests; three existing auth parser tests; three focused
usage checks (identity/schema parsing, header injection/bounds, and a synthetic
native reader while an existing writer stays open); scoped strict all-target
Clippy; optimized offline build; 23 projection/dependency Python tests; spec and
407-package dependency checks. Synthetic Chromium checks cover vault entry,
settings/menus/recovery, narrow/zoom layouts, sequential polling, expiry masking,
reset invalidation, backoff, visibility, late-result discard and action admission.
The focused race check is .review-evidence/ui-feedback-preview/check-usage-race.cjs.
Independent source review findings were repaired and re-reviewed with no remaining
blocker in that scope. Package payload and 145 source inputs match the build.

This is a local candidate for owner use, not a real-account usage observation or
native visual acceptance. Prior account round-trip evidence belongs to its older
recorded build. No commit, push, signing or publication was performed.

## 2026-10-21: compact UI, generated icon and focused efficiency pass

The owner rejected the prior UI and approved the imagegen compact dark account-list
concept. This candidate implements active-first rows, observed quota bars, account
menus, Settings, visible recovery actions and a fixed Open Codex command. Routine
test instructions and completed switch history no longer dominate the main screen.
A generated jade/silver account-exchange icon replaces the CA placeholder in the
header and Windows icon resources. Source and prompt: crates/desktop-app/icons/README.md.

Current build: `dist/windows-x64-ui-refresh/codex-accounts-desktop.exe`.
SHA-256: `d3abf08335b323e6b6edd2d3249971a5a10a31d5d3527ff845b4761a908e4732`.
Close the older Accounts window before opening this build; retain the existing vault.
This candidate is ready for owner review. The real-account round trip below belongs
to the previous build; agents did not operate real accounts or launch this native UI.

Efficiency changes: resolve sandbox identities only for ACEs needing an exception;
omit discarded certificate/report work in operational discovery while keeping full
image hashes and retained-handle checks; query the exact official package family;
remove an unused stored path; share identical path normalization. No dependencies
or Windows permissions changed. Synthetic debug samples: 2,000 private ACL checks
302ms -> 7ms; existing 8,192-file cleanup 7.58s -> 4.87s; retained-image discovery
0.71s -> 0.45s. These are single samples with uncontrolled caches, not full-switch
latency measurements. The narrower package query had no measured additional gain.

Verification: optimized offline build; strict all-target Clippy for desktop,
vault-storage and discovery; six desktop tests; focused existing permission,
cleanup and discovery checks. Headless Chromium exercised the actual bundled assets
with synthetic IPC: menus, dialogs, settings, recovery, typed Open Codex dispatch,
420px layout and a 200%-equivalent viewport. No JavaScript errors or horizontal
overflow. These checks are not native screen-reader or real-account UI acceptance.
Checks and screenshots are retained in .review-evidence/ui-refresh. Package payload
and all 144 source input hashes were checked after copying. No commit or publication.

## 2026-10-21: owner confirms real account round trip

The owner reports successful browser sign-in and saving B, switching back to A,
logging in and confirming A in Desktop, then switching back to B to continue this
conversation. The supplied screenshot shows both accounts accepted by the official
runtime, Desktop opened, and user confirmations recorded. This completes the
owner-run A -> B -> A trial and an additional return to B on the delivered Windows
build. The preceding screenshot also shows both saved accounts and B highlighted
as last known active. No real account labels or credentials are copied here.

The functioning Windows application requested for this host is delivered. This
is owner-reported real use with screenshot support, not an automated identity
attestation or completion of every release scenario. Signing/publication, broader
release qualification and the separately selected macOS work remain open.

Build: `dist/windows-x64-cleanup-fix/codex-accounts-desktop.exe`.
SHA-256: `a22b264a7c8391ba452991a86f51fb35bd3182623965450c81ac2e17e8d5a5f1`.
No executable changes or additional account operations were needed to record this
result. Earlier entries below retain the evidence available at their checkpoints.

## 2026-10-21: completed-login cleanup and active-account display

The owner completed browser login but B was not listed. Cleanup runs before the
fresh capture is published, so that failed attempt did not save B. Three concrete
barriers were repaired: the 4,096-entry cap was smaller than the runtime plugin
inventory; the ordinary 240-character path limit rejected plugin assets; and the
normal AppData parent has the official package capability's write grant.

The owner explicitly approved verified Codex-managed parent permissions. Windows'
package SID derivation matched that grant to the fixed official package family.
Only that exact public capability is accepted on ancestors; owners, arbitrary
capabilities and private vault/credential ACL rules remain unchanged. No existing
Windows permissions were modified. See the dated permission amendment.

Cleanup permits 65,536 entries and extended paths only in the registered scratch
tree. It still pins and verifies every object before deleting anything and rejects
reparse points, hardlinks, foreign readers/writers and live writers. Three focused
regressions reproduced the prior refusals and pass after repair. Eight cleanup,
one capability, two current-association and five desktop tests pass; affected
all-target Clippy, JavaScript syntax and optimized offline build pass. Bounded
independent source review found no actionable issue. The current official helper
also passed isolated login start/cancel, exit and scratch cleanup without login.

A temporary metadata-only diagnostic accepted all 7,874 objects in the actual
interrupted scratch tree after the fixes. It read no credential content, performed
no deletion, and was removed before building. Actual cleanup and another completed
browser sign-in remain owner actions; do not claim the real A -> B -> A succeeded.

The UI now shows the last known active account and marks its saved profile. It
uses the encrypted registry association, clears ambiguous/recovery states, and
explains checking again after external sign-in. Add preserves A; Switch selects B.
It does not change the browser's general ChatGPT sign-in.

Current candidate: `dist/windows-x64-cleanup-fix/codex-accounts-desktop.exe`.
SHA-256: `a22b264a7c8391ba452991a86f51fb35bd3182623965450c81ac2e17e8d5a5f1`. Close older Accounts and Codex
clients, open this build's existing vault to clean the interrupted login, then
retry Add B and explicitly Switch. Earlier candidate entries below are historical.

## 2026-10-21: Add account false policy refusal corrected

The owner reports Add another account failed before opening a browser. Their
screenshot shows the main account saved; they report returning to it afterward.
This is not a completed second-account or A -> B -> A trial.

The installed runtime supplies the official ChatGPT backend URL even in an empty
home. The app incorrectly treated any present URL as a custom override. The
codec now classifies only the exact official production endpoint (with or without
its final slash), discards its text, and retains all other policy restrictions.
Official source: https://github.com/openai/codex/blob/main/codex-rs/config/defaults.toml

The existing native smoke reproduced Err(Policy) before repair and passes after
repair. It now exercises compatibility, actual isolated login start/cancel, owned
helper exit and absence of auth.json, without opening a browser or reading an
existing account. Endpoint negative cases and both existing config codec checks
pass. Affected all-target Clippy, optimized offline build and bounded independent
source review pass. User authentication and the real A -> B -> A remain pending.

Current candidate: `dist/windows-x64-login-fix/codex-accounts-desktop.exe`.
Close the older Accounts window before launching it. SHA-256:
`6230fab85869bb54769c00fd403bb39e2160a7f6d7f69154395784274367a068`. Prior entries below are historical.

## 2026-10-21: existing Codex sandbox permission compatibility

The owner explicitly approved supporting the normal local CodexSandboxUsers
read-only active-auth grant. The adapter resolves the fixed group qualified by
the OS computer name, verifies local alias/domain/SAM identity, and accepts only
read/execute rights. Unknown groups and sandbox write/control grants refuse.
This exception does not apply to vault/staging creation, owners or ancestors.
No existing Windows permissions are changed.

The native fixture reproduced UnsafePath before implementation, then passed with
the local group read grant while rejecting write access and strict-vault access.
The existing arbitrary-reader refusal and focused right-mask checks also pass.
A temporary read-only metadata diagnostic passed the actual ancestor and auth
file checks, without reading credential data or changing descriptors. Its source
was removed. Affected Clippy and optimized offline build pass. Account capture
and the real A -> B -> A trial remain unverified; the owner must retry.

Current candidate: `dist/windows-x64-sandbox-fix/codex-accounts-desktop.exe`.
Close the previous Accounts window before launching it. SHA-256:
`76476ceaa6339baf96a7cc85e3bccb91ce7a1a8aab88fc25e1ca7ef1cdb6a5db`.


## 2026-10-21: reported capture refusal corrected

The owner reported E_COMPAT_UNKNOWN with no saved accounts. Fixed-name metadata
showed both Desktop global-state files at 2,456,274 bytes. The app incorrectly
used its 1 MiB credential limit for these unrelated UI state files. Native read
and strict parsing now share a separate 16 MiB state limit; credential and TOML
limits stay 1 MiB, with duplicate/depth/UTF-8 checks unchanged.

The focused 2.5 MB regression failed before repair and passes after repair. Three
selection tests, 23 existing vault tests, affected Clippy and release build pass.
A temporary diagnostic using the production parser accepted both real selection
files, displaying only success booleans. No auth file or account operation was
accessed. The diagnostic source was removed; independent source review found no
actionable issue. Actual account capture/switch remains to be retried by the owner.

Corrected candidate: `dist/windows-x64-state-fix/codex-accounts-desktop.exe`.
Close the older running candidate before using this one. SHA-256:
`a59b281f1220a7e9d706767fe1b16fef6d18203c1cdc8486281b29dc144962ec`. Earlier payloads and evidence below are historical.


2026-10-21: the owner request is integrated in `.worktrees/native-signer182` on
`codex/fix-discovery-factory-lifetime`. The optimized standalone Windows app builds.
Capture, isolated managed Add, rename/remove, health/quota, switch and recovery
are connected. The current official helper passed handshake/configuration/exit
in an empty synthetic home. No real account credentials have been accessed.

Switch cancellation, native account confirmations, the encrypted interrupted-login
record and current-account reauthentication are integrated. Fresh credentials and
forward recovery publish atomically; recovery rechecks policy and preserves any
recorded helper refresh. The final 32 coordinator checks pass. The ownerâ€™s running
first build at `target/release/codex-accounts-desktop.exe` was kept intact. The
updated portable candidate is built under `target/candidate/` and delivered in
`dist/windows-x64/`. Close the older Accounts window before using it. No actual
account trial result has been received; do not claim real A â†’ B â†’ A success.

The updated candidate also includes native sanitized diagnostics export and
backend-validated user acknowledgement for the current open-vault session. Two
focused storage checks and five desktop checks pass; workspace Clippy and the
optimized build pass. The native Save dialog has not been manually exercised. After launching the
updated candidate, UI automation failed with "foreground window did not report
a process id" on initial inspection and one fresh-selection retry. The candidate
process remains running/responding; do not infer successful UI or Save-dialog
behavior from its process state.

Final candidate SHA-256:
`028225d84d5283701d088ccb17a18b9f8c4085212158000f699ab50c1999c1aa`.
The portable folder includes usage instructions, payload/source build metadata,
checksums and a CycloneDX inventory of the 407 locked external packages. The
final executable compiled offline and passed workspace Clippy; it has not been
used with real accounts. The earlier open-vault UI smoke used the preserved first
build. Source changes remain local and uncommitted.

The protected empty vault from the initial UI smoke is retained. The corrected UI opens the protected vault;
real A â†’ B â†’ A remains pending; the owner agreed to both account sign-ins.
Use isolated Add for B, not Desktop logout. Exact-build approval was explicitly
removed; the adapter resolves each current Store build and checks runtime/home,
file backend, policy, process quiescence and safe helper cleanup.

Changes are uncommitted. All 407 external package versions/checksums remain
unchanged. Focused runtime/native/UI checks and 260 Python checks pass in their
stated local scope. macOS native implementation, signing/publication and full
release scenarios remain open. Historical packet claims below are not current
routing or claims that the application is still a disabled preview.

## Historical checkpoint

Current main is `9b3cc2dffc2be77f47d868918b9d9a2c5b42acee`, after ordered green
merges of #24 and #23. #24's separate main run also passed; #23's main
run 38815869659 failed its debug discovery test process with an access violation.
The controlled serial sequence reproduces a second-hash crash after apartment
teardown. Repair and verify it before further native collection.
[Validation](../validation.md) records source, tested and actual
merge identities and full Windows inventories. Continue
`codex/fix-discovery-factory-lifetime` through [the current packet](../next-pr.md).

One selected-build READ_CONTROL-only ancestor observation returned access denied,
with stable registration and complete owned cleanup. Protection remains unknown.
The single historical-package metadata attempt refused with its specific cause
unavailable; do not invent a network/package-drift diagnosis or retry a new build.
After the repair passes, resume separately reviewed static bootstrap evidence. Never execute packaged
source or use static selection rules to infer live effective home/policy.
Standing engine-workflow override and conditional green merge authority continue;
all application qualification, security and real CI gates remain intact.

## Historical CA-02B handoff

PR #24's reviewed supervisor-fixture and diagnostic repair is joined into
this branch. Its merge remains a prerequisite: require #24 green and merged,
then verify this combined source/tree and completed hosted gates before
merging #23. The original PR #22 post-merge failure remains recorded in
[validation](../validation.md); no deadline, case or qualification gate
is waived.

Current checkpoint: PR #22 merged source `5f5cdf724a5349bbc62bac93ee2ce0b702f2e116`
as `6ffc3d4568364caae1dc5dc3a2d6402ef5cfee3b`; source, tested merge
`a36528fdaf145bf091331f30ee51fead0f2b3ff6` and actual merge share tree
`b001811290f68cbbe46e98b88d1b78df83392d64`. All six PR jobs passed in
run 38806743149. Inspected Windows output proves 108 native executions,
398=344+54 compiled partitions, the bundled-navigation test in both profiles and
mandatory supervisor proof. Its post-merge run is a separate observation.

Resume `codex/cached-primary-signer` through [the current packet](../next-pr.md).
The [signer increment](../ca-02b-cached-primary-signer.md) has independent source
and additive dependency/checker review, 254 Python tests and all eleven local
combined-workspace validation stages passing. One exact selected-image observation
reported cached trust and the bounded primary leaf digest; it remains unqualified.
Final hosted gates remain required. The next Q0 work should address real binding
and ancestry evidence, preserving prior access-denied results and all remaining
home/backend/policy/resource/lifecycle unknowns. The owner-approved engine override
does not waive them, authorize ACL repair or enable credential operations.

## Historical CA-06B handoff

Current checkpoint: PRs #14 through #21 are merged; main is
`e11471f6840502e90b33f5686dccd5fd2d0f452f`. PR #21 run 38799021210 and post-merge
run 38801230587 passed all six jobs. Both inspected Windows logs prove 108 native
executions, 96 codec executions, two cancellation-join executions and complete,
disjoint 397=343+54 compiled inventories per profile. No original-image command
was repeated for CA-05H's schema selection.

Resume `codex/windows-static-preview` through [the current packet](../next-pr.md).
The Windows-only static shell retains disabled synthetic controls and no backend;
its exact additive graph and guarded asset staging are recorded in
[CA-06B](../ca-06b-static-preview.md). Final hosted gates are still required.
The next separately reviewed chunk is cached primary signer observation, preserving
all unresolved Q0 fields and the reader's no-execution boundary. Do not integrate
its two SDK features without an additive review that preserves prior records.
The owner extended the standing engine workflow override and conditional green
merge authority. This does not waive security, exact-head tests or qualification.

## Historical CA-05H handoff

As of 2026-10-15, PRs #14 through #20 are merged into main
`67c6f157993ed786025fc6b57305a3eb433fa7f5`. The active branch is
`codex/private-rate-limit-codec-integration`. Read
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

PR #20 passed all six PR jobs and merged the reviewed tree. Its post-merge CI
is a separate observation. CA-05H extends only private rate-limit wire observations;
the native helper, credential integration and production constructors remain
unavailable. Preserve the existing driver/session rather than adding another
sequencer. Final evidence belongs to CA-05H's own head and completed checks.

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

The corrected release executable is open with the empty vault. The owner was
asked to quit Codex clients normally, save A, use isolated Add for B, switch to B,
confirm in Desktop, then quit and switch back to A. Await the actual result.
