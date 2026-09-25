# Validation and evidence boundary

## Current review: CA-04B / PR #9

CA-04B implements private Windows x64 owner/canonical-home locking, handle-bound
process identity and normal-quit observations, retained descendant tracking and
owned-job exit evidence. The journal, credential formats and public mutation
boundary remain unchanged. All effects tests use newly created synthetic objects
and exclusively owned children. No official Desktop or real credential is used.

PR #9 continues from merged main `27afd3381958b68154947e7f8d0092b214c226c6`.
Existing published work was recovered from its exact committed-source artifact,
whose tree matched `7fa24f2b7c4d8f98be77e304094ad68ede7f35d5`; no older source
archive replaced that work. Local toolchain/dependency acquisition was checked
separately and is not counted as validation.

| Executed evidence before documentation closeout | Observed result |
| --- | --- |
| Local source | 114 Python tests, specification/model/projection/provenance, workflow contract and exact 58-package dependency review passed. All 43 requirements and 34 acceptance IDs remain; zero qualified records. |
| Local Linux execution | Pinned Rust 1.90.0 formatting, workspace all-target debug and optimized tests, three doctests and Clippy with `-D warnings` passed against recovered CA-04B code. Windows-only effects were not executed locally. |
| Hosted source head | `078dc4c4a9ebfc6a7fe937e1a85b892b30c8bdff`, tree `3bb00602d5d94b8626fccbec5d38e2b1e0cd80a8`. Run [36107357329](https://github.com/SUaDtL/codex-accounts/actions/runs/36107357329) completed successfully: source, three Rust lanes, isolated Linux and aggregate CI gate. |
| Inspected Windows job | `107982842804`: **176 workspace tests passed in debug and again in release**, including 87 storage/lifecycle cases, plus three doctests. Named verification separately executed all eleven CA-04B behaviors in each profile: **22 passing records**, not 22 new distinct behaviors. Formatting, optimized compilation and Clippy passed. |
| Tested GitHub merge | `a4c9f1f3776a655ec2436f0a46ef547a1cfc8ea9`, whose tree equals the stated source tree. This is a CI test merge, not an assistant merge. |
| Native limits | Windows Server 2025 x64 build 26100, not owner Windows 11 or ordinary-user qualification. Three inherited two-user helpers were ignored / NOT RUN, excluded from pass counts. Workspace counts include three child-test entry points, not extra behavior proofs. |

These records bind only the stated source. The documentation-closeout commit must
have its own completed checks; final head, tested merge, run and inspected logs are
recorded on PR #9 before readiness. No older green result is transferred to a later
head, and native success does not close a complete T-01 through T-34 scenario.

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

After CA-04B review and verified final-head checks, CA-04C is the next bounded
product slice: a private target-resource adapter exercised against synthetic homes
through the existing journal. See `next-pr.md`. It has not started. Native owned
helper construction/private stdio for the official runtime remains CA-05 work.
No fixture supplies effective home, qualified policy, user consent or authority to
mutate live resources. Full Q0/Q1/Q2, macOS, login, UI and release remain open.
