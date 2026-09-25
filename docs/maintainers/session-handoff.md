# Session handoff: CA-04C merged, independent preparation in review

Checkpoint: **2026-09-25**. Refresh live refs, PRs and completed checks before editing.
This is continuation routing, not a second roadmap or a release claim.

## Current checkpoint

| Item | Recorded state |
| --- | --- |
| Repository | `SUaDtL/codex-accounts` |
| Owner-merged main at task start | PR #11 / CA-04C, `455baa2eeee614586b26d419ee961475fe1ad23b` |
| Main source tree | `25e5f0ebb19f0b9b793ae22ddb7111698e680a70` |
| Selected review | PR #12, `feat/ca-05-06-macos-preparation`; read its actual current head |
| Selected scope | CA-05A protocol preparation, CA-06A static presentation and independent macOS preparation |
| CA-04D | Separate unfinished staging-repair work; its existing checkout/branch was preserved, not imported or overwritten |
| Product authority | No qualified installation; live capture/login/switch/refresh and Desktop control remain disabled |

No PR was open when this preparation began, so PR #12 was created from merged main.
Do not reopen or reset merged PR #11, silently import another dirty checkout, or
claim that these independent preparations complete CA-04D or operational Q3/Q4.
[Preparation handoff](preparation-handoff.md) records this review's exact source
surface, reproduction and verification limits. [Next packet](../next-pr.md) and
[the single roadmap](../implementation-plan.html) govern selection.

## Reuse existing foundations

| Area | Existing implementation and remaining boundary |
| --- | --- |
| `crates/discovery` | Read-only Windows package inventory and conservative sanitized observations; exact publisher/runtime/home/policy qualification remains open. |
| `crates/core`, `crates/platform` | Always-refuse guards and separate credential, launch, identity and recovery observations; no fixture can qualify an installation. |
| `crates/vault`, `crates/vault-crypto` | Exact bytes/absence, composite identity, generations, authenticated envelopes and Windows DPAPI. macOS key protection and owner/two-user qualification remain open. |
| `crates/vault-storage/src/engine.rs` and repair modules | Encrypted persistence, immutable generations, expected-parent publication, retention and encrypted recovery. This is not a live Desktop resource adapter. |
| `coordinator.rs`, `journal.rs`, `journal_codec.rs`, `switch_tests.rs` | One write-ahead forward/restoration protocol and complete generation holds. Production Effects remain unavailable; do not expose test constructors. |
| `native/lifecycle` and `lifecycle_model.rs` | Private Windows home/process/owned-family behavior; incomplete writer association and namespace evidence still block production mutation. |
| `native/lifecycle/target.rs` and CA-04C tests | Synthetic target-resource replacement and authenticated cleanup. Torn/unverified staging repair and native durability remain explicit gates. Verify actual module paths in current source. |
| `crates/runtime` | Raw bounded transport, terminal EOF and outbound method-shape classification, not schema validation or official-runtime execution. |
| `app/` | Three synthetic static views and source accessibility/escaping tests, not a Tauri application or consent surface. |
| `docs/macos`, `tools/macos_preparation.py` | Documentary native packet and seven-file source baseline; no native implementation or qualification. |

Read the CA-03C storage, CA-04A journal, CA-04B lifecycle/native review and CA-04C
resource/native review documents for detailed inherited contracts. Historical
exact-source results remain in [validation](../validation.md) and their PRs, not
copied into current-head proof. Keep the reviewed bounded activation behavior and
stack-owned COM apartment; do not retry tests until green, change services or relax
permissions. Signal receipt and process appearance are not exit/identity proof.

## Validation and safe continuation

Read live AGENTS.md and the relevant canonical specification symbols before changes.
Use `tools/spec_index.py list/show/check`; preserve all 43 requirements, 34 complete
acceptance scenarios and the empty catalog. Source hashes are evidence anchors, not
reset targets or authority. Review dependencies before acquisition; no new packages
are required by the preparation.

Run the complete command set in [preparation-handoff.md](preparation-handoff.md),
including existing source/projection/provenance/dependency/workflow checks and
Rust formatting, debug/release workspace, doctests, optimized build and Clippy.
The source suite has 143 tests with this preparation. Exact-name Windows proof
retains all eleven CA-04B and twelve CA-04C behaviors in both profiles. Inherited
two-user helpers remain NOT RUN unless separately executed under the documented
procedure. Generic macOS builds are not macOS native-adapter tests.

Inspect final-head run/job logs and the tested merge/tree, not a prior badge. The
Linux direct-IP-isolated job remains mandatory and is not a filesystem/IPC sandbox
or Windows/macOS isolation. Browser navigation was blocked before preview load;
layout/keyboard/200% zoom and screen-reader checks are NOT RUN. Native macOS,
ordinary-user owner-platform and physical-power-loss qualification are NOT RUN.

One integrator serializes review-branch pushes and owns shared interfaces, module
wiring, manifests and planning. Independently reviewable protocol, presentation and
macOS documentary work cannot bypass operational dependencies. Resolve CA-04D and
review/reissue the shared interface baseline before macOS native coding. Never
update hashes merely to silence drift. [Parallel work](parallel-work.md) describes
eligible lanes; [open asks](open-asks.md) scopes missing owner evidence without
requesting real authentication material.

Re-read a branch before a non-force update and inspect any intervening commit.
Verify the resulting ref and complete changed-file set. Use the connector index for
supported publication actions. Update the single roadmap, validation and routing
with current facts rather than appending conversation history. No merge, reset,
force push, release, permissions change or live account operation is authorized.
