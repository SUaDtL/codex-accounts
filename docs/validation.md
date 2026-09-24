# Validation and evidence boundary

## Current review: CI assurance / PR #8

Owner-requested CI/CD and coverage review starts from merged main
`103d1a409ea19544ac4a18f33a19df925f5efc58`, tree
`50ad33a56c30b3a0bb6bbfde444c39f98457cc62`. PR #7 is merged; its old draft body is
not the current implementation state. This repair changes CI, its executable
regressions, documentation/API doctests and the maintained evidence records, not
credential behavior. It does not start CA-04B or qualify a platform.

Baseline main run [35962095930](https://github.com/SUaDtL/codex-accounts/actions/runs/35962095930)
completed successfully. Inspected source job 107512653814 records **78 Python
passes**, 58 reviewed external packages, 43 requirements, 34 acceptance IDs and zero
qualified records. Inspected Windows job 107512653966 records **153 Rust passes**:
13 core, 16 discovery, two diagnostic, three platform, nine runtime, 23 vault-data,
23 crypto and 64 storage. Storage includes 44 generic cases and 20 Windows cases,
including two child-test entry points. Three inherited two-user helpers were
ignored / NOT RUN, excluded from pass counts. The Windows host was Server 2025 x64
build 26100, not owner Windows 11 or an official Desktop installation.

PR #8 adds independent source checks, exhaustive test-executable continuation,
documentation tests, optimized compilation, host checks and strict step/job outcome
assessment. Native test matrix and warnings remain blocking. Dependency records,
lockfiles, specification/provenance and qualification catalog remain unchanged.
The checkout SHA is retained; its Node 20 deprecation warning is not claimed fixed.

The final PR head must have a completed PR-triggered run with all four validation
jobs and the aggregate **CI gate** successful. Record that exact head, tested merge
SHA, run and inspected logs in PR #8. Earlier green heads do not establish later
results. Optimized compilation is not optimized test execution, packaging or release.
Local container and Python tools failed before running commands in this review;
no new local test pass is claimed. Hosted validation is distinct from that limitation.

See `ci-review.md` for findings, coverage gaps, owner-required main protection,
executable commands and rollback. Cargo `--offline` is not a build/test OS network
sandbox. Full network isolation, current advisory automation, complete release
provenance, physical-power-loss and native qualification remain open. The gate
uses actual outcomes; a skip, cancellation, missing result or masked failure is not
success. Its regression checker cannot replace trusted review of changes to CI itself.

## Merged CA-04A evidence and limits

PR #7 merged head `bfe30a0eee174dfe107e5b7edc07d5c637408e28` as the baseline above.
That head finished the storage wiring and tests after the intermediate `43902b9...`
commit. The initial failure was a dependency-manifest mismatch before Rust execution;
the reviewed existing-core path edge was recorded without adding external packages.
Temporary preparation was removed. No production effects adapter or live-switch
entry point was added. The merged-head run above supersedes stale pending-CI text.

Prior implementation-local records report Rust 1.90.0 workspace tests, formatting,
Clippy and 78 Python passes, including 44 generic storage tests. These are preserved
historical records, not executions by this CI-review session. Source was restored
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

After the CI assurance review, CA-04B remains the next product packet: native
owner/canonical-home lifetime locking, writer observation and actual exit/descendant
proof. It has not been started here. Reuse the existing journal; do not recreate
CA-04A, widen public effects, or manufacture consent/policy/compatibility evidence.
Full Q0/Q1/Q2, macOS, official login, UI and release qualification remain open.
