# Q0: prove one exact Desktop contract

Status: NOT QUALIFIED. The inventory tool is a starting point, not completion.
Use a disposable, owner-controlled Windows 11 x64 or macOS arm64 test environment.
Linux validates development logic only. Never collect evidence from a production
account or share real authentication files with an agent.

## Prerequisites and permissions

The owner identifies the official Desktop package, its bundled runtime and the
actual candidate Codex home. Inventory requires ordinary-user read access only.
Do not elevate, change ACLs, disable security controls, stop processes or change
config to make inventory succeed. Use organization-approved test accounts and
confirm applicable workspace/login policy before any later integration trial.

## Read-only discovery

Run `tools/q0_inventory.py` with three explicit absolute paths. It does not consult
PATH, run a shell, launch either executable, open auth.json, copy history or use a
network. Review `qualified: false` and the unresolved evidence array even after a
zero exit status. Optional local-path output must stay on that machine.

Record official package/bundle identity, publisher/signature, exact version,
executable hashes, OS build/architecture and authoritative home resolution using
trusted native inspection. A filename or valid local file hash is not publisher
verification. Inspect both Desktop and runtime, not just a standalone CLI.

Resolve configuration precedence, file/keyring/auto/secrets auth, managed policy,
environment authentication and runtime overrides. The probe reads only the
candidate home's top-level declared setting; it cannot establish effective state.
Missing file/setting is unknown, permission denied is not missing, and `auto` is
not eligible merely because auth.json also exists.

## Contract evidence still required

1. Derive the approved runtime's exact protocol schema only after signature/path
   binding is reviewed. Do not execute the first `codex` found on PATH.
2. Establish the full auth-resource set, absence semantics and ownership. No
   companion file is qualified by its name. Never rewrite global state/history.
3. Establish normal quit, descendant/writer discovery, 30-second stop behavior,
   normal reopen and application identity observation. Do not force-kill.
4. Prove policy reproduction for staging login without copying untrusted hooks or
   weakening requirements. Establish network/startup/refresh effects explicitly.
5. After Q1-Q3 are implemented and tested, perform isolated owner-run A->B->A trials
   with newest-generation preservation, rejection/offline/launch failures and
   crash recovery. Q0 discovery alone does not authorize this step today.

Keep raw observations outside Git; author a reviewed compatibility record only
when every field has evidence. No model test or fake runtime is a qualification.

## Verification and rollback

The current inventory has no filesystem write or process-control operation, so
there is no application-state rollback to perform. Verify this using synthetic
before/after fixtures and native filesystem/process monitoring. A failed inventory
must return a typed error without changing the candidate home. Later live trials
require the spec's transactional rollback; do not invent a backup-copy shortcut.

## Specification ambiguity to resolve before enabling an adapter

F-006 and SPEC-DECISIONS allow a positively established, qualified official file
*default*, while T-07's wording says only qualified *explicit* file mode proceeds.
The source document has been preserved without silently resolving that distinction.
This increment permits neither. Obtain a deliberate spec/model+projection edit and
corresponding test intent before implementing default-file eligibility.
