# Next bounded slice: CA-04A

Parent packet: CA-04 / Q2. Read live refs, AGENTS.md, PR #3 and PR #6 before choosing
a base. CA-03C is submitted in PR #6 on the CA-03A dependency branch, after the owner
merged CA-03B there. Do not reset that history or merge into the dependency branch.
Reconcile/retarget only after the relevant owner merge. Current CI is recorded on
the actual PR head, not established by this pointer.

## Deliverable

Implement a storage-backed encrypted switch journal and executable coordinator
for synthetic resource effects. Use the existing persistent generation/registry
engine, not a second filesystem transaction implementation or trait-only scaffold.
The journal must durably bind operation ID, source/target generations, expected
resource presence/integrity, per-resource intent/completion, cancellation, primary
failure and restoration outcome. Journal references must participate in the vault's
complete persisted retention holds in the same commit protocol. A caller-supplied
partial reference list or active=true value cannot authorize deletion or switching.

Drive explicit requested/locked/quiescent/source-saved/staged/installed/observed/
committed/relaunch/confirmation and recovery transitions. Before each modeled
external replacement, persist its intent and verify prerequisites. Preserve exact
newest generations; helper refresh must be captured before restoration. Reuse the
separate installation/acceptance/launch/Desktop-identity/recovery result types.
Cancellation after the first write enters reconciliation; launch failure after
commit never silently selects an older account. Restoration itself is journaled.

The concrete executable effects in this packet are encrypted switcher-owned
storage and controlled synthetic resources only. No production adapter may accept
fixture evidence as quiescence, policy, consent, verified identity or compatibility.
A public credential CLI, arbitrary filesystem/RPC adapter, forged qualification
receipt, live auth replacement or normal-user application termination remains
prohibited. Actual native writer discovery and the target-home lock are separate
CA-04 work; the current vault-root lock is not their completed implementation.

## Read and test

Read SPEC-TRANSACTION, SPEC-RECOVERY, F-009 through F-014, F-019 through F-025,
S-003 through S-009, F-026 and the complete relevant T-13/T-14/T-18 through T-23/
T-27 through T-31 scenarios. Read the current storage/crypto contracts and tests,
not just this outline. The plan does not amend those requirements.

Test full synthetic A0->A1->B->A1, post-helper B1/restoration, every resource presence
combination, stale/unknown external bytes, duplicate operations, startup recovery,
cancellation at every stage and faults before/after each durable forward/restoration
write. Restart using persisted state and verify generation reachability/holds.
Test primary and restoration errors remain distinct. No successful helper response
or file installation becomes Desktop confirmation. Corrupt evidence and missing
preconditions must prevent the corresponding transition, not be silently skipped.

Run pinned formatting, workspace compilation/tests/Clippy, Python and source/
projection/provenance/dependency checks. Add native tests only for actually executed
OS effects. Inspect final-head CI and logs, preserve blocking gates and update the
single roadmap/validation record. Exact new dependencies/native bindings require
scoped review before acquisition. Publish one normal review PR; no merge, force
push, reset, release, permission changes or live account operation.

## Unresolved integration gates

Q0 publisher/runtime/home/backend/policy/resource/identity/lifecycle evidence,
CA-03B ordinary two-user/unavailable-store tests, Windows physical-power-loss and
directory-metadata durability qualification, and independent macOS persistence
remain open. ca-03c-storage.md and ca-03b-crypto.md contain safe synthetic collection
steps and redaction rules. Missing evidence blocks real integration and release,
not this explicitly bounded generic implementation. No full Q1/Q2 closure or
CA-05 advancement is implied by selecting CA-04A.
