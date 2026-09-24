# CA-03C: protected persistence and storage recovery

## Implemented boundary

`crates/vault-storage` joins the CA-03A data rules and CA-03B cryptography to a
Windows x64 persistence adapter. It stores an encrypted registry, identity metadata,
immutable resource/generation records and a write-ahead control record. It accepts
owned structural memory, not a filename from which to capture a Codex account.
No Desktop binding, auth-file read/replacement, login, process control, UI, credential
CLI/MCP/HTTP, network client or production qualification is introduced.

The public constructor resolves the native LocalAppData known folder and appends
`CodexAccounts-Vault-v1`. It offers no arbitrary-root, plaintext-key or custom-filesystem
constructor. Unknown platforms return `UnsupportedPlatform`. Private tests use only
new random synthetic directories. A safe native storage object is not a qualified
Codex installation, verified identity, or proof that a saved session is valid.

## Records, context and bounds

The existing current-user DPAPI root is opened before decrypting any registry.
Creating a vault refuses an existing root. Missing/corrupt wrapped keys over existing
data are not replaced. Only a valid original staged wrapped key in an otherwise empty
bootstrap is eligible for publication. A torn first key stays unavailable; no guessed
key, rollback to a different key, key migration or destructive reset is implemented.

The root-bound `state.bin` control context is fixed by code and the native-protected
root identifier. Its authenticated contents select registry/object contexts and
ciphertext digests. The registry selects profile/generation/resource records; their
own untrusted headers never supply the expected context. Labels, trust domains and
issuer/subject/workspace identity are encrypted. Generated UUIDv4 names contain no
email, label, account ID or user path. Timestamps are local observations, not provider
authority. The schema rejects truncation, trailing bytes, invalid tags and inconsistent
generation/reference graphs before use. Unknown schemas have no fallback migration.

Credential limits remain 1 MiB per resource, 4 MiB per complete set and 64 JSON
container levels, with exact original bytes and explicit absence. The registry is
limited to 50 profiles. Conservative implementation bounds are 128 retained generations
for the whole vault, 16 resource slots, 512 KiB per encoded metadata record and 2,304
object references. Envelope overhead is 66 bytes, not additional credential capacity.
A limit refusal does not evict unresolved recovery references or silently increase a
bound. Resource digests inside authenticated records cover randomized ciphertext,
not plaintext credential fingerprints published to diagnostics.

## Commit and recovery protocol

A storage update validates the expected parent and composite identity, encrypts the
new resource/generation records, and records their intended names, lengths and digests
in a durable encrypted intent. Metadata ciphertext is retained inline for deterministic
reconstruction; credential bytes are not reconstructed from parsed fields. Objects
are created without overwrite. The selected registry changes only after all referenced
objects and resource semantics verify. The prior registry and generations remain
reachable until the new terminal record is committed.

Each state update binds a monotonically advancing sequence and the previous control
ciphertext digest. A failed stage/flush/rename poisons that in-memory writer; reopening
checks disk rather than assuming the failed operation had no effect. A complete valid
staged control must match its predecessor and pass inventory/reachability checks before
publication. Old authenticated stages are not permission to roll back current state.
Whole-vault replay by an attacker able to replace all protected state is not excluded
by this local sequence scheme or by AEAD; no anti-rollback hardware is claimed.

| Observed state | Implemented result |
| --- | --- |
| Clean terminal state | Read/update permitted within this storage library; no live account authority. |
| Valid pending intent | `CommitPending`; explicit reconciliation verifies/publishes intended objects, then commits the registry. |
| Missing/torn unpublished resource | Current state remains reachable. Explicit restoration preserves damaged stage evidence under encryption before removing only the compared unpublished file. |
| Torn or unauthenticatable control stage | `ControlRepairRequired`; normal operations are blocked. Opening does not delete or adopt the stage. |
| Explicit control repair | Verify committed registry, snapshot and complete inventory; encrypt and verify an evidence copy; compare again; remove only the original unpublished stage. Preserve the last authentic selection. |
| Interrupted first control with a surviving key | Only an otherwise empty, recognized bootstrap can initialize an empty registry with that same key. Unknown payloads block it. |
| Changed committed data, wrong key/context, stale stage or foreign file | Refuse without replacing it. Evidence files are not accepted as operational state. |
| Interrupted cleanup | Resume the recorded cleanup after checking reachable and held references; never prune a referenced generation. |

Evidence names are reserved `recovery-<random UUID>.bin` and their stage names, with
32 files and 2 MiB per file as conservative limits. Evidence is encrypted before disk
writes, never selected as a registry or replayed into a credential resource, never
exported, and never automatically expired or pruned. A crash during archival preserves
the original plus any partial encrypted evidence; retry uses a fresh name and nonce.
Capacity exhaustion refuses further repair while preserving original data. Manual
forensic export/evidence management is not implemented; there is no delete-evidence
shortcut that grants mutation authority.

Retention protects latest generations and complete persisted operation holds. Old
unreferenced generations are pruned only after a clean terminal state and a later
verified startup. Active-profile removal is refused; unknown active association also
refuses. This slice deliberately has no public setter for active state or partial hold
lists. Later coordinator integration must establish and durably update those facts as
one operation; the internal model's tests are not production authority for that step.

## Native protection and unsafe review

Application-authored native effects are private to `src/native.rs` and its child
modules. Adjacent codec/engine/record/recovery modules forbid unsafe code. Existing
core/platform/runtime restrictions are unchanged. SDK types come from the exact
reviewed windows/windows-sys/windows-collections pins; native output owners release
handles, allocated descriptors and known-folder strings on all exits.

Before object access, an explicit drive-root query requires a fixed local drive.
The containing volume must be NTFS. UNC/device/relative/traversal/alternate-stream and
known-sync names are refused. Ancestors are opened and retained without delete sharing,
checked for reparse points and compared by volume/file identity. Registered sync-root
paths also undergo ancestor traversal; the query cannot follow an unchecked junction
or mapped remote drive. Read denial, incomplete sync inventory and unexpected CloudFilter
results are refusals, not evidence of absence. Successfully enumerated registered
legacy/modern roots are compared by handle identity; no provider registration changes.

Ancestor ownership is restricted to the current user, SYSTEM, Administrators or the
exact Windows servicing identity, with writable grants checked. The direct parent must
be user-owned. The vault root and every leaf require current-user ownership and a
protected DACL restricted to that owner and SYSTEM. Permissions are present at creation;
existing arbitrary directories/files are never repaired. A kernel-held exclusive lock
coordinates vault instances. Hardlinks, unsafe DACLs, non-file leaves, changed identities
and sharing conflicts are refused. This lock is not yet the later target-home lock and
cannot constrain arbitrary same-user malicious code or administrator actions.

Sync inventory uses an uncached registered activation factory. Its interfaces are
released within each query. One balanced thread-local MTA initialization is retained
until that storage thread exits, preventing per-operation apartment teardown. An
incompatible pre-existing apartment is refused; later GUI integration must use an
appropriate owned core thread, not reconfigure a UI thread. No generated heuristic
DLL-search fallback or process-static activation factory is used in this adapter.

Writes create a secure same-directory stage, use write-through handles and explicit
file flushes, validate content/security, rename through the held source handle, flush
again and recheck resulting identity/content/security. Deletion compares the held
object and uses native file disposition. There is no truncate-in-place update, remote
fallback, cross-volume move, plaintext backup or administrator volume flush.

**Durability qualification remains open.** Native tests prove the exercised file API
and process-restart behavior. They do not prove hardware power-loss ordering, storage
controller persistence or directory-metadata behavior on every Windows 11 filesystem.
The implementation currently uses file flushes and NTFS handle operations; it does
not claim a separately qualified directory-flush primitive. F-012/T-21/T-22 and full
Q1 require that evidence before live integration. Do not turn this limitation into an
assertion that process exit is a physical power-loss test or a request for elevation.

## Executable checks and safe evidence

Prerequisites: Python 3.11+, Rust 1.90.0 with rustfmt/Clippy, reviewed dependencies,
and a Windows x64 user session for native execution. No Codex installation, account,
secret file, new OS user, permission change to existing files or elevated session is
required by the ordinary suite. The test fixture creates its own protected parent
rather than relaxing ownership rules for administrator-owned runner directories.

Run from the reviewed repository root in PowerShell:

```powershell
python tools/check_dependencies.py
if ($LASTEXITCODE -ne 0) { throw 'Dependency review failed' }
cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw 'Dependency acquisition failed' }
cargo test -p codex-accounts-vault-storage --all-targets --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'Storage tests failed' }
```

The suite uses obvious synthetic canaries and newly created directories only. It
covers exact bytes/absence, stale parents, identity/profile bounds, persistent holds,
corruption/unknown files, cleanup, torn control and encrypted repair evidence. Fault
injection covers writes in the forward, restoration, control-repair and cleanup paths.
Each injected append failure must reach a verified old or new generation after
reopening; mere preservation of inaccessible bytes is not a passing recovery oracle.
Native tests cover secure creation, actual junctions and hardlinks, DACL refusal,
sharing, handle-based replacement/deletion, repeated WinRT queries, synthetic DPAPI
bootstrap and child-process restart at 20 before/after commit-effect boundaries.
The child is the same test binary, not a shell, user application or Codex runtime.

Share only source commit, OS/build, compiler, command, test result, whether the session
was ordinary/elevated and any fixed error category. Do not attach test directories,
wrapped keys, raw paths, identities or authentication files. Test cleanup removes only
the newly created synthetic tree; fixture creation never overwrites an existing path.
An unavailable native environment is NOT RUN, not passed. CI's hosted Windows Server
result is not owner Windows 11 evidence.

Two-existing-user and unavailable-store qualification remains separate; follow the
synthetic controls in [ca-03b-crypto.md](ca-03b-crypto.md), never a real auth file. macOS
Keychain/persistence, representative sync-provider environments, physical-power-loss
qualification, and the exact Q0 Desktop contract remain pending. Full T-11 through
T-18, T-21/T-22 and T-31 are not closed by these library tests.

## Next boundary and references

CA-04A may add the storage-backed switch journal/coordinator under a separately
requested bounded packet. It must bind durable operation references and independently
observed active state without exposing a caller-controlled bypass. Native quiescence,
live resource writes, official runtime/login, UI and release remain separate gates.

Primary implementation references, in addition to pinned SDK sources and the inherited
crypto review:
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
- https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers
- https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-roinitialize
- https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-rouninitialize
- https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-rogetactivationfactory
- https://learn.microsoft.com/en-us/uwp/api/windows.storage.provider.storageprovidersyncrootmanager.getcurrentsyncroots
- https://www.rfc-editor.org/rfc/rfc9562.html#section-5.4
