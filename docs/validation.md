# Validation and evidence boundary

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


## 2026-10-21: diagnostics and user acknowledgement

The candidate now exports a strict allowlist of fixed operation codes, random
operation IDs and timestamps through a native Save dialog. It preserves existing
files and excludes account labels, credentials, paths and raw helper output.
Acknowledgements validate the completed current generation and remain explicitly
user-confirmed session metadata. They never assert automatic Desktop identity.

The two new focused storage cases, five desktop Rust tests, existing synthetic
browser flow, workspace all-target Clippy and optimized offline build pass.
Dependency and macOS source-baseline checks pass; neither establishes native
macOS behavior. No native Save-dialog trial or real A â†’ B â†’ A has been performed.

## 2026-10-21: account controls and reauthentication completion

The Windows source now includes profile-targeted official reauthentication,
an encrypted interrupted-login record, native account confirmations and direct
switch cancellation. A current-account repair publishes fresh credentials,
non-promoting source evidence and its journal together. Cancellation after that
publication retains forward recovery; superseded same-account credentials are
never restored. An authenticated rejected fresh target becomes `NeedsLogin`
after owned-helper exit and staging cleanup, so a fresh sign-in remains possible.
Recovery repeats policy admission before resuming credential installation.

The final integrated coordinator suite passed 32 cases, including recovery-policy
admission and interrupted-helper refresh; five focused login/cleanup cases passed. The UI's
five Rust tests, synthetic browser workflows and workspace all-target Clippy
passed. The final payload is recorded in the current handoff.
These are local checks using synthetic data. The running
first build was preserved, and real account A â†’ B â†’ A is still awaiting the owner.
No publication, signing or macOS native qualification is claimed.

## 2026-10-21: current discovery repair passes locally

The prior apartment-189 red run compiled the preserved pre-fix snapshot, which
still used generated cached WinRT factories. It did not test the dirty uncached
adapter. The current adapter initially failed compilation because three raw
HRESULT values called `map_err` without converting through `ok()`.
The corrected source now uses uncached registered activation and a SHA-256 owner
that releases its interfaces before its apartment. Both scoped reader callbacks
return owned observations; they are internal helpers, not a generally enforced
lifetime API for arbitrary callbacks.

On this source, the existing synthetic discovery suite passed 32 cases in each
of debug and release. Sequential and concurrent apartment regressions also passed
individually in fresh processes in both profiles. Discovery all-target Clippy
with warnings denied, formatting and diff checks passed. Only the real installed
inventory case was excluded from these focused synthetic runs; it remains in CI.
The original failing artifacts and unknown hosted crash stack are preserved.
Independent source review found no remaining interface escape in current callers.
Full combined CI, live account switching and product qualification are not claimed.

The owner requested completion of the existing build with focused functional
validation and confirmed availability for a later interactive A -> B -> A check.
No real credentials were used for these checks.

## Signer post-merge debug crash and controlled pre-fix reproduction

Main run 38815869659 at `9b3cc2dffc2be77f47d868918b9d9a2c5b42acee` failed the
Windows debug workspace complement: discovery's test process exited
STATUS_ACCESS_VIOLATION (0xc0000005). Release complement, supervisor proof and all
108 named native executions passed. The aggregate correctly remained failed.
Prior PR green does not supersede this separate result.

A reviewed bounded diagnostic used the previously verified compiled library-test
image from the same Rust source. Five fixed focused cases passed individually.
The sixth invocation, a 20-case serial ordinary complement, crashed at
`native_hash_uses_standard_sha256` immediately after the cached-signer report's
hashing case completed. The diagnostic stopped at that first failure; none of
its sixteen predeclared parallel complements ran. Owned cleanup proved an empty
job and unchanged source/executable identity. No installed registration inventory,
application entrypoint, real configuration/auth/history or credential operation
was selected. This diagnostic excludes that inventory case and nine separately
native cases; it is not a full CI pass or a change to the actual CI test inventory.
Diagnostic SHA-256 is
`95571fdfbcf16f292767e2940224e8e28024eb59035b92ca3203875db656549e`.

The pinned windows-core 0.62.2 FactoryCache retains agile factory pointers past
individual apartment teardown. Discovery uses generated cached factories, while
the existing storage sync observer deliberately uses uncached registered activation.
This is a concrete lifetime hazard consistent with the controlled second-hash
crash. The exact original hosted failing thread/stack remains unknown. Fresh
sequential/concurrent teardown tests and a scoped activation repair are pending;
no passing repair, qualification or new native collection is claimed here.

## PRs #24 and #23 merged; selected-build ancestor denial

Supervisor repair [PR #24](https://github.com/SUaDtL/codex-accounts/pull/24)
passed all six jobs in run 38812293928 and merged as
`d5680a5c212ac68d1a0c71ea571b80128f2c1b8c`. Reviewed source `fcb0888`, tested
merge `4419a719d4e2a38d8e08a61b267ac576232380d7` and actual merge share tree
`5cb9fe18d83eab48b8453214eb004ff3296a6611`. Separate main run 38814063142
also passed all six jobs. Both Windows logs prove 108 named native executions,
398=344+54 compiled cases per profile and 12 supervisor tests/four actual fixtures.
These later passes do not identify the original PR #22 post-merge failure cause.

Signer [PR #23](https://github.com/SUaDtL/codex-accounts/pull/23) joined that repair
in reviewed source `c7245dee0563ada92e8cc5ed6ed172cf2efddaf0`. All six jobs passed
in combined run 38813448749 at tested merge
`042549652aa4139a7b4634425f9522504305018b`. Actual merge
`9b3cc2dffc2be77f47d868918b9d9a2c5b42acee` has the same
`7b4f4de95c0a730e60a551e053de18673d4488dc` tree. Inspected Windows output proves
108 named native executions, 401=347+54 compiled cases per profile, 96 codec
executions and the unchanged supervisor proof. The combined local Python suite
passed 260 tests with one existing platform skip. Its separate main run
38815869659 is still pending at this record; PR green is not post-merge green.

After #24 merged, one independently reviewed read-only observation selected
Desktop 26.1007.9446.0 x64 and checked registration before/after. Ancestor indices
zero and one attribute handles succeeded; index two READ_CONTROL-only open
returned Win32 5 (access denied). No security descriptor was queried, all owned
handles closed and the owned job emptied. Observation SHA-256 is
`888bc7a9a55a8975d750d9b5f25704592145604d0f1d58e1ef5701934a795dd8`.
Target identity/tag and protected ancestry remain unknown. No ACL, privilege,
configuration, credential or history change occurred; no installed executable ran.

One separately reviewed historical-package metadata attempt refused, child exit
one, in 1.205 seconds with an empty owned job. Its generic private reporter lost
the specific failure category, so package drift or network refusal is not proved.
No metadata was admitted and no retry/new-build fallback occurred. Resource-body
acquisition remains unestablished. Static runtime-selection evidence is the next
bounded investigation; no observation enables account operations or qualification.

## PR #22 post-merge supervision failure and bounded fixture repair

PR #22 merged as `6ffc3d4568364caae1dc5dc3a2d6402ef5cfee3b`, with the same
`b001811290f68cbbe46e98b88d1b78df83392d64` tree as reviewed source
`5f5cdf724a5349bbc62bac93ee2ce0b702f2e116`. All six PR jobs passed in run
38806743149; inspected Windows output proves 108 native executions, 398=344+54
compiled partitions per profile, bundled navigation in both profiles and the
required twelve supervisor tests/four native fixture identities.

Its separate [main run 38809063813](https://github.com/SUaDtL/codex-accounts/actions/runs/38809063813)
failed the aggregate supervisor proof in 4.955 seconds. The reporter discarded
failure IDs and tracebacks, so the original failing assertion/cause is unknown.
Named native execution did not run. The earlier green result does not erase this
failure; PR #23's signer merge remains held for this repair and fresh combined CI.

A controlled child reproduced a faulty fixture assumption: a non-inherited
handle's numeric value can denote a different valid child object. The old
`GetHandleInformation == 0` assertion failed despite restricted inheritance.
The replacement probes the actual private pipe effect. A child with a valid
unrelated same-number handle cannot write to that pipe; intentionally adding the
pipe to the real owned child's handle list makes the same assertion detect the
leak. Parent pipe usability and the unrelated owned process remain checked.

Two no-process regressions also failed before correction: native fixture setup
cleared real cleanup poison, and model setup failed to restore it. Native setup
now refuses existing poison before any child creation; model tests temporarily
isolate their synthetic flag and restore the previous real value afterward.

Failure reporting adds only allowlisted test IDs and fixed categories, bounded
to twelve rows with truncation indicated. Raw exceptions, tracebacks, assertion
payloads, paths and subtest parameters remain private. Canary/hostile-formatting
tests verify that boundary; the passing JSON summary is unchanged.

All eight focused gate tests and the actual twelve-test/four-native proof pass
locally. The full Python suite passes 256 tests with one existing POSIX-only skip
on Windows. Independent review of all three changed Python files found no material
issue. Production `windows_process.py`, the exact native inventory, operation and
cleanup deadlines, no-skip assessment and no-retry policy are unchanged. These
reproduced fixture defects do not establish the original hosted failure's cause.
Final combined-head hosted checks remain mandatory before merge.

## CA-02B: cached primary signer candidate

The [signer observation](ca-02b-cached-primary-signer.md) retains the original
read-only image/registration boundary, with bounded owned DER copying from live
cached WinTrust state and exactly-once cleanup. A genuine missing-field regression
failed before implementation. Independent source and additive dependency/checker
reviews found no actionable issue. No package is added to the 407-package lock;
all eight historical reviews remain immutable.

Fresh local Python runs 254 tests successfully with one existing platform skip.
All eleven combined-source stages pass in 493.862 seconds: checks, formatting,
debug/release complements, eight doctests, optimized all-target build and strict
workspace Clippy. Each complete/disjoint inventory is 401=347+54: 344 passed,
three existing owner-run cases ignored and 54 native cases filtered for their
mandatory separate hosted execution. All copied/original input hashes remained
stable, all owned jobs emptied, and no native inventory case/deadline was removed.

One reviewed release diagnostic then observed the selected registered Desktop
26.1007.9446.0 x64 without executing its image or reading real config/auth/history.
It exited zero with empty owned job, stable collector/source/image/registration
and a primary leaf digest matching the prior static extraction. Exact image,
collector and report digests are in the increment record. The result is cached
signature evidence, not publisher approval, protected ancestry, effective-home
proof or complete Q0 qualification. Final exact-head hosted gates remain required.

## Merged CA-06B / PR #22

Source `5f5cdf724a5349bbc62bac93ee2ce0b702f2e116`, tested merge
`a36528fdaf145bf091331f30ee51fead0f2b3ff6` and actual merge
`6ffc3d4568364caae1dc5dc3a2d6402ef5cfee3b` share tree
`b001811290f68cbbe46e98b88d1b78df83392d64`. All six jobs passed in
[PR run 38806743149](https://github.com/SUaDtL/codex-accounts/actions/runs/38806743149).
The inspected Windows log proves 108 native executions, 96 codec executions,
two cancellation-join and two bundled-navigation executions, complete/disjoint
398=344+54 compiled partitions per profile and twelve supervisor tests including
four actual Windows fixture methods. Post-merge CI is a separate observation.

## Preserved CA-06B local implementation evidence

The [static shell](ca-06b-static-preview.md) adds no application commands or
native account integration. Additive source/license/advisory review covers the
exact 407-package lock, preserving all seven predecessor records. Independent
dependency/checker and shell/security reviews found no remaining actionable issue.
Two source-checker findings (URL-bearing attributes/CSS escapes and malformed
configuration diagnostics) were fixed and their regressions passed before the
combined source snapshot.

One clean copied-source validation ran all eleven stages successfully in 503.705
seconds: dependency, schema, static asset, specification and workflow checks,
formatting, debug complement, doctests, optimized build, release complement and
strict all-target Clippy. Each profile proves a complete/disjoint 398=344+54
compiled partition: 341 passed, three existing owner-run two-user cases ignored,
and 54 native cases filtered for their separate mandatory runner. All eight
doctests pass. This is local snapshot evidence, not final-head hosted proof.
Final Python verification runs 250 tests successfully with one existing
POSIX-only skip on Windows. Eight static-preview tool regressions cover asset
drift, active content, malformed input, path links, bounded reads and fixed errors.
Existing strict JSON depth/duplicate and the new bundled-navigation regression
pass with the expanded graph. No native case or deadline was removed.

A separately built static-context executable created one responsive owned window
and a fresh owned WebView profile, accepted WM_CLOSE, exited normally and left its
owned job empty. Optional owned-client capture produced no accepted image;
rendered HTML, interaction and accessibility remain UNOBSERVED. The first launcher
attempt refused before launch because Cargo's output had two hard links; an
exclusive byte-identical private copy satisfied the unchanged single-link check.
This is a source-equivalent context smoke, not a final-workspace GUI receipt or
network/filesystem isolation claim. No original Desktop image was invoked.

Final combined-head Python/source checks and all inherited hosted jobs remain
required before merge, including every isolated native execution in both profiles.
No complete Q0-Q5 requirement or release scenario is closed.

## Merged CA-05H / PR #21

Source `69599d5ad29044979bc35f970575515aa0266ef1`, tested merge
`f5e0208a393e124d03ca38acfce8e4bab19e627a` and actual merge
`e11471f6840502e90b33f5686dccd5fd2d0f452f` share tree
`8edf5fa3a40b2997010b5c0837ea8745de31d32d`. All six jobs passed in
[PR run 38799021210](https://github.com/SUaDtL/codex-accounts/actions/runs/38799021210)
and [main run 38801230587](https://github.com/SUaDtL/codex-accounts/actions/runs/38801230587).
Both inspected Windows logs confirm 108 native executions, 96 codec executions,
two cancellation-join executions, 397=343+54 compiled partitions per profile and
twelve supervisor tests including four actual Windows fixture methods.
PR #20's separate post-merge run 38797855057 also passed all six jobs with its
own reviewed tree and the Windows counts recorded below.

## Preserved CA-05H local implementation evidence

The [rate-limit codec](ca-05h-rate-limit-codec.md) adds fifteen actual-driver
tests without runtime IO, polling, cache merge or credit operations. Fresh local
debug/release each pass 167 ordinary runtime tests and five compile-fail doctests.
Both Windows workspace complements pass, with complete/disjoint 397=343+54
compiled inventories per profile; the 54 isolated native cases remain mandatory
in each hosted profile. Python runs 232 tests with one existing POSIX-only skip
on Windows. Formatting, strict workspace/all-target Clippy, workspace doctests,
optimized all-target build and schema/spec/dependency/workflow checks pass.

Independent security, coverage and schema review found no actionable issue and
compared all three new files to original captured bytes while preserving all
prior files and the full 317-entry inventory. The first integrated runtime run
failed the old explicit manifest assertion; updating that assertion to the actual
new selection pin restored the full suite. Three isolated safeguard-removal
failures and their restored green results remain separate regression evidence.
Final hosted evidence is recorded above. No dependency,
catalog, native deadline, security or application qualification gate changes.

## Merged CA-05G / PR #20: managed-login wire codec

PR #20 merged as `67c6f157993ed786025fc6b57305a3eb433fa7f5`. Source
`ac92dac0ee1afa941cd10a289e7d458f3abae0be`, tested merge
`d504ac64f62719418fbdc0f514a0f1b7834336c2` and actual merge share tree
`407dc829e452cabc7c55878fc23eedb681c2cc4b`. All six jobs passed in
[PR run 38796177485](https://github.com/SUaDtL/codex-accounts/actions/runs/38796177485).
The inspected Windows log confirms 108 native executions, 66 codec executions,
two cancellation-join executions, complete/disjoint 382=328+54 compiled partitions
per profile and twelve supervisor tests including four actual Windows fixtures.

The [private managed-login codec](ca-05g-managed-login-codec.md) adds sixteen
actual-driver test groups and one session cancellation-notification regression.
Fresh local runtime debug/release each pass 152 ordinary tests and five
compile-fail doctests. The full Python suite runs 232 tests with one existing
POSIX-only skip on Windows. Formatting, strict workspace/all-target Clippy and
schema/spec/dependency/workflow checks pass. Independent security and
coverage/schema reviews found no actionable issue; all nineteen schema exports
were independently compared against the original admitted collection.

Two isolated safeguard-removal controls fail their intended assertions, then
are restored. Cancellation without intent has a failing-before/passing-after
category regression. Later increments require their own combined-head CI. No local
result establishes application qualification or transfers to a later source.
The inherited native cases, process-supervision proof, deadlines, no-retry rule,
dependency graph and empty compatibility catalog are unchanged.
Both local Windows workspace profiles pass their complete/disjoint 328-case
complement of the 382 compiled cases; all 54 isolated native cases still require
hosted execution in each profile. Workspace doctests and the optimized all-target
build also pass. The staged-file secret scan reports zero findings.

## Merged CA-05F / PR #19: pinned schema and private codec

PR #19 merged as `e75ef1618984f1eac79cb7d28d0ae6410353a26c`. Reviewed source
`7803f2789332427013658cf960f8478bcaff5659`, tested merge
`d6b11f463dce901018272a8518f52cbef4bb5acf` and actual merge share tree
`586d7c94ed89eec312a543849056aa6e7289e5a8`. All six jobs passed in
[PR run 38790391915](https://github.com/SUaDtL/codex-accounts/actions/runs/38790391915)
and [post-merge main run 38792671825](https://github.com/SUaDtL/codex-accounts/actions/runs/38792671825).
Both inspected Windows logs prove all 108 native executions, all 17 new codec
cases per profile, 365=311+54 complete/disjoint compiled partitions per profile
and twelve supervisor tests including four actual Windows fixture methods.

The reviewed original-image collector admitted 317 JSON files / 3,590,402 bytes
with exit 0, signaled parent, empty job, completed drains and successful profile
deletion. Fourteen exact generated files and the complete digest inventory are
pinned publicly. This is a contained local Q0 experiment, not actual server/RPC,
effective Desktop-home proof or installation qualification. See
[the provenance record](runtime-provenance-observation.md).

The [private codec](ca-05f-schema-codec.md) adds seventeen actual-synthetic-driver
tests, retaining every prior runtime case. Fresh local runtime debug/release each
pass 135 ordinary cases and five compile-fail doctests. The Python suite runs
232 tests with one existing POSIX-only skip on Windows. All seven schema-checker
tests pass, including actual owned Windows junction refusal. Dependency/spec/
workflow/schema checks, formatting, strict workspace/all-target Clippy and diff
checks pass. Independent security and coverage/schema reviews found no actionable
issue. Two isolated safeguard-removal controls fail for the intended assertions
before restoration; the notification/request diagnostic fix has a regression.

The first combined Python run exposed a missing explicit unsafe prohibition in
the new test file and a stale exact dependency-map assertion. Both were corrected
without weakening either guard; the full suite then passed. The two earlier
schema-checker findings, Python 3.11 junction fallback and CLI traceback exposure,
have reviewed fixes and passing owned-junction/fixed-output regressions.

These exact-head results belong to PR #19; CA-05G must obtain fresh checks. The 54 native case identities per
profile, supervisor proof, deadlines and no-retry behavior are unchanged. No
catalog entry, production constructor, native helper, login, credential IO or
completed Q0-Q5 acceptance follows from this candidate.

## Merged PRs #17 and #18: image observation and CI process ownership

PR #17's source `2261c3b385cd3948cb4f685a27b92487602cf73c`, tested merge
`5c53a3640cd0b63b020a3c44061fa5dd9d288f5a` and actual merge
`96767e850fa503e975505ad976c89aa2a0502d8f` share tree
`6a110686859bf17ccdeca35910280592a7f53ce7`. All six jobs passed in
[PR run 38770382714](https://github.com/SUaDtL/codex-accounts/actions/runs/38770382714),
including 108 named native executions, 54 per profile. The 348-case Windows
compiled inventory partitions into 294 complement and 54 native cases per profile.
The later main failures below remain part of the record.

PR #18 merged as `b734f0c0f02010e59c736d1abc758172cd9e171b` after source
`7ba570a0e6d392890334677c483a303c90b89b25` passed all six jobs in
[PR run 38779539609](https://github.com/SUaDtL/codex-accounts/actions/runs/38779539609).
Source, tested merge `c5e78518e2d733a16a0c93ce0481cdbb9de7b7f9` and actual merge
share tree `a52618515cbe9fbc4615af3f89598024d6e9711b`. Inspected Windows output
proves all 108 required native executions with no missing/duplicate/extra identity,
both complete/disjoint compiled partitions and the mandatory 12-test supervisor
proof including four actual Windows fixture methods. Per-case deadlines and
no-retry behavior remain unchanged. These are exact-source CI results, not an
explanation of the preceding storage failures or completed Desktop qualification.
The post-merge [main run 38782120146](https://github.com/SUaDtL/codex-accounts/actions/runs/38782120146)
also passed all six jobs at `b734f0c`; its Windows log independently contains all
108 required native passes and the same 12-test/four-native-method supervisor
proof. Later changed source must obtain its own checks.

The [partial runtime provenance record](runtime-provenance-observation.md) separately
documents the selected registered image's public-source equality and one contained
original-image version observation. Its private experiment is outside product CI;
schema, native integration and full Q0-Q5 acceptance remain open.

## Merged PR #16: Windows storage and registered metadata

PR #16 merged as `9389c33b7bc8d41fee92f14046eebd546c5c86f2`. Reviewed head
`5729693d99b93f93e8dac2c00255e36ce3e80541` passed all six jobs in
[run 38761237982](https://github.com/SUaDtL/codex-accounts/actions/runs/38761237982).
The source and tested merge `67974269ecf5188edf74244a37e328b9886e411d` share tree
`65082c2815639a22e46dd3dae8b798701b9295b4`, also preserved by the actual merge.
Local and inspected hosted Windows logs each contain 102 successful named native
executions: 51 cases per profile. The compiled 343-case inventory has a complete,
disjoint 292-case workspace complement and 51 native cases. Three existing owner
two-user helpers remain ignored / NOT RUN. Local Python ran 206 cases with one
existing POSIX-only skip. Formatting, Clippy, both-profile doctests, optimized
build, dependency review and source/workflow projections passed.

Earlier restoration timeouts remain recorded. Scheduling internally parallel
restart suites serially in the outer runner preserved every assertion, all cases,
the 180-second deadline and no-retry behavior; the final full runs passed.
Independent security, coverage and combined source reviews found zero material
issues. Real registered metadata inspection succeeds for package 26.1007.9446.0
after the regression-backed UTF-8 BOM fix. This establishes no runtime role,
effective home/backend/policy, publisher authority, credential access or Q0-Q5
qualification. See [Windows compatibility](windows-storage-compatibility.md).
These exact-source results do not transfer to the next increment or substitute
for inspection of its final head and completed CI.

## CA-05E / PR #15: synthetic driver and test-proof follow-up

Preceding head `caeb46c42efe4ff22926c3d7ac817a580556c59e`, tree
`a1f4f8c4ed24bcf025144a283a8bdff96e9c56a8`, passed all six jobs in
[run 37261149762](https://github.com/SUaDtL/codex-accounts/actions/runs/37261149762).
Its tested merge `d8bfbbb896f8946aa262ae476739628ac3bb10df` has the same tree.
Inspected logs show 172 Python passes, 242 portable Rust passes per profile,
280 Windows workspace passes plus 27 separately named native behaviors per profile,
and eight workspace doctests. Three owner-run two-user helpers remain NOT RUN.
Those results do not establish execution of the subsequent changed tree.

The authorized follow-up strengthens the full-queue wake/exact-byte oracle and
adds partial output cancellation and blocked login-cancel timeout tests. Local
debug/release each pass 118 ordinary runtime cases and five compile-fail doctests;
the Python suite passes 171 cases with one Windows POSIX-only skip. Formatting,
workspace/all-target Clippy, dependency/specification/workflow checks pass.
Five isolated fault fixtures fail for the intended assertions; production runtime,
native code, pins, workflow and compatibility catalog remain unchanged.

An additional prerequisite review gap is closed by extending the login-order test
at AwaitInitialized. An isolated premature-login fault passes the old assertion
set and fails the strengthened test; the unchanged implementation passes. Test
function counts and all production qualification boundaries remain unchanged.

CodeArbiter context is scaffolded but uninitialized, with host support tracked in
[issue #950](https://github.com/arbiterForge/codeArbiter/issues/950). The owner
authorized a logged missing-context preflight override scoped to this follow-up;
numeric maturity/coverage is unassessed. No application qualification, security or
exact-head hosted CI gate is waived. Final source/tree/run and completed logs must
be checked on PR #15 after its last publication and base reconciliation.
See `ca-05e-transport-driver.md` for the transport contract and proof limits.

## CA-05D: current PR #14 execution boundary

CA-05D continues CA-05C source `db7fcfced20c3fab1f602991fd6e56712ec016b6` on
`feat/ca-05c-session-correlation`. Thirty new deterministic Rust tests and four
Python checks extend the actual sealed sequencer. All 58 old runtime cases remain;
the combined runtime has 88 ordinary tests and four compile-fail doctests.

Login reports, cancellation acknowledgements and deadline checks are normalized
protocol evidence only. No constructor, real IO, helper, credential operation,
native API, dependency, workflow or catalog authority is added. Final source,
tested merge/tree and inspected completed checks belong to the updated PR #14.
Its earlier CA-05C run 36192783799 is historical, not proof of this changed head.
All inherited workspace/native checks and owner qualification omissions remain.
See `ca-05d-login-sequencing.md` for exact behavior and cancellation limits.

## CA-05C: preceding PR #14 increment

Base is owner-merged PR #13 at `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
The selected branch is `feat/ca-05c-session-correlation`; its final head, tested
merge/tree and inspected completed jobs are recorded on that live review. Never
inherit the historical PR #13 results below for this changed source.

Twenty-four deterministic tests exercise the actual sealed non-login sequencer;
four Python checks enforce construction, no-IO and fixed-budget boundaries. The
runtime has 58 ordinary cases and two compile-fail doctests. Model events are not
wire-schema, clock/scheduler, pipe-origin, native-exit or credential evidence.
No dependencies, native interfaces or CI execution partitions change.

All existing hosted jobs and 54 named native executions remain required. Ordinary
user/two-user, unavailable-store, exact Desktop Q0, macOS native, UI accessibility
and physical-power-loss qualification remain separate and unestablished. Consult
`ca-05c-session-sequencing.md` and `next-pr.md` for exact scope and next selection.

## Merged PR #13: historical CA-04D/CA-05B evidence

Owner merge: `85aff5e32e6619d7250bf7d4dbfcffefbc016043`. Final source
`0ab969c4b05ffc25820f2f6c538ff510bcfa34ae` passed all six jobs in run
[36186587877](https://github.com/SUaDtL/codex-accounts/actions/runs/36186587877).
Its source and test merge `4dd0e38ddf7ff5088c4f533a13e8fce374d53929` share tree
`d58404dfcf67aa3f3c9b59aae36fed708f46c5d6`. Source: 159 Python passes. Windows:
198 workspace passes plus 27 named native cases in each profile; four doctests;
three two-user helpers NOT RUN. Portable/isolated Linux and macOS: 160 Rust passes
per profile and four doctests. These records do not qualify Desktop or later heads.
The following preserves that PR's review scope and evidence limitations.

This review combines CA-04D source `345533eacd4ed4ed288fa7a4580dd4e408106ede`
with main `d64d3e31712798b1b790c70c3830c6853a67c1a2` (owner-merged PR #12).
Only the review branch is advanced; both parents are retained in its integration
commit. Five conflicting documentation files are reconciled with both code tracks
preserved. macOS source drift is resolved through the scoped CA-04D interface review,
not by declaring any native case passed.

CA-05B adds strict syntax validation, zeroizing frame owners, eleven adversarial
runtime tests and a private-constructor compile-fail doctest. All 23 inherited
runtime tests remain. See `ca-05b-json-objects.md` and `ca-05b-dependencies.json`.
Only internal dependency edges change; all 58 external package records are retained.

Final exact-head local/hosted outcomes, tested merge/tree and inspected job logs
are recorded on PR #13. A local targeted pass is not a combined-head or native
qualification. Browser/keyboard/zoom/screen-reader and macOS native checks remain
NOT RUN; three two-user helpers remain NOT RUN. Physical-power-loss and exact
Desktop qualification are not claimed. All prior evidence below is historical.

## Merged PR #12: preparation evidence (historical)

PR #12 starts from owner-merged CA-04C at `455baa2eeee614586b26d419ee961475fe1ad23b`.
See `maintainers/preparation-handoff.md` for the exact source/test surface and
`next-pr.md` for current routing. The local combined preparation passed 143 Python
source tests. Hosted final-head results belong to the PR; earlier green runs are
not inherited. First protocol-only run 36126434474 passed Ubuntu tests/Clippy and
isolated Linux but failed formatting, subsequently corrected in source.

Fourteen new runtime integration tests and sixteen presentation plus nine macOS
source checks do not close any full requirement or T-xx scenario. Browser navigation
was blocked before load: browser layout/keyboard/zoom and screen-reader validation
are NOT RUN. The macOS packet contains documentary facts and drift checks only;
all native behavior and exact Desktop qualification are NOT RUN. No credential,
helper, login, refresh, live command integration or renderer consent is introduced.
No dependency, manifest, workflow, native implementation or qualified catalog change.

The CA-04C review record and earlier records below are historical implementation
evidence; CA-04C was subsequently merged in PR #11. They do not establish this PR's
combined-head success or macOS behavior.


## CA-04D review: staging recovery and CI partitioning

Base: owner-merged CA-04C / PR #11 at
`455baa2eeee614586b26d419ee961475fe1ad23b`. Current implementation and reproduction
are in `ca-04d-staging-recovery.md`; CI equivalence/timing review is in
`ci-partition-review.md`. Final source head, tested merge/tree, completed run and
measured timings are recorded on the actual review PR after log inspection, never
inherited from the historical run below.

The candidate adds typed encrypted staging evidence and explicit staging-only repair.
Normal cleanup remains restrictive. Six portable behaviors cover provenance/codec,
guards, encrypted write faults, retries and capacity. Four new named Windows cases
add exclusive-handle/archival/refusal and five repair-process interruption points.
All earlier native and encrypted-store fault behaviors remain mandatory.

Windows workspace execution proves its compiled complement excludes exactly the 27
named native cases. The independent verifier must execute all 27 in each profile.
Do not label the workspace complement alone as complete. Three inherited two-user
helpers remain NOT RUN, excluded from pass totals. Python's FIFO fixture is explicitly
platform-specific; the Linux source lane runs the full suite. The Unicode and mocked
/proc fixtures are corrected, not disabled. No new case is ignored.

No Desktop qualification, real credentials, production effects, directory-metadata
or physical-power-loss guarantee is added. Check current exact-head results before
claiming this review is ready. The old encrypted registry formats remain readable;
staging evidence uses explicit CAREG003. Envelope/generation/key formats are unchanged.

## Merged CA-04C / PR #11: historical exact-source evidence

Source `b51e460cbd2a9d82bd24f3344d4f9d83c06c4d64` passed all six jobs in
[run 36117748297](https://github.com/SUaDtL/codex-accounts/actions/runs/36117748297).
Test merge `2e7decc441cbf516c620e7155bc0c93990a31e46` and owner merge share tree
`25e5f0ebb19f0b9b793ae22ddb7111698e680a70`. Source: 118 Python passes. Windows:
190 workspace passes per debug/release profile, three doctests, and 46 exact-name
executions of 23 behaviors. Three two-user helpers were NOT RUN. These are historical
records, not evidence for CA-04D. CA-04C's three torn-stage cases blocked automatic
recovery; CA-04D adds explicit repair without deleting that refusal proof.

## Merged CA-04B / PR #9: final-source evidence

CA-04B implements private Windows x64 owner/canonical-home locking, handle-bound
process identity and normal-quit observations, retained descendant tracking and
owned-job exit evidence. The journal, credential formats and public mutation
boundary remain unchanged. All effects tests use newly created synthetic objects
and exclusively owned children. No official Desktop or real credential is used.

Owner merged PR #9 on 2026-09-25 as
`614eb1d68559b37ea4e32014214519b4d2a62e54`. The final PR source head was
`821ebdc26ee6081b5c74ff1da79763bef2f9ea6f`, not the earlier pre-documentation head.

| Final-source evidence | Observed result |
| --- | --- |
| Final run | [36108485791](https://github.com/SUaDtL/codex-accounts/actions/runs/36108485791): source, three Rust lanes, isolated Linux and aggregate CI gate all completed successfully |
| Tested merge / tree | `65147ae9dfdb5752b3a24371d4358d9e836c2ed2` / `85b0bb90d2166a17814068c184b1c07726466f32`; owner merge has the same tree |
| Source job `107986355627` | 114 Python tests; spec/model/projection/provenance, workflow and exact 58-package review passed. All 43 requirements and 34 acceptance IDs retained; zero qualified records |
| Windows job `107986355847` | 176 workspace passes in each debug/release profile, including 87 storage/lifecycle cases; three doctests; formatting, optimized compilation and Clippy passed |
| Named Windows proof | Eleven behaviors in each profile: 22 passing execution records, not 22 distinct tests or qualification receipts |
| Ubuntu `107986355639` and macOS `107986355732` | All applicable checks passed; Windows-only verifier not run on these hosts |
| Isolated Ubuntu `107986355470` | Fresh-target compilation, 129 workspace passes per profile, three doctests and Clippy passed inside the checked privilege-dropped, direct-IP-isolated namespace |
| Aggregate `107987551146` | CI gate passed after all prerequisite jobs succeeded |
| Limits | Server 2025 x64 build 26100, not owner Windows 11 or ordinary-user qualification. Three two-user helpers ignored / NOT RUN. Workspace counts include child-test entry points |

The final source, Windows and isolated-Linux logs were inspected during CA-04B
closeout. Earlier run 36107357329 at `078dc4c4a9ebfc6a7fe937e1a85b892b30c8bdff`
is preserved on PR #9 but is not substituted for the final-source run above.
Local CA-04B checks recorded 114 Python passes, source checks, formatting, doctests
and Clippy; Linux debug/release tests also ran before final documentation closeout.
An additional final-tree local debug run hit the execution tool's 45-second limit
and was not counted as a pass. Windows native evidence was hosted, not local.

These are exact-source historical records, not inherited results for later commits.
The bootstrap/concurrency documentation increment changes no Rust, dependency,
workflow or product authority. Its own final-head checks belong to its documentation
PR. No full T-01 through T-34 scenario or platform qualification is closed here.

### Failure repair and safety review

Earlier Windows runs failed even though the isolated CA-04B cases passed. Run
36106651127 at `4ee65d02845d7a974a53bec3444ba72e869c62be` exposed fixed API-stage
and numeric HRESULT diagnostics: factory activation returned `CO_E_SERVER_STOPPING`
(`0x80080008`). The repair permits at most four activation calls with three 50 ms
pauses only for that result, retaining the initialized stack apartment. Exhaustion
and all other errors refuse; complete positive inventory/path checks still follow.
Tests cover transient success, exhaustion and immediate other-error refusal. No
service restart, failed-inventory-as-empty assumption, ACL repair or test retry is
used. The guard's stack lifetime also avoids COM cleanup in Windows TLS teardown.
See `ca-04b-native-review.md` for exact reviewed contracts and limits of inference.

The eleven native behavior cases cover canonical aliases, recursive/cross-process
contention and exit release; ownership/link/replacement refusal; stale process start
identity; normal close versus refusal; late descendants/final writes; and owned
helper timeout without terminating an unrelated child. Incomplete observations stay
explicit. Broker-created or missed-intermediate descendants and shared-home writer
association are not proven by snapshots; shared-home readiness never authorizes a
production write. OwnedFamily has only a test constructor in this increment.

### Standard-hosted assurance

Every Rust lane executes debug and optimized tests, doctests, optimized compilation
and Clippy. Exact-name Windows verification rejects missing/ignored/duplicate/failed
cases. A separate standard Ubuntu job starts with an empty build directory inside a
new network namespace, drops privileges/capabilities, verifies IPv4/IPv6 route refusal
and runs the checks after reviewed acquisition. This is direct-IP isolation, not a
hostile-build filesystem/IPC sandbox or Windows/macOS isolation. Actions use scoped
reviewed Node 24 pins; seven-day artifacts contain committed public source plus its
digest record only, never test vaults, keys or raw helper output. Source capture is
not a build SBOM, signed provenance, reproducibility proof or a release.

All 58 external package/version/checksum records, Cargo.lock, four predecessor
review documents, normative specification/provenance and empty compatibility catalog
are unchanged. Five SDK features are reviewed in `ca-04b-dependencies.json`; no new
external package is introduced. Unsafe code remains inside the private native
boundary. No global process kill, account creation, user ACL repair, permission
change, release or live account action occurs. Safe collection/cleanup commands and
remaining owner evidence are in `ca-04b-lifecycle.md`; never share real auth files.

## Preserved CI assurance evidence / PR #8

PR #8 head `cbc65e4f9ba0463fd52811364cb024c9259d48bc` passed run 35963995396:
95 Python and 153 Windows Rust passes, three doctests, optimized compilation and
all five jobs. Owner merge `27afd3381958b68154947e7f8d0092b214c226c6` passed main
run 35965081513. That increment introduced strict source/step/job assessment and
removed duplicate feature-push checks; it did not execute optimized tests or isolate
build networking. Those later additions belong to CA-04B. Its historical local
execution failure is not a current tool limitation. `ci-review.md` preserves findings
and current dispositions; main protection remains a separate owner action.

## Merged CA-04A evidence and limits

PR #7 merged head `bfe30a0eee174dfe107e5b7edc07d5c637408e28` as
`103d1a409ea19544ac4a18f33a19df925f5efc58`.
That head finished the storage wiring and tests after the intermediate `43902b9...`
commit. The initial failure was a dependency-manifest mismatch before Rust execution;
the reviewed existing-core path edge was recorded without adding external packages.
Temporary preparation was removed. No production effects adapter or live-switch
entry point was added. Merged-main run 35962095930 supersedes its stale pending-CI
text: inspected logs recorded 78 Python and 153 Windows Rust passes, including
64 storage cases.

Prior implementation-local records report Rust 1.90.0 workspace tests, formatting,
Clippy and 78 Python passes, including 44 generic storage tests. These are preserved
historical records, not current CA-04B executions. Source was restored
from the digest-checked development inputs; acquisition/preparation was not validation.

The executable fault suite retains encrypted medium and synthetic external resources
independently of coordinator memory. It covers A0-to-A1-to-B-to-A1, post-helper B1
restoration, optional absence combinations, late binding/policy/writer failures,
duplicate requests, conflict evidence, cancellation and separate primary/restoration
outcomes. Forward/restoration storage effects, request/cancel/recovery-choice commits
and external stage/replace/helper/reap/cleanup/launch failures are injected. Reopening
must preserve reachable newest generations and complete holds, not merely old bytes.

Two added Windows cases exercise actual DPAPI/storage reopen for complete and
interrupted journals; external credential/helper/launch effects are synthetic. The
journal is a `CAREG002` registry extension committed with selection and complete
holds. `CAREG001` remains readable without rewrite; first-journal conversion uses
the existing recovery protocol. Unknown codecs, broken references and untrusted
conflict evidence cannot become selectable state. Public profile writes/pruning are
blocked by unresolved journals. No fixture supplies production authority.

Actual Desktop publisher/runtime/home/backend/policy/resource/identity/lifecycle,
owner Windows 11, ordinary two-user/unavailable-store, representative sync providers,
physical power-loss/directory-metadata and independent macOS qualification remain
open. No complete T-01 through T-34 scenario is closed. `ca-04a-journal.md` and the
native procedures give safe collection, expected results, cleanup and redaction.
Never share real authentication files, account identities, keys, paths or helper output.

## Preserved earlier exact-head evidence

| Increment | Source and executed evidence |
| --- | --- |
| CA-03C / PR #6 | Head `6c8144f041ba16be1b6c33c1de6f3d1f6aa8dfb8`; PR run 35933850694 and push 35933846776 passed. Source/Windows logs: 75 Python, 129 Rust, including 40 storage cases. Three two-user helpers NOT RUN. Native tests covered protected storage, lock release, link/ownership/sharing refusal, WinRT inventory, control repair and 20 process-restart boundaries. |
| CA-03B / PR #5 | Head `9fd8e6902ce909dfcad39fbb40f48e6af81bc17e`; PR run 35897568260 and push 35897560936 passed. Inspected logs: 71 Python, 89 Windows Rust, including 23 crypto cases. Three two-user helpers NOT RUN. Scoped primitive/zeroization/DPAPI/advisory review remains in ca-03b-crypto.md and ca-03b-dependencies.json. |
| CA-03A / PR #3 | Head `595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`; PR run 35819942583 and push 35819939120 passed. Source job 107049451062: 62 Python; Windows job 107049451273: 66 Rust, including 23 vault-data and nine discovery/native cases. Formatting and Clippy passed on all three platforms. |
| CA-02 / PR #2 | Owner merged head `6ee778e9592d46132f253e2bee836b2650a44bd0` as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. Run 35808957582 passed: 48 Python and 43 Windows Rust. Later T-07 adoption and acquisition repair are not attributed to that merge. |
| Foundation / PR #1 | Merge `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`; tested head `9b9328f474322c9de48dc6a8db332ee8105899dd`, runs 35761967137 and 35761962743 passed: 35 Python and 19 Rust, formatting and Clippy. |

These results belong only to their stated source/evidence. They are not current-head
passes by inheritance. CA-03C's protected test-parent and WinRT apartment corrections
preserved production restrictions. File flush/write-through and process restart are
not proof of physical-power-loss ordering or a directory-metadata persistence
primitive; F-012/T-21/T-22 and full Q1 remain open. Whole-vault replay, administrators,
same-user malware, backups and secure physical erasure are not excluded.

Owner review/merge of PR #3 adopted the bounded P02-A08 T-07 model/projection and
v2 provenance migration. Original source bytes still reconstruct to the retained
digest; all 43 requirements and 34 acceptance IDs remain. That amendment is not a
qualification record. Native discovery and structural identity/generation evidence
do not prove effective official identity, process quiescence or live operation safety.

## 2026-10-15: PR #17 post-merge native failure and runner repair

PR #17 merged as `96767e850fa503e975505ad976c89aa2a0502d8f` after source
`2261c3b385cd3948cb4f685a27b92487602cf73c` passed all six jobs and 108 named
native executions in run `38770382714`. Its post-merge run `38772365458` failed
debug `ca04c_process_restart_restoration_boundaries` (64.547 seconds) and release
`ca04c_process_restart_forward_boundaries` (82.375 seconds). Their nested storage/
coordinator categories were omitted by the reporter; the exact cause remains unknown.
The earlier green run does not erase this failure. A local raw debug reproduction
finished assertions in 191.64 seconds, exceeding the required 180-second gate;
release forward finished in 121.41 seconds. Neither explains the original errors.

The follow-up retains only allowlisted nested error categories and bounded partial
diagnostics on timeout. Native cases and prebuilds now start suspended, join their
own kill-on-close job before resume, and require owned root/member exit plus empty
job accounting. Unproven cleanup blocks later launches in that interpreter.
The case deadline stays 180 seconds; cleanup has a separate five-second failure
bound. All 54 identities per profile, serial outer restart execution, two ordinary
workers and no-retry behavior remain required. This fixes process ownership and
observability; it is not a claim to have fixed the original storage failure.

Fresh local verification passed 224 Python tests with one existing POSIX-only skip,
including 12 supervisor tests and four actual Windows fixture methods. The strict
Windows proof gate rejects missing, duplicate, skipped or expected-failure fixtures;
CI requires that gate before the unchanged native inventory. Actual nested-job and
discovery integration passed four selected case/profile invocations. Independent
security review found no material issue. The final combined-head hosted result is
recorded above; no Desktop or release qualification follows from these tests.

## Historical next-implementation checkpoint

Finish the CA-05C/D -> CA-05E review stack and inspect its exact-head checks. `next-pr.md` selects the
bounded follow-on, not automatic runtime/login/UI integration. The single roadmap,
root entry point, handoff and open-asks register retain the remaining gates.


## 2026-10-21 functional Windows integration

Local working tree in `.worktrees/native-signer182`; no integration commit, PR,
merge or public release yet. The following checks establish their stated scope:

- Optimized `codex-accounts-desktop` build passes offline using the existing pinned
  dependency cache. All 407 external package identities/checksums are retained.
- Combined workspace all-target Clippy passes with warnings denied.
- Runtime: 148 unit cases, 14 framing cases, 11 JSON cases and 5 doctests pass.
  Desktop: 5 Rust cases pass. Synthetic browser flows cover Add/cancel, closing
  during cleanup, rename, quota projection, switching/recovery and the guide.
- Current capture/rename, isolated duplicate-aware save and native private scratch
  cleanup checks pass. No real account payloads are used as test fixtures.
- The actual current official helper passes initialize, initialized, configuration
  requirements, configuration read, explicit owned-job exit and final installation
  revalidation in a new empty synthetic home. Both ordinary and current provisioned
  registered-core selection have been exercised; no auth.json is created.
- Python: 260 checks pass with one existing platform skip. After document/source
  binding updates, the 46 affected checks pass. Specification integrity retains
  43 requirements and 34 acceptance scenarios; no full release closure is claimed.

The release executable renders the Accounts and Diagnostics interface and selects
installed Store version 26.1016.6573.0. That number records this observation only;
normal updates use operation-time checks and require no version approval.
A UI opening smoke exposed and fixed an absent-login-namespace startup check
that incorrectly reported vault recovery. The corrected optimized executable
opens the existing protected empty vault and enables account controls. Its SHA256
is a31018f7f1bd2422ba00fdeed326dc5c2b3eb2adb5ad76be6e70d09fa1e68ef9.
Real browser login, account acceptance and A â†’ B â†’ A are still pending.
The owner will perform the interactive sign-ins; macOS native delivery is not run.
