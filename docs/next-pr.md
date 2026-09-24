# Next bounded product slice: CA-04B

Parent packet: CA-04 / Q2. CA-04A and the preserved CA-03C dependency are merged
through PR #7. Verified main baseline is `103d1a409ea19544ac4a18f33a19df925f5efc58`;
main run 35962095930 passed. Inspect live refs, AGENTS.md, current open PRs and actual
head checks before choosing a base. This snapshot is not permission to reset history.

The owner requested a CI/CD and coverage review before resuming product work.
PR #8 carries that bounded assurance repair and `docs/ci-review.md` records fixed
and unresolved gaps. Review it first; do not assume it merged or recreate its work.
CA-04B remains selected but is not started by the CI review. The main `CI gate`
protection setting requires separate owner action; a workflow edit does not enable it.

## Deliverable

Implement concrete Windows x64 owner/canonical-target-home lifetime locking and
bounded writer/process observations for the future coordinator adapter. Reuse the
existing private journal and storage engine; do not create a second transaction
protocol or an arbitrary public process/filesystem executor.

Bind the kernel lock to the observed canonical home identity and current owner,
not only an unnormalized pathname or marker file. Prove contention and release on
exit with controlled child processes. The existing vault-root lock is not a
substitute for this target-home lock. Detect path/owner replacement, reparse and
hardlink hazards; never repair arbitrary directory permissions.

Implement bounded process identity/liveness observations, including owner, stable
start identity and descendant tracking. A PID, name substring or successful signal
is not exit proof. Use exact qualified installation/runtime bindings when available;
without the necessary Q0 rules report unknown and do not authorize mutation. Never
terminate a user-owned Desktop, IDE or terminal in ordinary tests. Normal-quit and
owned-helper timeout behavior must be tested with controlled synthetic children
before any later real integration. Do not infer shared-home association from a
process name, current directory or an unverified command line alone.

Keep effect authority private and impossible to manufacture through deserialized
booleans, a public trait implementation, a developer override or a fake receipt.
Do not connect synthetic evidence to production switching. No live credential read
or replacement, login, inference, task replay, UI, installer or release belongs in
this slice. Exact Q0 policy/home/resource facts and filesystem durability remain
separate prerequisite gates.

## Required reads and validation

Read SPEC-PROCESSES, SPEC-TRANSACTION, SPEC-RECOVERY, F-004 through F-014,
F-019/F-024 and the complete T-06/T-14/T-17/T-19/T-20/T-21/T-22 scenarios. Read the
current discovery, storage and CA-04A journal contracts and relevant tests. The
plan cannot silently amend these requirements.

Review any new SDK feature/native binding against exact pinned source and official
documentation before acquisition. Keep the unsafe boundary small and justified;
retain safe-core and coordinator restrictions. Do not upgrade unrelated packages
or change CI failure semantics as incidental cleanup.

Test actual kernel contention/release, Unicode/canonical identity and replacement,
wrong owner, denied/partial enumeration, process disappearance and reused PID/start
identity, hidden/late descendants, normal quit refusal/timeout and helper signal
versus observed exit. Native tests must own their newly created directories and
children; no global process kills, account creation or permission changes. Preserve
journal refusal and restart tests when integrating observed prerequisites.

Run the current pinned formatting, workspace compilation/tests/Clippy, documentation
tests, optimized compilation, Python/source/projection/provenance/dependency and
workflow-contract checks, plus selected native OS tests. Inspect completed results
for the actual final PR head and test-merge SHA. Cargo-offline execution is not an
OS network sandbox. Update the single roadmap and validation record; report missing
native evidence honestly. Publish one normal review PR; no merge, force push/reset,
permission change, release or live account operation.

## Unresolved integration gates

Q0 publisher/runtime/home/backend/policy/resource/identity/lifecycle evidence,
ordinary two-user/unavailable-store protection, owner Windows 11/sync-provider
qualification, physical power-loss and directory-metadata persistence, and
independent macOS remain open. Native process tests with synthetic children do not
qualify Codex Desktop. Provide exact safe collection steps and redacted expected
results for remaining evidence; never request real authentication files.

CI review follow-ups include owner-enforced main protection, a scoped checkout
runtime upgrade, actual build-network isolation, refreshed advisory tooling and
future measured coverage/release provenance. Do not represent those as resolved by
a green test count. Full CA-04/Q2 remains open; no CA-05 advancement or production
authority follows from a code or test pass alone.
