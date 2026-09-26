# Validation and evidence boundary

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

## Next eligible implementation

Finish the combined CA-05C/CA-05D review and inspect its exact-head checks. `next-pr.md` selects the
bounded follow-on, not automatic runtime/login/UI integration. The single roadmap,
root entry point, handoff and open-asks register retain the remaining gates.
