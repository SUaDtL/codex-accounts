# Next bounded slice: CA-04B

Parent packet: CA-04 / Q2. CA-04A is submitted in PR #7 against main. Inspect live
refs, AGENTS.md, its actual diff and final-head CI before selecting a base. The PR
preserves the CA-03C dependency that the owner merged into the former CA-03A branch.
Do not reset history, merge a dependency automatically, or duplicate completed work.

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

Run pinned formatting, workspace compilation/tests/Clippy, Python/source/projection/
provenance/dependency checks, and selected native OS tests. Inspect completed checks
for the actual final PR head. Update the single working roadmap and validation record;
record unobserved native cases honestly. Open/update one normal review PR. No merge,
force push/reset, permission change, release or live account operation.

## Unresolved integration gates

Q0 publisher/runtime/home/backend/policy/resource/identity/lifecycle evidence,
ordinary two-user/unavailable-store protection, owner Windows 11/sync-provider
qualification, physical power-loss and directory-metadata persistence, and
independent macOS remain open. Native process tests with synthetic children do not
qualify Codex Desktop. Provide exact safe collection steps and redacted expected
results for remaining evidence; never request real authentication files.

CA-04B has not been started by CA-04A's handoff. Full CA-04/Q2 remains open; no
CA-05 advancement or production authority follows from a code or test pass alone.
