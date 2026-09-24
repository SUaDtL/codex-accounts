# Validation and evidence boundary

## Current review: CA-04A / PR #7

CA-04A implements a private executable switch coordinator and authenticated encrypted
journals inside the existing storage commit protocol. Production effects, real
account inputs, native Desktop process authority, login and qualification records
remain absent. PR #7 targets main and preserves the reviewed CA-03C dependency:
PR #6 was merged by the owner into the former CA-03A branch after PR #3 merged.
No branch reset, automatic merge or replacement of that history is part of this work.

| Evidence | Observed scope |
| --- | --- |
| Source baseline | Continued review branch at `43902b9beeff27d06abf3e440bec29f0bfb0baa4`, source tree `1f902b637f9e584c43340c2dbcf43f5a1e5e58e4`, recovered and checked against its Git source artifact. |
| Local toolchain | Rust 1.90.0, rustfmt and Clippy from the digest-checked existing development inputs. Preparation/acquisition does not count as validation. |
| Local executable checks | `cargo test --workspace --all-targets --locked --offline` passed, including 44 storage cases: 22 inherited storage plus 22 journal/coordinator tests. Formatting and workspace all-targets Clippy with `-D warnings` passed. |
| Local source checks | 78 Python tests passed; specification/model/projection/provenance and exact 58-package manifest/lock checks passed. |
| Native checks | Two new Windows tests exercise actual DPAPI/storage reopen for completed and interrupted journals; external resource/helper/launch effects remain synthetic. Local Windows execution is unavailable. Final-head hosted results and inspected logs must be recorded on PR #7 before marking ready. |
| Release evidence | Actual Desktop contract, owner Windows 11, ordinary two-user/unavailable-store, sync-provider, physical-power-loss/directory-metadata and independent macOS qualification remain open. |

The fault suite injects every recorded storage effect in forward/restoration
transitions and request/cancel/recovery-choice commits. It discards runtime session
state on reopen and requires a verified reachable newest generation, exact external
synthetic bytes, complete journal holds and bounded cleanup. External stage/replace/
helper/reap/cleanup/launch failures are separately injected. A0-to-A1-to-B-to-A1,
post-helper B1 restoration, optional absence combinations, late binding/policy/writer
failure, duplicate requests, conflict evidence, typed helper policy failure and
uncertain launch outcomes are tested. Neither an I/O error after publication nor an
old surviving file justifies fabricating a rollback or Desktop-confirmed state.

The initial unfinished PR head failed the dependency-manifest gate before Rust
execution. The repair adds the missing local core dependency edge to the lock and
records its scoped review separately; all 58 external records and predecessor review
bytes remain unchanged. Temporary preparation is removed. No blocking test,
platform, warning policy, safety lint or CI condition is removed or weakened.
No new external package or native API is added.

The journal is an explicit `CAREG002` registry extension committed together with
selection and complete retention holds. Existing `CAREG001` data is read without a
rewrite; conversion occurs through the tested recoverable first-journal commit.
No key migration or silently accepted unknown codec exists. Unresolved switch
records block public profile writes/pruning. Conflict evidence is bounded and
retained, never accepted as a valid credential generation or erased automatically.
The private synthetic effects driver cannot be selected through the public API.

Synthetic and hosted native results cover the exact tested source only. Final-head
PR/push results are recorded on PR #7 after publication; older or preparation-run
results do not count. No complete T-01 through T-34 release scenario is closed.
See `ca-04a-journal.md` for executable commands, expected native test counts and
redaction/cleanup rules. No real auth files, account identities, keys, user paths
or raw helper output should be shared.

## Preserved CA-03C final evidence

PR #6 head `6c8144f041ba16be1b6c33c1de6f3d1f6aa8dfb8` passed PR run
35933850694 and push run 35933846776. Inspected source/Windows logs recorded
75 Python and 129 Rust passes, including 40 storage cases; three inherited two-user
helpers were NOT RUN. Native tests exercised synthetic protected storage, lock
release, link/ownership/sharing refusal, WinRT enumeration, control repair and
20 process-restart boundaries. Those results apply to CA-03C, not CA-04A.

The native protected test-parent and apartment-lifetime corrections preserve
production path/ownership restrictions. The native unsafe boundary is unchanged
by CA-04A. File flush/write-through and process restart are not proof of physical
power-loss ordering or a qualified directory-metadata persistence primitive.
F-012/T-21/T-22 and full Q1 remain open. Same-user malware, administrators,
whole-vault replay, backups and secure physical erasure are not excluded.

## Preserved earlier evidence

| Increment | Exact evidence and scope |
| --- | --- |
| CA-03A / PR #3 | Head `595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`; PR run 35819942583 and push run 35819939120 passed. Source job 107049451062 recorded 62 Python passes; Windows job 107049451273 recorded 66 Rust passes, including 23 vault-data cases and nine discovery/native API cases. Formatting and Clippy passed on all three platforms. These are historical results, not CA-03C validation. |
| CA-02 / PR #2 | Owner merged head `6ee778e9592d46132f253e2bee836b2650a44bd0` as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. Run 35808957582 passed: 48 Python tests and 43 Rust tests in the inspected Windows log. Later T-07 adoption and CI acquisition repair are not attributed to this merge. |
| Foundation / PR #1 | Merge `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`; tested head `9b9328f474322c9de48dc6a8db332ee8105899dd`, runs 35761967137 and 35761962743 passed: 35 Python and 19 Rust tests, formatting and Clippy. |

The owner merge of PR #3 includes the remaining P02-A08 canonical/visible T-07 change and v2
provenance migration. Original bytes reconstruct to the preserved digest; all
43 requirements and 34 acceptance IDs remain. Review/merge of that amendment is
not a qualification record. CA-03A supplies strict data/generation rules, not
verified Codex identity, durable storage or complete recovery references.

CA-02's native reader, conservative configuration observations, path/refusal and
redaction tests are delivered. Actual official publisher/runtime binding,
effective home/policy, auth resources/identity, writer lifecycle and actor-attributed
traces remain Q0 evidence gaps. Earlier hosted Windows was Server 2025 x64, not an
owner's Windows 11 Desktop qualification. See q0-qualification.md and PR #2's
checklist disposition for the detailed discovery evidence.


## Preserved CA-03B final evidence

PR #5 head 9fd8e6902ce909dfcad39fbb40f48e6af81bc17e passed PR run 35897568260
and push run 35897560936. Inspected logs recorded 71 Python and 89 Windows Rust
passes, including 23 crypto cases; three two-user helpers were ignored/NOT RUN.
Those results apply to CA-03B, not later storage changes. Its scoped crypto, KDF,
randomness, zeroization, DPAPI and advisory evidence remains in ca-03b-crypto.md and
ca-03b-dependencies.json. The owner merge into the dependency branch does not itself
qualify native storage, validate Desktop behavior or close the remaining release gates.

## Next eligible implementation

CA-04B: native owner/canonical-home lifetime locking, writer discovery and actual
exit/descendant proof. This is a future bounded packet in `next-pr.md`, not an
automatically executed phase. Reuse CA-04A's journal; do not expose synthetic
quiescence, consent, policy or compatibility as production authority. Full Q0/Q1/Q2,
macOS, official login, UI and release qualification remain open.
