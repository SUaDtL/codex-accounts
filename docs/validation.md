# Validation and evidence boundary

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

## CA-06B: Windows static developer shell candidate

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
