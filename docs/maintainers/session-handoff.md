> CA-04C implementation update: resume `feat/ca-04c-synthetic-target-resources`
> and inspect its live PR/checks. The dated snapshot below predates that implementation.
> `docs/next-pr.md` and `docs/ca-04c-target-resources.md` describe the current review
> and remaining torn-stage/namespace/durability gates. Do not recreate merged work.

# Session handoff after CA-04B

Checkpoint: **2026-09-25**. This file replaces conversation reconstruction, not
[the roadmap](../implementation-plan.html) or [the selected packet](../next-pr.md).
Re-read live state before editing; a recorded hash is an evidence anchor, not a reset target.

## Verified product checkpoint

| Item | Recorded value |
| --- | --- |
| Repository | `SUaDtL/codex-accounts` |
| Last product PR | [#9, CA-04B](https://github.com/SUaDtL/codex-accounts/pull/9), merged 2026-09-25 at 07:47:55 UTC |
| Observed main / owner merge | `614eb1d68559b37ea4e32014214519b4d2a62e54` |
| Final CA-04B source head | `821ebdc26ee6081b5c74ff1da79763bef2f9ea6f` |
| Final PR test merge / tree | `65147ae9dfdb5752b3a24371d4358d9e836c2ed2` / `85b0bb90d2166a17814068c184b1c07726466f32` |
| Source relationship | The owner merge and final tested candidate have that same tree. The test merge was not an assistant merge. |
| Final CA-04B CI | [Run 36108485791](https://github.com/SUaDtL/codex-accounts/actions/runs/36108485791): all six jobs completed successfully |
| Next implementation | CA-04C, private synthetic target-resource adapter; not started at this checkpoint |
| Product authority | Zero qualified Desktop records; capture/login/live switch and Desktop control remain disabled |

The bootstrap refresh is documentation-only. Its review PR/head/checks are separate
from this product checkpoint. Do not inherit the above green run for a newer commit.
No implementation worker or additional lane is claimed to be running.

## Reuse this code; do not rebuild the foundations

| Area and entry files | Existing behavior | Missing boundary |
| --- | --- | --- |
| `crates/discovery/src/lib.rs`, `windows_reader.rs`, `native.rs` | Read-only Windows package inventory, bounded reads, conservative typed observations and redaction | Exact official publisher/runtime/home/backend/policy contract is not established |
| `crates/core/src/lib.rs`, `compatibility.rs` | Refusal and separate installation/acceptance/launch/identity/recovery outcomes | No caller or fixture can qualify an installation |
| `crates/vault/src`, `crates/vault-crypto/src` | Exact bytes, explicit absence, composite identity, generations, authenticated envelopes and Windows DPAPI | Native two-user/unavailable-store evidence and macOS key protection remain open |
| `crates/vault-storage/src/engine.rs`, `records.rs`, `control_repair.rs`, `native.rs` | Encrypted persistence, immutable generations, expected-parent commits, repair and deferred retention | Target Desktop resources are not the vault's fixed-root storage contract |
| `crates/vault-storage/src/coordinator.rs`, `journal.rs`, `journal_codec.rs`, `switch_tests.rs` | Private CA-04A coordinator, durable holds, per-resource intent, newest/post-helper capture, cancellation and transactional restoration | Production effects adapter unavailable; synthetic effects are test-only |
| `crates/vault-storage/src/native/lifecycle/{home,process,helper}.rs`, `lifecycle_model.rs` | CA-04B home mutex, retained process identity, bounded descendants/normal quit and owned-family exit proof | Home association and full writer set unqualified; OwnedFamily construction is test-only; crash-recoverable official-helper ownership is later work |
| `crates/runtime/src/lib.rs` | Bounded framing and method allowlist | Structured version-bound RPC, official launch, private pipes and managed login are not implemented |
| `app/README.md` | Intended shell boundary | No Tauri consumer application yet |

Read [CA-03C storage](../ca-03c-storage.md), [CA-04A journal](../ca-04a-journal.md),
[CA-04B lifecycle](../ca-04b-lifecycle.md) and [native review](../ca-04b-native-review.md)
for the exact contracts. Symbol/path names in this map are retrieval starting points,
not frozen interfaces. Verify their full definitions and any nested AGENTS.md first.

## Evidence available and still missing

Final CA-04B source logs record **114 Python tests**. Windows logs record **176
workspace passes per debug/release profile**, including 87 storage/lifecycle cases,
three doctests and **22 named executions of eleven behaviors**. Three inherited
two-user helpers were ignored and are **NOT RUN**, not passing evidence. Workspace
counts include child-test harness entry points. Windows ran on **Server 2025 x64
build 26100**, not a qualified ordinary-user Windows 11 Desktop installation.

The separate Ubuntu namespace job compiled from a fresh target and passed 129
workspace tests per profile, doctests and Clippy with direct IP networking blocked.
It is not a filesystem/IPC sandbox or Windows/macOS isolation. macOS CI establishes
portable-library behavior, not Keychain or Desktop qualification. Full Q0/Q1/Q2,
all complete T-01..T-34 scenarios and consumer readiness remain open. See
[open asks](open-asks.md) for narrowly scoped missing inputs.

Keep the bounded `CO_E_SERVER_STOPPING` activation handling and stack-owned COM
apartment correction in `native/sync.rs`. No retry-until-green policy, service
restart, cached empty inventory or permission relaxation is an acceptable substitute.
Known descendant/broker limitations are explicit exclusions, not permission to
claim shared-home quiescence. Signal receipt is never exit proof.

## Startup and implementation procedure

Read the live repository identity/default branch, main ref, open PRs and relevant
review threads. Resolve the chosen packet. Read AGENTS.md, the packet, relevant
specification symbols, dependency policy, workflow and full affected definitions.
Check local tool availability anew; prior tool outages or recovered toolchains are
not permanent environment facts. No installed Desktop is needed for synthetic tests.

Start a normal branch from observed current main unless resuming a matching PR.
Unreviewed stacking must not be hidden. Do not reuse PR #9's merged review branch
for unrelated work. If the branch moves, inspect the intervening commits rather
than overwriting them. One branch integrator serializes ref updates and common-file
changes; [parallel work](parallel-work.md) describes delegable slices.

## Validation commands

Prerequisites: Python 3.11+, pinned Rust 1.90.0 with rustfmt/Clippy and reviewed
locked dependencies. Read the live workflow because it may have changed. Run these
individually and inspect each exit status; a pasted PowerShell group does not by
itself stop at a native command failure. Review must pass before acquisition.

```console
python tools/check_dependencies.py
cargo fetch --locked
python tools/spec_index.py check
python tools/ci_contract.py
python -m unittest discover -s tests/python -v
cargo fmt --all -- --check
cargo test --workspace --all-targets --no-fail-fast --locked --offline
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
cargo test --workspace --all-targets --release --no-fail-fast --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

On a supported Windows x64 test host, also run `python tools/run_native_tests.py`
using the prerequisites and stop-on-error procedure in the lifecycle document.
It currently expects eleven named behaviors in both profiles. Add separately named
CA-04C proof when implementing that packet; do not drop inherited cases. The Linux
network-isolation command belongs in its documented hosted job, not a blanket
instruction to change a workstation's networking or privileges.

Final PR evidence must identify source head, tested merge/tree, run/job IDs and
completed outcomes. Inspect logs, not just badges. A timeout, omitted platform or
unavailable compiler is NOT RUN/incomplete. Independent failures stay visible.
Do not delete assertions, loosen unsafe restrictions or add continue-on-error.

## Closeout and carry-forward

Review scope, credential secrecy, newest generations, forward/recovery durability,
helper exit and separated outcomes. Preserve the base tree and file modes, commit
with the observed parent, re-read the review branch and advance with `force:false`.
Verify the new ref and complete changed-file set before opening/updating the PR.
The [connector index](GITHUB_CONNECTOR_INDEX.md) gives the named action sequence.

Update current-state sections of the roadmap, validation, next packet and this
handoff. Record outstanding asks once, with source references; do not append a chat
history. Exports are regenerated snapshots, never a second editable plan. Evidence
must bind its own exact commit; an artifact's retention expiry is not proof it never
ran, and absence of an artifact is not permission to invent one. Leave reviews,
merge, publication, permissions and live account operations to separate authorization.
