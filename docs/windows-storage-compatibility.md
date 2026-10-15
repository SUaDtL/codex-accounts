# Windows storage and metadata compatibility

This increment follows merged PRs #14 and #15 (main `727eb6154db1d7c4ac5db10a7868b6271cd55293`).
It implements guarded storage placement and registered metadata inspection. It does not
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

The storage baseline ran 201 Python cases. The combined increment runs 206,
with one existing POSIX-only skip on Windows.
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

The explicit registered-package metadata reader now retains registered-root and
descendant handles, reads bounded manifests and declared images, and reports
unchecked outer ancestry and unresolved executable roles truthfully. Default
strict inspection remains unchanged. Real discovery library and diagnostic tests
pass in both profiles; independent security review found zero actionable findings.
Three temporary safeguard removals each failed the intended native assertion in
both profiles before restoration. The reader never reads configuration,
credentials or history in that mode, launches a candidate, or converts inventory
into qualification authority. See [the Q0 procedure](q0-qualification.md).

PR #16's CI required 51 named native cases per profile: all 27 inherited cases, 18
storage cases and six metadata cases. The workspace complement is checked against
its actual compiled inventory. A hosted source-job failure exposed the real Rust
adversary's missing toolchain prerequisite; the reviewed 1.90.0 installation and
its successful outcome are now mandatory, with the actual adversary unchanged.
The local expanded storage run passed all 18 new cases in both profiles, but the
inherited restoration restart test exceeded its unchanged 180-second limit in
both profiles. Those failures remain recorded. The final source subsequently
passed all 102 named native executions locally and in inspected hosted output;
all six jobs passed in run 38761237982. PR #16 merged as
`9389c33b7bc8d41fee92f14046eebd546c5c86f2`. These results are exact-source
implementation evidence, not Desktop qualification.

The unchanged restoration test subsequently passed alone in 159 seconds under
the same 180-second deadline. Restart suites already run four fixture workers
internally; the outer runner now awaits each restart suite before submitting the
ordinary two-worker pool. No test, assertion, deadline or failure is removed, and
failed cases are not retried by the runner.

## Read-only host observation, 2026-10-15

The selected registered Windows x64 package reports version `26.1007.9446.0`.
Initial metadata inspection exposed a valid UTF-8 BOM rejected by the XML string
parser. A synthetic regression failed in both profiles before the shared parser
was changed to remove exactly one leading BOM after strict UTF-8 validation.
Double, nonleading and malformed prefixes still refuse.

After rebuilding, the real registered-metadata command completed successfully:
one package, two declared x64 images, matching manifest identity and registration
stable during inspection. Cached signatures were observations only. Publisher
binding, runtime role, effective home/backend/policy and outer ancestry remain
unestablished. Default strict inspection still refuses with `E_ACCESS_DENIED`.
No configuration, credential, account or history material was read, and no
candidate executable was launched. These facts do not qualify a Desktop build.
