# Codex Accounts

Manual, owner-controlled account handoff for the official Codex Desktop application.

**Development only. No Desktop build is qualified. Capture, login, live account
switching, persistent credential storage and Desktop process control are not enabled.**

## Current implementation

CA-02 provides a concrete read-only Windows x64 registered-package reader,
bounded executable/configuration observations, conservative compatibility results
and native/synthetic tests. The source-level mutation gates always refuse and the
compatibility catalog remains empty. Unknown publisher, runtime role, effective
configuration or policy remains unknown; finding `auth.json` cannot establish a
file backend. macOS qualification remains independent; Linux/WSL are not supported
product targets.

CA-03A adds a library-only in-memory data boundary: exact resource bytes and explicit
absence, strict bounded JSON, composite identity and immutable-generation metadata.
It does not persist, capture or install credentials. See
[the data-slice contract](docs/ca-03-vault-data.md) for its structural limits.

CA-03B adds authenticated XChaCha20-Poly1305 envelopes, HKDF-SHA256-separated keys,
OS-random roots and nonces, and current-user Windows x64 DPAPI root-key protection.
Application-owned resource, identity and decoded-key buffers use reviewed
zeroization. This is a library, not a persistent account vault or an authorized
capture/switch path. Two-user native evidence and independent macOS Keychain remain
pending. See [the crypto contract and safe native checks](docs/ca-03b-crypto.md).

PR #5 is a dependent review on PR #3. Do not merge it into the dependency branch;
after the owner merges #3, reconcile and retarget through normal reviewed
operations. Next implementation: CA-03C protected persistence and registry commits.

The product direction is Rust core plus a thin Tauri 2 shell. No inference proxy,
credential-management CLI/MCP/HTTP interface, background rotation, host patch,
self-updater or broad history migration is part of the product.

## Start here

The normative [specification](docs/local-codex-switcher-spec.html) owns product
behavior through its embedded `artifact-model`; its visible text is checked against
that model. The [implementation plan](docs/implementation-plan.html) is the single
working roadmap, adopted from the Project packet. The [next-packet brief](docs/next-pr.md)
defines the bounded next increment. [Validation](docs/validation.md) separates source,
CI/native API tests and real Desktop qualification.

The in-place T-07 clarification and v2 source-manifest migration are submitted in
PR #3 for owner review, as described in [the scoped amendment](docs/spec-amendments/ca-02-t07.md).
Original specification bytes remain reconstructable to their retained digest;
unchanged comparison/index inputs still verify byte-for-byte. A passing document
check or merge never qualifies an installation. The
[comparison](docs/codex-switchers-comparison.html) remains supporting research,
not authority to expand the product. Open HTML files locally for their views.

## Build and validate

Use Python 3.11+ and the pinned Rust 1.90.0 toolchain with rustfmt and Clippy.
Run these commands individually and stop at any nonzero exit. Dependency review
must pass before acquisition. Do not weaken `--locked`. The Windows Q0 procedure
below includes explicit PowerShell exit checks for its build/collection steps.

```console
python tools/check_dependencies.py
cargo fetch --locked
python -m unittest discover -s tests/python -v
python tools/spec_index.py check
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Hosted Windows tests exercise real OS APIs using synthetic objects. They are not
installed-Codex or consumer-release evidence. No real authentication fixtures belong
in Git, logs or support reports.

## Read-only discovery

Build `codex-accounts-discovery` and run it without arguments to list current-user
registered candidates. See the complete [Q0 procedure](docs/q0-qualification.md) for
safe selection, expected refusals and redactions. `--help` describes the developer
entry point. `--show-local-paths` is terminal-only and must not be shared.

The original `tools/q0_inventory.py` is retained as a legacy synthetic developer
aid. It does not replace native discovery or resolve effective Desktop configuration.
Neither inventory opens live authentication files, executes nominated binaries,
writes the nominated home or creates a qualification receipt.

## Source layout and authority

| Path | Purpose |
| --- | --- |
| `crates/core` | Refusal, compatibility observations and separate result vocabulary |
| `crates/platform` | Platform classification; no native credential writer |
| `crates/discovery` | Read-only native inventory with a narrow documented unsafe boundary |
| `crates/runtime` | Bounded untrusted byte framing and method-name allowlist, not live RPC |
| `crates/vault` | Exact-byte in-memory resource/identity/generation rules |
| `crates/vault-crypto` | Authenticated envelopes and Windows root-key protection; no disk storage |
| `app` | Future Tauri shell; no consumer application yet |
| `compatibility` | Empty catalog; no record or boolean can open production authority |
| `docs` / `tools` | Specification, roadmap, scoped validation and developer checks |

Changing accounts does not isolate local history or workspaces. The later product
must protect inactive credentials, preserve newest generations and recover from
interrupted writes before live handoff exists. Signing, packaging, reproducibility,
license selection and real Desktop qualification remain separate release work.
No owner license has been selected; crates retain `publish = false`.
