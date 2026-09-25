# Parallel work and integration boundaries

Checkpoint: **2026-09-25**, after merged CA-04B / PR #9. This is a **proposed work
allocation guide**, not another roadmap, a qualification record or permission to
start every lane. [The roadmap](../implementation-plan.html) retains CA-02..CA-08
and Q0..Q5. [The selected packet](../next-pr.md) remains CA-04C. No extra lane has
started merely because it appears here.

## What can proceed together

The best immediate split is **one CA-04C implementer/integrator, one independent
test/review worker after the private contract is fixed, and owner evidence collection
on a separate test environment**. CI hardening, protocol preparation and static UI
work can also be selected as separate bounded packets. They must produce executable
code/tests or specific reviewed evidence, not empty framework scaffolds.

| Lane / status | Useful independent deliverable | Allowed write area after assignment | Join barrier / prohibited shortcut |
| --- | --- | --- | --- |
| **P1: CA-04C implementation**; next eligible packet | Private Windows target-resource operations through the existing journal, with secure staging, exact-byte/absence handling, expected-old comparisons, failure recovery and synthetic native tests | Assigned new target-adapter modules under `crates/vault-storage/src/native`; integrator-controlled coordinator wiring and packet records | No live home/auth resource discovery, production effects selector, new transaction engine or guessed companion. Q0 and platform durability still gate real integration |
| **P2: CA-04C adversarial tests/review**; may accompany P1 | Fault matrix, test cases for forward/restoration interruption, late writes, absent companions, sharing/ACL/link errors and cleanup failure; independent review of P1 | Separate assigned test modules/fixtures under the storage crate; propose shared test-harness edits to P1 instead of racing them | First agree the private adapter contract and fixture ownership. Tests must execute against the actual candidate, not a duplicate implementation or a public test API. Integrator owns module declarations and final test wiring |
| **P3: native contract/evidence**; owner-environment dependent | Safe Q0 inventory and exact-build fact collection; synthetic ordinary-user/two-user runs; precise unresolved observations | Sanitized evidence/procedure changes in a dedicated review, no live code/qualification-catalog writes | Read-only inventory does not launch Codex or change accounts. Distinct authorized test environments only; real integration trials require later implementation and separate approval. No fixture or report boolean grants authority |
| **P4: CI and supply-chain assurance**; separate selection required | One scoped improvement, such as reviewed advisory checking or measured decision-path coverage, with positive/negative tests and honest result semantics | Assigned `tools` and `tests/python` files; one CI integrator owns `.github/workflows/ci.yml` and shared gate contracts | Preserve all blocking lanes. Coordinate new named CA-04C cases with P1. New dependencies/action pins need review; no auto-upgrades, protection changes or false coverage/qualification closure |
| **P5: CA-05 protocol preparation**; separate bounded subpacket required | Extend existing framing/allowlist behavior and schema-independent hostile-input tests; later version-specific protocol tests from actually collected pinned schema | `crates/runtime/src` and separate runtime tests; source review of existing interfaces is read-only | No invented official schema, production process constructor, login, refresh, environment stripping or credential IO. Any parser dependency first needs scoped review. Operational CA-05 still depends on Q0/Q2 |
| **P6: CA-06 presentation preparation**; separate bounded subpacket required | Bundled static account/status/recovery views using clearly synthetic public metadata, with keyboard/zoom/long-label/escaping tests | Assigned static UI/test files under `app`; no edits to core/native authorities | No production Tauri IPC or consent-by-boolean, remote assets, active switch control or fake Desktop confirmation. Tauri/package dependencies and native confirmation require later reviewed integration. Preview is not a consumer application |
| **P7: CA-08 macOS preparation**; independent future lane | Read-only contract collection and review of Keychain/filesystem/process requirements; an explicitly selected native subpacket only after core interfaces and binding review are stable | Separate assigned macOS source/test/procedure files, not Windows implementations | Do not translate Windows guarantees into macOS claims. No new unsupported platform. macOS qualification can remain pending without blocking a separately qualified Windows release |

The P-n labels are delegation labels, **not new CA work IDs or approved spec
amendments**. Select one concrete outcome per extra packet and record its base,
allowed paths, tests and blocked integration. A request to plan concurrency does
not authorize P4..P7 implementation or owner-machine access.

## Dependency map

```text
Merged data + crypto + storage + CA-04A journal + CA-04B lifecycle
                         |
                         v
                CA-04C synthetic adapter <--- P2 tests after contract freeze
                         |
        exact Q0 contract + required native protection/durability evidence
                         |
                         v
             separately reviewed real-effect integration
                         |
                         v
          CA-05 operational runtime/login/refresh ownership
                         |
                         v
          CA-06 integrated UI, native consent and recovery
                         |
                         v
          CA-07 complete scenarios and installed release candidate
                         |
                 separate owner publication decision

P3 owner evidence, P4 assurance, P5 pure protocol preparation and P6 static views
can overlap appropriate implementation work, but join only at their named barrier.
P7 uses stable reviewed interfaces and its own native evidence; Windows results
never close macOS gates. Real A->B->A qualification is a later integrated trial,
not a prerequisite that must be fabricated before its implementation exists.
```

This distinguishes **parallel development** from **permission to operate**. Missing
signing choices do not stop parser tests. Missing an exact Desktop contract blocks
live integration, not synthetic implementation. Missing ordinary-user/durability
proof blocks qualification, not an honest code-review PR with those limits retained.

## Pieces that must be serialized

One integrator owns shared changes to `Cargo.toml`, crate manifests, `Cargo.lock`,
dependency review records, workflow/gate contracts, module declarations, public
interfaces, `coordinator.rs`, `journal.rs`, `journal_codec.rs`, recovery/state codecs,
and generation/retention semantics. A worker requests a scoped change with reason
and tests; it does not independently edit these from a stale checkout. Contract
changes stop dependent work until the affected workers re-read the agreed revision.

The same applies to `AGENTS.md`, the normative specification/provenance, the roadmap,
validation, next-packet pointer and bootstrap exports. Do not hand-maintain multiple
roadmaps, regenerate the spec independently, loosen source checks, renumber CA IDs,
or race separate claims of completion into shared documents. P4/P5/P7 manifest
changes must be integrated one at a time, with the exact review chain revalidated.

No two tests or sessions share a target home, vault, synthetic credential fixture,
owned helper family or real account operation. Use separate checkouts/build targets
and newly created test roots. Two-user trials deliberately share only their specified
unchanged synthetic fixture under the existing procedure; no other test writes it.
A repo branch assignment does not grant access to the owner's machine.

## Worker contract and branch protocol

Before delegation, record in the selected PR or task:

```text
Lane and bounded outcome:
Base commit and prerequisite PRs:
Owned files and explicitly forbidden shared files:
Private interface/fixture contract revision:
Required tests and expected refusal cases:
Evidence class: source | synthetic | native OS | owner exact-build
Join condition and named integrator:
Return: branch/commit, complete changed files, commands/results, blockers
```

Use one independent branch/worktree per worker, never simultaneous pushes to a
shared review branch. Truly independent packets normally branch from current main.
A test worker depending on P1 uses an explicitly recorded parent revision; its PR
must disclose that dependency, not present dependent changes as an independent
main-based result. Do not stack unnoticed work or force-update someone else's branch.

Only the assigned integrator applies reviewed contributions to the packet branch,
re-reads its current ref, publishes non-forced updates and verifies the complete
combined diff. Independent green runs do not establish that their combination is
green. Run the blocking workflow and selected native tests on the final integrated
head; inspect its actual logs and tested merge/tree. Every worker reports omissions
and limitations. No one marks a full T-scenario or platform qualified by test count.

## Suggested sequence of assignments

Select CA-04C first. Have its implementer define the narrow private operation/fixture
contract and own journal integration; then assign P2 a distinct regression surface.
Collect safe P3 evidence concurrently only when the owner selects that procedure
and has the prerequisites. Add one P4 assurance task or P5 protocol slice when
there is capacity and nonoverlapping ownership. Static P6 work is useful after the
public status/intent vocabulary is fixed, not as a substitute for the adapter.
Do not start all seven lanes by default.

Basis: [next packet](../next-pr.md), roadmap CA-04..CA-08 and decisions D-03/D-04/D-07/D-08;
specification `SPEC-PLAN`, `SPEC-PROCESSES`, `SPEC-RECOVERY`, `SPEC-PROTOCOL`, `SPEC-UI`,
`S-010`..`S-015`, `F-009`..`F-014`, `F-019`..`F-026`. The lane split and ownership
rules are this documentation proposal; the referenced product requirements are not
changed by it. [Open asks](open-asks.md) owns pending external inputs.
