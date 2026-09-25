# CA-04D: explicit encrypted staging repair and CI partitioning

## Delivered boundary

Continue from owner-merged CA-04C / PR #11 at
`455baa2eeee614586b26d419ee961475fe1ad23b`. This slice adds an explicit private
staging-repair operation through the existing coordinator and encrypted-store
transaction. It does not enable a real Desktop adapter, introduce another credential
transaction engine, or change the normative product specification.

Normal cleanup still refuses unverified/torn staging. Repair is a separate owner
intent, not a startup heuristic, a `qualified` flag, or a retry that silently turns
conflict into success. Production construction and renderer dispatch remain absent.

## Archive before deletion

The core loads the authenticated unresolved journal, verifies the vault, and checks
confirmation, canonical-home lock, recovery binding, quiescence and write guards.
It requires saved source state, registered stages and recorded live resources. It
refuses running, terminal, committed/launched or uncaptured-helper states. Unknown
live data, wrong keys, stale storage, policy failures and missing registration cannot
be repaired by discarding a stage.

A durable cancellation/recovery intent precedes the effect. The Windows adapter
inventories only this operation's declared stage names and pins every candidate with
exclusive handles, including delete exclusion. Shape, ownership, permissions,
identity and bounded exact bytes are checked before any removal. It holds those
handles while the core commits the entire batch as encrypted, authenticated journal
evidence and verifies readback. No plaintext backup/support archive is created.

Only then does the adapter recheck each held object and dispose of that exact file.
It never deletes by an unchecked name, substitutes an arbitrary path, or writes a
live credential resource. A failed archival transaction leaves all plaintext stages
untouched. A failed/partial deletion retains the encrypted batch and an unresolved
operation. Retry re-inspects remaining objects and cannot discard changed evidence.

Successful staging repair still reports Recovery/SwitchPending. Source restoration
requires its own explicit choice and follows the existing write-ahead resource
protocol. The newest source/target generations and original failure are retained;
archived staging bytes are never selected as credentials or Desktop identity.

## Explicit record evolution

`CAREG003` adds an evidence-kind byte distinguishing Live from Staging evidence.
Versions 1 and 2 still decode and re-encode unchanged; merely opening old storage
never migrates it. Existing live evidence retains its original meaning. The first
staging-evidence commit opts into version 3, and subsequent commits never downgrade.
Older readers reject version 3 rather than misinterpreting the extension.

Evidence kind and references are authenticated within the existing encrypted
registry; resource envelopes bind operation, evidence ID and slot. Envelope/key and
credential-generation formats are unchanged. Staging evidence must be present and
belong to the operation's recorded staging/restoration masks. Unknown kind values,
unregistered slots, truncation, trailing bytes and silent format downgrade refuse.
There is no new crypto implementation, dependency or native binding.

## Evidence and remaining limits

Portable tests exercise exact bytes/provenance, old/new codec behavior, guard/key/live
state refusal, every encrypted-commit failure, idempotent retry and bounded capacity.
Windows tests exercise exclusive handles, sharing/link/unregistered-file refusal,
archive failure, explicit restoration and five actual repair-process interruption
boundaries. All inherited seventeen forward and seventeen restoration interruption
points remain, including their original automatic torn-stage refusal assertions;
the formerly blocked cases additionally exercise explicit repair and recovery.

These are code and synthetic/native OS behaviors, not complete acceptance-scenario
closure. Directory-metadata and physical-power-loss durability, exact official
resource/home/policy/process qualification, ordinary two-user/unavailable-store
protection and independent macOS integration remain open. Uncooperative same-user
namespace races are still not an atomic compare-and-swap guarantee. This operation
cannot make an unsupported installation safe or reverse provider token invalidation.

## Reproduction and safe reporting

Use the reviewed checkout, Python 3.11+, pinned Rust 1.90.0 and a local Windows x64
synthetic test environment. Run dependency review before `cargo fetch --locked`.
Run each validation command separately and check its exit code:

```powershell
python tools/spec_index.py check
python tools/ci_contract.py
python -m unittest discover -s tests/python -v
cargo fmt --all -- --check
python tools/ci_workspace_tests.py debug
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
python tools/ci_workspace_tests.py release
python tools/run_native_tests.py
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

The workspace runner verifies its compiled partition; its success alone does not
include the separate native cases. The native runner requires all 27 named behaviors
per profile: 54 case records plus two profile/timing summaries. Every record explicitly
states that Desktop qualification is not established. The owner-run two-user helpers
and the platform-specific Python FIFO fixture remain separate, explicit omissions on
Windows, never included as passes. The Linux source lane runs the full Python suite.

No real account or authentication file is needed. Share only source/build IDs,
case/profile/outcome, sanitized counts and timing. Do not export raw helper output,
local paths, account identifiers, key material, plaintext stages or private logs.
The existing parent-owned fixture cleanup remains bounded to its own test directories.
