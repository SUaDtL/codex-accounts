# CA-04A: encrypted switch journal and recoverable coordinator

CA-04A connects a private executable coordinator to the existing CA-03C storage
commit engine. It implements bounded synthetic slices of F-013/F-014/F-019 through
F-026, S-003/S-005/S-006/S-009 and their recovery scenarios. No full Q0/Q1/Q2 or
release scenario is closed. No production effects implementation, switch entry
point, credential CLI, actual Codex file, runtime, process, login or network action
is provided. The only effect driver is compiled into tests.

## Persistence and authority

The encrypted registry contains up to 16 operation journals, with at most one
nonterminal operation. Each journal binds a random operation UUID, opaque
installation/home binding digest, source and original/current target generations,
resource presence and keyed fingerprints, staging/apply/restore intent and completion
bitmaps, helper ownership intent, cancellation, primary/restoration errors, and
separate acceptance/launch observations. The registry's root-bound authenticated
control record selects its envelope context independently of untrusted bytes.
Embedding journals inside that authenticated registry makes journal, selected
profile, generation references and retention holds one storage commit, rather than
introducing a second filesystem transaction implementation.

The codec reads existing `CAREG001` registries without rewriting them. The first
journal commit selects the explicit `CAREG002` extension through the same recoverable
write protocol. The old reader rejects that new magic. Both versions are bounded
and validated; there is no key migration, silent downgrade or permissive fallback.
The envelope format and primitive algorithms do not change. Journal fingerprints
have a fixed-width storage codec used only inside authenticated encrypted metadata;
they are neither diagnostics nor proof that a live file or Desktop identity matches.

Public construction and storage paths retain CA-03C's restrictions. The private
coordinator receives only enumerated effects and observations, not filesystem paths,
executables, RPC methods or an arbitrary public callback. An in-memory operation
session is lost on reopen; persisted state alone cannot authorize normal dispatch.
Recovery requires a fresh explicit choice and re-establishment of the indicated
binding, policy, lock and quiescence prerequisites. Those prerequisites are modeled
by controlled tests here, not evidenced for any Desktop installation.

## Execution and failure rules

| Stage | Enforced behavior |
| --- | --- |
| Request and quiescence | Check selected latest generations and matching resource contract; confirm, lock, then quiesce before reading the outgoing snapshot. Refuse overlapping requests and stale targets. |
| Source preservation | Capture exact newest matching-principal bytes and explicit absence. Commit the source generation, journal and holds before any external replacement. Unknown identity requires reconciliation. |
| Staging and installation | Persist registration before staging secret bytes; persist each resource intent before replacement. Recheck guards and expected bytes around effects, then persist completion. A rename is not a whole-set atomic operation. |
| Optional helper | Persist operation-scoped ownership intent before start. A response or signal is not exit proof. Reap the owned helper/descendants and save any newest matching target generation before commit or restoration. |
| Observation | Accepted, rejected, unknown and observation-unavailable remain distinct. Policy and other helper failures retain typed errors; they are not converted into ordinary offline success. Rejected profiles remain unavailable for another switch until a later reviewed reauthentication path exists. |
| Commit and launch | Commit target selection separately from launch. Persist launch intent first. Launch failure retains the target; a crash with uncertain launch never replays launch automatically. No thread/prompt/task action exists. |
| Cancel and restore | Before replacement, clean registered staging without changing live bytes. After application begins, cancellation enters reconciliation. Each restoration write has its own persisted intent/completion. Preserve the original failure and separate restoration failure. |
| Startup | Reconcile the underlying encrypted storage protocol, then require fresh switch recovery. Never treat a stale session or a terminal-looking partial resource set as authority. |

`OperationStatus` is bounded in-memory metadata, not an export serializer. It returns
installed-profile evidence and the existing separate credential/launch/Desktop/
recovery observations. A helper acceptance or an opened process can produce only
unknown/awaiting Desktop confirmation. This slice has no human-confirmation input and
cannot produce an automatically verified Desktop identity. An operation result is
its recorded outcome, not continuous monitoring of a subsequently running client.

## Conflict, retention and repair

Unexpected bytes or invalid identity enter conflict without overwriting the external
resources. When a bounded snapshot is readable, up to four distinct evidence sets
are encrypted under generated names and retained through authenticated journal
references. Evidence preserves exact bytes/absence, including malformed data;
it is never a selectable login generation. Failed/oversized snapshot collection
remains a recorded failure, not invented evidence. Diagnostics expose fixed error
categories only, not evidence, marks, account identities or paths.

Complete holds are derived from every persisted journal's current source, original
target and newest target references. Missing/partial holds or altered expected
fingerprints fail graph validation before registry selection. Public profile writes
and pruning stop while a switch is unresolved. Terminal journals without conflict
evidence can be pruned only after a verified later startup. Journals with evidence
are retained and bounded; no automatic evidence deletion or expiry is provided.
Hitting the journal/generation/evidence bound refuses further growth, not silent
loss of the newest session or an unresolved transaction.

Underlying torn control records use the existing explicit CA-03C encrypted-evidence
repair. Recovery can select only a validated reachable old/new registry. Corrupt or
foreign files, missing keys, and invalid committed evidence continue to block.
No failed restore becomes successful merely because some old ciphertext survived.

## Executable validation

Prerequisites: reviewed source, Python 3.11+, Rust 1.90.0 with rustfmt/Clippy, and
reviewed acquired dependencies. Run each command individually and stop on nonzero
exit. The first command must succeed before Cargo acquisition. Subsequent Rust
checks run offline and locked.

```console
python tools/check_dependencies.py
cargo fetch --locked
python tools/spec_index.py check
python -m unittest discover -s tests/python -v
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Generic tests retain encrypted medium and synthetic external resources independently
of the coordinator. They cover A0-to-A1 preservation then A-to-B-to-A1, all optional
presence combinations, B1 helper refresh before rejected-target restoration,
duplicate/stale requests, unknown bytes/identity, typed policy failure, helper exit,
all executable cancellation stages, late guard failures, launch uncertainty,
complete holds and corruption. Faults are injected before/after each recorded
forward/restoration storage effect, plus request/cancel/recovery-choice commits and
external stage/replace/helper/reap/cleanup/launch boundaries. Reopen discards runtime
session state and verifies reachable generations and complete holds, not just an
in-memory state-machine label. Synthetic protected state outside the declared
resource set is never part of the effects interface.

### Native storage checks with synthetic external effects

On ordinary Windows x64, from the repository root after prerequisites succeed:

```powershell
cargo test -p codex-accounts-vault-storage --locked --offline native_switch_journals_persist_reopen_and_prune_without_live_effects
if ($LASTEXITCODE -ne 0) { throw 'Native journal roundtrip failed' }
cargo test -p codex-accounts-vault-storage --locked --offline native_interrupted_journal_blocks_dispatch_and_preserves_synthetic_helper_refresh
if ($LASTEXITCODE -ne 0) { throw 'Native interrupted journal recovery failed' }
```

Each filter must run exactly one passing test, not zero filtered cases. These tests
create a fresh protected sandbox, use actual DPAPI/native storage, drop/reopen all
storage and key owners, and verify persisted outcomes or recover an unfinished
journal. External credentials, helper behavior and launch remain memory-only
synthetics. The test owns and removes only its created sandbox; it never changes
existing directory protection or uses real auth files. A hosted Windows pass is
native API evidence, not owner Windows 11 or Desktop qualification.

Share only source commit, OS/build, test names, executed case counts and pass/fail.
Do not upload test vaults, keys, encrypted evidence, actual auth files, user paths,
account identities or raw helper/native output. The ordinary two-existing-user
procedure remains in `ca-03b-crypto.md`; durability/path limits and safe collection
remain in `ca-03c-storage.md`. Missing prerequisites are NOT RUN, not a passing test.

## Remaining work

CA-04B must implement native owner/canonical-home kernel locking, writer discovery,
normal quit and actual exit/descendant proof, then bind its evidence to this private
coordinator without exposing synthetic authority. Physical power-loss and directory-
metadata durability, two-user/unavailable-store evidence, exact Q0 Desktop
publisher/runtime/home/backend/policy/resource/identity rules, and independent macOS
persistence remain open. CA-04A does not authorize live target-resource writes or
claim resistance to same-user malware, administrators, whole-vault replay or secure
physical erasure. Official-runtime protocol, login and consumer UI remain later
packets. No automatic advance to CA-05 is implied.
