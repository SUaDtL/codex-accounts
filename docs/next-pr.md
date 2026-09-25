# CA-04C: private synthetic target-resource adapter

Parent: CA-04 / Q2. **Selected next implementation packet; not started.**
Checkpoint: 2026-09-25. CA-04B is merged through [PR #9](https://github.com/SUaDtL/codex-accounts/pull/9)
at observed main `614eb1d68559b37ea4e32014214519b4d2a62e54`. Its final source head
`821ebdc26ee6081b5c74ff1da79763bef2f9ea6f` passed all six jobs in run 36108485791.
The snapshot is not a reset target or a claim that later code has passed.

Read [the handoff](maintainers/session-handoff.md), live AGENTS.md, current refs,
open PRs, reviews and checks before selecting a base. Resume a matching CA-04C PR
if present. If another prerequisite is unresolved, inspect it; do not repeat merged
code, overwrite branch movement or stack unnoticed work. This documentation refresh
does not implement CA-04C or automatically advance the product.

## Deliverable

Implement a narrow private Windows target-resource adapter over the existing
CA-04A journal and storage engine, first exercised only against freshly created
synthetic target homes. Reuse CA-04B home/process ownership; do not add another
transaction protocol or a public arbitrary-path/filesystem/process executor.

Stage the declared bounded synthetic resource set in its destination directory
with secure creation, exact bytes/explicit absence, handle-bound expected-old
comparisons and per-resource journal intent/completion. Validate resulting identity,
permissions and bytes. Preserve newest outgoing and post-helper generations and
complete retention holds. Foreign/unexpected data must enter conflict without
blind overwrite. Restoration uses the same write-ahead protocol and preserves
primary/restoration failures separately.

Trace whether existing protected-storage primitives can be reused safely; do not
copy vault naming, ACL or fixed-root assumptions into an actual Desktop home.
No live auth filename, guessed companion or guessed effective home belongs in a
synthetic resource contract. No behavior-test fixture can enable a distributable
production effects adapter. A real adapter remains blocked until the exact Q0
resource/home/policy/lifecycle contract and required filesystem durability are
established through reviewed owner evidence.

Characterize file flush, replacement and directory-metadata durability separately.
Do not rename a successful process restart test as physical-power-loss proof or
remove F-012/T-21/T-22 because metadata durability is difficult. An unsupported
primitive or unobserved case remains a named integration gate.

## Required reads

Read live AGENTS.md, the current implementation plan, validation and CI contract,
CA-03C storage/recovery, CA-04A coordinator/Effects/journal and CA-04B ownership APIs.
Read SPEC-PROCESSES, SPEC-TRANSACTION, SPEC-RECOVERY, S-003/S-005/S-008/S-009,
F-007/F-009/F-012/F-013/F-014/F-019/F-020/F-022/F-024/F-025 and the complete
T-05/T-08/T-13/T-14/T-17/T-18/T-19/T-20/T-21/T-22/T-23/T-29/T-30/T-31 scenarios.
Use tools/spec_index.py list/show/check when local execution is available.

## Tests and review

Use real Windows filesystem/lock/process APIs with controlled synthetic resources.
Prove exact-byte/absence combinations, midpoint multi-resource interruptions,
expected-parent and external-writer conflicts, sharing/ACL/link/replacement refusal,
cleanup failure, newest-generation preservation and reopen at every durable forward
and restoration boundary. Native tests own their new directories and children only.
Retain all inherited journal/refusal/native and optimized tests; no global process
kills, real account input, user creation or existing permission repair.

Any added binding, SDK feature or dependency requires exact pinned source/API review
and a scoped manifest/lock record preserving predecessors. Keep unsafe code inside
the private native boundary; keep the core/coordinator/codec restrictions.

Run existing standard-hosted source/Python/projection/provenance/dependency checks,
formatting, debug/release workspace tests, doctests, optimized compilation, Clippy,
named Windows CA-04B cases and Linux direct-IP-isolated checks. Add exact named
CA-04C native proof without allowing zero/ignored cases. All failures remain
blocking, with final-head and tested-merge evidence inspected before handoff.

Update the single roadmap, validation and safe collection procedure. Open/update
one normal PR, verify its complete file set and checks. No merge, force push/reset,
release, permission/protection change or live account operation.

## Deliberately outside this packet

No official login/refresh client, new RPC/CLI/MCP/HTTP credential surface, renderer,
installer, inference, app patching, broad history migration or automatic rotation.
Owned-runtime production construction/private stdio is later CA-05 work, not a
reason to expose the test constructor. Native confirmation and exact installation
qualification cannot be supplied by a bool. macOS remains independently gated.

Q0, ordinary two-user/unavailable-store protection, owner Windows 11/sync providers,
physical-power-loss/directory-metadata and complete product acceptance remain open.
CI follow-ups still include Windows/macOS network isolation, current advisory
automation, measured decision-path coverage, SBOM/provenance and clean unsigned
payload comparison. Main protection is an owner setting; do not change it here.

## Delegation within this packet

The primary implementer owns the private target adapter and coordinator integration.
After agreeing its private operation/fixture contract, an independent test worker
may own separately assigned adversarial test modules. The integrator alone changes
module wiring, journal/codec/recovery interfaces, manifests/reviews and named CI
inventory. No shared branch pushes or shared mutable test homes.

See [parallel work](maintainers/parallel-work.md) for the proposed allocation and
join barriers. Optional protocol/UI/CI/macOS preparation is separately selected
work, not part of this packet by implication. [Open asks](maintainers/open-asks.md)
identifies owner evidence that blocks real integration but not these synthetic tests.
