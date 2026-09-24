# Next safe slice: CA-03C

Parent packet: CA-03 / Q1. CA-03A and its in-place T-07/provenance amendment are
in PR #3. CA-03B is implemented in dependent PR #5, based on CA-03A head
`595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`. Read live refs, PR review/merge state,
AGENTS.md, the working plan and current dependency records before selecting a base.
Do not merge a dependent PR into its parent feature branch. After owner merges,
reconcile/retarget with normal reviewed commits; never reset or force-push.

## Implement one protected persistence slice

Implement a local Windows x64 storage adapter and encrypted registry/generations
as one recoverable unit. Reuse CA-03A strict exact-byte/absence/generation rules
and CA-03B crypto primitives; do not invent another cipher, refresh client or
credential-management interface. Review exact pins/features before any dependency
change. The current lock and 58-package record are not permission to float versions.

Create switcher-owned storage only after validating the local, non-network,
non-known-sync root and its ancestors with native handle-based ownership/link
checks. Use protected owner/SYSTEM DACLs from creation, not post-write permission
repair. Refuse unsafe roots, broad inherited access, junctions, symlinks, hardlinks,
sharing conflicts and replacement races. No secrets or keys in an unqualified
folder, no blanket ACL changes to an arbitrary directory, no plaintext fallback.

Implement random profile/generation IDs, native-protected root-key bootstrap,
independently selected envelope context, bounded encrypted registry and immutable
generation files. Encrypt identity metadata and preserve every resource's exact
bytes and absence. Enforce the 50-profile product limit and resource/set bounds.
Define serialization overhead separately; do not silently widen credential bounds.
Do not accept a matching envelope's own untrusted context as expected context.

Durably preserve an encrypted generation before advancing its registry pointer.
Implement write-ahead recovery for key/bootstrap, registry and generation commits
in the same packet as their writes. Exclusive operation locking, same-directory
staging, flush/replacement/durability, conflicting/stale-parent refusal and startup
reconciliation need concrete native behavior. A single atomic rename does not make
a multi-file storage commit atomic. Failures must preserve the prior reachable
state or an explicit recoverable state; no silent key regeneration on corrupt/lost
key material and no destructive reset option.

Retention/deletion must protect the latest and all unresolved journal references.
Prune only at the specified clean-terminal/later-startup boundary. Active-profile
deletion stays refused, and removal never revokes provider sessions or touches the
official application's live auth/history. Persistent integration must account for
complete journal references, not trust a caller's incomplete in-memory list.

## Tests and authority boundary

Use obviously synthetic data and owned test directories only. Test initial/create/
reopen, exact-byte roundtrips, all resource presence combinations, wrong key/context,
corruption, incompatible schema, concurrent writers, stale updates, profile bounds,
retention/deletion and secret canaries. Inject failure before/after each durable
bootstrap, generation and registry write and each recovery write, including
flush/replace/permission/sharing/disk errors. Verify recovery after process restart,
not just an in-memory state transition. Native tests need the actual target OS.

CA-03B's three two-existing-user tests and unavailable-store evidence remain open
until executed and reviewed; ca-03b-crypto.md provides the exact safe procedure.
Do not invent users, lower permissions or use a fixture/Boolean as native proof.
macOS Keychain and native path/durability qualification remain independent.
Q0 publisher/runtime/home/policy/resource/identity/lifecycle facts are still pending.
Persistent code with synthetic inputs cannot qualify a Codex credential schema.

No live Codex credential capture/replacement, managed login, Desktop process
control, credential CLI/MCP/HTTP, Tauri action, network request or production
dispatch in this slice. No automatic CA-04 advancement. Full Q1 remains open until
its complete native, corruption, retention and secret-handling obligations close.

## Validation and handoff

Run the pinned Rust formatting/compilation/tests/Clippy, Python tests and source/
projection/provenance/dependency checks. Inspect final-head CI and native logs;
keep every independent check blocking. Record native gaps explicitly. Update the
existing implementation plan and validation record, not a second roadmap.
Publish one normal review PR on an observed dependency base; re-read before a
force:false ref update, verify the tree and complete changed-file set. No merge,
force push, release, permission changes or live account operations.
