# Windows storage compatibility

This increment follows merged PRs #14 and #15 (main `727eb6154db1d7c4ac5db10a7868b6271cd55293`).
It implements the storage portion of the Windows compatibility work. It does not
qualify Desktop or enable account switching.

## Storage behavior

New vaults use the fixed `CodexAccounts-Storage` container below the current
profile directory. Existing legacy vaults under local application data remain in
place. A pure decision table distinguishes absence, directories, inaccessible
objects, unexpected types and changed observations for both slots. Opening
requires exactly one unambiguous existing slot; creation requires both absent.
There is no migration, alternate-root search or access-denied fallback.

A protected location mutex is acquired before resolving the fixed locations.
The native adapter retains handle identities, descriptor observations and the
selected root lock. Legacy observations inspect metadata without opening vault
payloads. Reserved-name collisions, unsafe grants, namespace changes and
unrecognized initialization states refuse before data access or key generation.
The adapter never repairs existing ACLs or changes another application's files.

Authenticated staged initialization and interrupted registry states retain the
existing explicit recovery methods. Constructor-returned cleanup and vault
commit holds block ordinary operations until their actual recovery completes.
Legacy bytes, root keys and registry formats remain unchanged.

## Verification and limits

The local Windows checks cover both Rust profiles, the complete workspace and
its inherited 27 separately executed native cases per profile. An additional
bounded execution bridge verifies exact named storage cases and rejects missing,
duplicate, ignored, failed or malformed results. Negative fixtures verify the
reserved-only collision and pending-recovery assertions without claiming a
production defect or retroactive production RED.

The Python suite has 201 cases, with one existing POSIX-only skip on Windows.
Formatting, Clippy, offline builds, doctests, dependency review, canonical
specification and workflow contracts passed locally. Independent coverage and
security reviews found no remaining findings in the storage scope. Hosted CI on
the published head remains required before merge; local results are not CI.

The macOS v3 packet is a source-only review of the shared recognition change.
Earlier packets are preserved. It establishes no macOS native or Desktop facts.
Three owner-run two-user helpers remain NOT RUN. Power-loss durability, effective
Desktop home/backend/policy, official runtime identity and all full Q0 through
Q5 qualification obligations remain open. The compatibility catalog stays empty.

## Workflow continuation

On 2026-10-15 the owner explicitly directed continuing outside codeArbiter's task,
reconciliation and native-review receipt bookkeeping because the engine blocks
progress and is being repaired separately. The attributed override is retained
in `.codearbiter/overrides.log`. Existing engine records are preserved locally;
they are not rewritten as accepted. Application security, qualification and CI
requirements remain in force.

The next bounded implementation is an explicit registered-package metadata
reader. It must keep default strict inspection unchanged, retain registered-root
and descendant handles, read only bounded manifests and declared images, and
report unchecked outer ancestry and unresolved executable roles truthfully.
It must never read configuration, credentials or history in that mode, launch a
candidate, or convert inventory into qualification authority.
