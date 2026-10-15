# Codex Accounts

Manual, owner-controlled account handoff for the official Codex Desktop application.

**Development only. No Desktop build is qualified. Capture, login, live account
switching and Desktop process control are not enabled. Persistent storage and
native lifecycle primitives are development libraries, not an account manager.**

## Current implementation

PRs #14 through #22 are merged. CA-05F/G/H provide private schema-pinned
handshake, account-read, managed-login/cancellation and rate-limit codecs over
synthetic endpoints. CA-06B adds a Windows-only Tauri developer preview of the
existing disabled pages, with no application commands or native adapter. See
[the preview boundary](docs/ca-06b-static-preview.md). Historical review-stack
references below do not reopen merged PRs.
CA-02B extends read-only discovery with a bounded cached primary signer digest and
distinct unavailable/cleanup states. Its [exact-build observation](docs/ca-02b-cached-primary-signer.md)
does not establish publisher approval or qualify an installation.

CA-02 provides read-only Windows x64 registered-package discovery, bounded native
executable/configuration observations and conservative compatibility evaluation.
Unknown publisher, runtime, effective home/backend or policy remains unknown.
Finding `auth.json` does not establish file storage. The qualification catalog is
empty and the source-level mutation gate refuses every backend.

CA-03A supplies exact-byte/absence preservation, bounded JSON, composite identity
and generation rules. CA-03B adds authenticated envelopes, separated keys, OS
randomness, owned-buffer zeroization and current-user Windows DPAPI. CA-03C adds
protected encrypted persistence, immutable generations, write-ahead commits,
startup reconciliation, deferred retention and explicit encrypted-evidence repair.
See [data](docs/ca-03-vault-data.md), [crypto](docs/ca-03b-crypto.md) and
[storage](docs/ca-03c-storage.md) contracts.

CA-04A provides the private storage-backed switch coordinator: complete generation
holds, newest-source preservation, post-helper capture, per-resource intent,
cancellation, conflict preservation and journaled restoration. Its external effects
remain synthetic-only; no public switch entry point exists. See
[the journal contract](docs/ca-04a-journal.md).

CA-04B, merged through PR #9, adds private Windows owner/canonical-home kernel locks, retained
process start/owner/image observations, bounded descendant tracking, normal-window
quit and independently proven owned-job exit. Native tests create only fresh
synthetic directories, hidden windows and owned children. These primitives do not
identify a qualified shared-home writer set, grant consent or connect to live
credential replacement. See [lifecycle and native checks](docs/ca-04b-lifecycle.md).

CA-04C/D add synthetic target-resource operations and encrypted staging recovery.
CA-05A/B provide bounded framing and strict JSON object validation. Draft
[PR #14](https://github.com/SUaDtL/codex-accounts/pull/14) adds session sequencing,
login-ID correlation and cancellation; stacked draft
[PR #15](https://github.com/SUaDtL/codex-accounts/pull/15) adds the private synthetic
transport pump. Its owned in-memory channels exercise real scheduling, backpressure
and EOF handling. They are not native pipes or an official runtime adapter.
See [the transport boundary](docs/ca-05e-transport-driver.md).

All full Q0/Q1/Q2 and complete release scenarios remain open. Owner Windows 11,
ordinary-user isolation, unavailable stores, sync-provider environments, actual
Desktop contracts and physical-power-loss/directory-metadata qualification remain
separate. macOS native persistence/lifecycle is not implemented. Ubuntu CI tests
portable code and build isolation; it does not add Linux/WSL product support.

The direction remains Rust core plus a thin Tauri 2 shell. No inference proxy,
credential CLI/MCP/HTTP interface, automatic rotation, host patch, self-updater or
broad history migration belongs in this product.

## Start here

New sessions: read [CODEX_ACCOUNTS_START_HERE.md](CODEX_ACCOUNTS_START_HERE.md),
[the handoff](docs/maintainers/session-handoff.md) and [parallel-work boundaries](docs/maintainers/parallel-work.md).
The [open-asks register](docs/maintainers/open-asks.md) separates owner evidence from
safe implementation work. Resume CA-02B, refreshing live refs/open PRs before
execution. PRs #14 through #22 are merged. The original CA-02
Project routing is superseded.

The normative [specification](docs/local-codex-switcher-spec.html) owns behavior
through its canonical `artifact-model`; the visible projection is checked against
it. The [implementation plan](docs/implementation-plan.html) is the single working
roadmap. The [next packet](docs/next-pr.md) describes the subsequent bounded work,
not permission to execute it. [Validation](docs/validation.md) separates source,
hosted native API execution and real Desktop qualification.

The scoped T-07 clarification and v2 provenance migration were adopted through PR
#3. [The amendment](docs/spec-amendments/ca-02-t07.md) preserves reconstruction of
original specification bytes. Unchanged research/index inputs retain their hashes.
A merge or document check never qualifies an installation. The
[comparison](docs/codex-switchers-comparison.html) is supporting research only.

## Build and validate

Prerequisites: Python 3.11+, pinned Rust 1.90.0, rustfmt and Clippy. Run commands
individually, inspect each exit status, and stop acquisition if review fails.

```console
python tools/check_dependencies.py
cargo fetch --locked
python tools/spec_index.py check
python tools/ci_contract.py
python -m unittest discover -s tests/python -v
cargo fmt --all -- --check
python tools/ci_workspace_tests.py debug
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
python tools/ci_workspace_tests.py release
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Standard GitHub-hosted Ubuntu, Windows and macOS jobs retain these blocking checks.
On Windows, the workspace verifier proves the compiled inventory partition before
running its complement; `tools/run_native_tests.py` runs all 27 named native
behaviors independently in debug and release. Together they cover the inventory
without counting child harnesses as scenarios; skipped/zero native tests fail.
The native verifier is Windows-only. Other platforms run the portable workspace.
A separate standard Ubuntu job recompiles and runs debug/release tests, doctests
and Clippy in a new direct-IP-isolated namespace after reviewed acquisition.
Cargo `--offline` alone is not OS network isolation. The isolated lane does not
sandbox filesystem/Unix-domain IPC or establish Windows/macOS network isolation.

[The CI review](docs/ci-review.md) and [scoped native/Actions review](docs/ca-04b-native-review.md)
record the evidence boundary and remaining work. CI retains only exact committed
public source and a digest record, not test vaults, raw helper output or dependencies.
No real authentication fixture belongs in Git, logs or support reports.

## Read-only discovery

Build `codex-accounts-discovery` and run it without arguments to inventory current-user
registered candidates. Follow the [Q0 procedure](docs/q0-qualification.md) for safe
selection, expected refusal and redaction. `--show-local-paths` is terminal-only and
must not be shared. The legacy `tools/q0_inventory.py` remains a synthetic developer
aid, not an effective Desktop configuration resolver. Neither inventory executes a
nominated runtime, reads live auth, writes the nominated home or qualifies a build.

## Source layout and authority

| Path | Purpose |
| --- | --- |
| `crates/core` | Refusal, compatibility observations and distinct outcomes |
| `crates/platform` | Platform classification; no credential writer |
| `crates/discovery` | Read-only inventory with a narrow native boundary |
| `crates/runtime` | Framing, strict JSON, sealed sequencing and private synthetic transport |
| `crates/vault` | Exact-byte in-memory resource/identity/generation rules |
| `crates/vault-crypto` | Authenticated envelopes and Windows root protection |
| `crates/vault-storage` | Protected persistence, journal and private native lifecycle |
| `app` | Canonical disabled synthetic static previews |
| `crates/shell-preparation` | Windows-only static Tauri developer shell; no backend |
| `compatibility` | Empty catalog; no boolean enables production authority |
| `docs` / `tools` | Specification, roadmap, scoped reviews and developer checks |

Changing accounts does not isolate local history or workspaces. Real Desktop
qualification, ordinary-user behavior, packaging, signing, reproducibility and the
license decision remain release work. Crates retain `publish = false`.
