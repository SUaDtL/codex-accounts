# Validation and evidence boundary

## Current review: CA-03C / PR #6

CA-03C implements protected Windows x64 persistence, encrypted registry/identity/
immutable generations, independent expected contexts, write-ahead storage commits,
retention and startup/explicit control recovery. Product credential authority is
still disabled. No real auth input, login, Desktop process action or qualification
record is introduced. The owner merged PR #5 into the CA-03A branch; PR #6 preserves
that dependency and must not be merged into it. PR #3's T-07 amendment remains subject
to its own review. Read live refs before retargeting or selecting a subsequent base.

| Evidence | Observed scope |
| --- | --- |
| Local toolchain | Actual Rust 1.90.0, rustfmt and Clippy restored from checked development-input artifact for source 4c09923ea8c19571fe83bdbf96e453732be734a9; cached package checksums and archive digests verified. Preparation is not a test pass. |
| Local closeout | Workspace tests and Clippy passed on Linux; 22 storage model scenarios include strengthened per-effect forward/restoration/control-repair fault tests. Windows GNU cross-target Clippy passed; it is not native Windows execution. |
| Local integrity | 75 Python tests, specification/model/projection/provenance and exact 58-package dependency checks passed. Diff/secret/scope review and preserving-tree comparisons passed for the selected source. |
| Implementation-head native checks | Head 4a122159089a593cc3bf2559bdc4ec2eb424f993, PR run 35933092487: all four jobs passed. Windows job 107423943943 records 129 Rust passes including 40 storage cases; three inherited two-user helpers were NOT RUN. The final documentation head is checked separately below. |
| Final-head CI | PR #6 records the actual final source commit, completed PR/push runs and inspected logs after publication. Only results bound to that final head count as current passes. |
| Native release qualification | Owner Windows 11, ordinary two-user/unavailable-store, sync-provider environment and physical-power-loss/directory-metadata qualification remain NOT RUN or incomplete. macOS storage is unsupported. |

Native repairs preserve protection rules. Hosted LocalAppData ownership did not
satisfy the direct-parent rule; tests now create a new protected synthetic parent,
never change the existing parent's owner/DACL. Repeated per-call apartment teardown
produced refusals and a child access violation. The adapter now owns one balanced
thread-lifetime MTA and uncached, scoped activation factories. Fresh successful sync
inventory remains mandatory. Fixed-local-drive checks run before object access;
registered sync-root ancestors cannot be followed through unchecked reparse points.

The storage recovery review closed the blocked-open gap for a torn control stage:
opening authenticates the committed reachable state, blocks mutations and offers an
explicit action that preserves encrypted evidence before exact-stage removal. A
valid staged control also checks inventory before publication. Tests no longer count
unreopenable storage as successful recovery merely because some old bytes survived.
Recovery evidence is bounded, never operational authority and never automatically
pruned. Corrupt committed state, foreign files and stale stages continue to refuse.

Native tests use only newly created synthetic directories: protected creation,
bootstrap/reopen, file replacement/deletion, broad-ACL and hardlink/junction refusal,
sharing, locks, repeated WinRT inventory, torn-control recovery and same-test-binary
process restart at 20 commit boundaries. They do not run Codex or read real credentials.
Three inherited two-user helpers remain intentionally NOT RUN and are not counted
as passes. No skip or relaxed assertion repairs a failing ordinary test.

File write-through, explicit file flush and handle rename tests are not full hardware
power-loss ordering or a qualified directory-metadata flush. F-012/T-21/T-22 and full
Q1 remain open. Same-user hostile code, administrators, whole-vault replay, swap,
backups and physical erasure are outside stronger guarantees. See ca-03c-storage.md
for exact limits, safe commands, expected results and remaining evidence.

The temporary preparation workflow is removed. All 58 external package/version/
checksum entries and predecessor review bytes remain unchanged. Scoped SDK features
and manifest/lock bytes are checked before acquisition. Existing CI remains blocking;
no main update, merge, force push, release or live account mutation is part of this PR.

## Preserved earlier evidence

| Increment | Exact evidence and scope |
| --- | --- |
| CA-03A / PR #3 | Head `595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`; PR run 35819942583 and push run 35819939120 passed. Source job 107049451062 recorded 62 Python passes; Windows job 107049451273 recorded 66 Rust passes, including 23 vault-data cases and nine discovery/native API cases. Formatting and Clippy passed on all three platforms. These are historical results, not CA-03C validation. |
| CA-02 / PR #2 | Owner merged head `6ee778e9592d46132f253e2bee836b2650a44bd0` as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. Run 35808957582 passed: 48 Python tests and 43 Rust tests in the inspected Windows log. Later T-07 adoption and CI acquisition repair are not attributed to this merge. |
| Foundation / PR #1 | Merge `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`; tested head `9b9328f474322c9de48dc6a8db332ee8105899dd`, runs 35761967137 and 35761962743 passed: 35 Python and 19 Rust tests, formatting and Clippy. |

PR #3 implements the remaining P02-A08 canonical/visible T-07 change and v2
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

CA-04A is the selected bounded storage-backed switch journal/coordinator slice in
next-pr.md, not an automatically executed phase. Persisted active-state/operation-hold
integration is still private/unavailable to production in CA-03C. Native quiescence,
target-home locking, official login, UI and all complete release scenarios remain
pending. Full Q0/Q1 and macOS qualification are not claimed by this handoff.
