# Codex Accounts

Manual, owner-controlled account handoff for the official Codex Desktop application.

**Development state: Q0 foundations. No installation is qualified. Account capture,
login, switching, vault storage and Desktop process control are not implemented or enabled.**
This is the first source increment, not an installable account switcher.

## Start here

The [product specification](docs/local-codex-switcher-spec.html) is the normative
contract. Its embedded `artifact-model` is the canonical structured model. The
[comparison](docs/codex-switchers-comparison.html) is supporting research, not
permission to expand the product. Both documents are preserved byte-for-byte from
the supplied project sources. Open the HTML files locally for their navigable views.

The first increment provides a read-only candidate-inventory tool, Rust safety
and status models, bounded transport primitives, tests, and the Q0 qualification
procedure. The core has no third-party crate dependencies. There is no credential
writer, OAuth client, inference proxy, HTTP server, background agent, updater or
GUI. The planned product remains Rust + Tauri 2; the Q4 shell is not fabricated here.

## Validate the bootstrap

Prerequisites: Python 3.11+ and Rust 1.90.0 with rustfmt and clippy. Use the pinned
`rust-toolchain.toml`. On an owner-reviewed machine with the toolchain acquired:

```console
python -m unittest discover -s tests/python -v
python tools/spec_index.py check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Python checks do not execute Rust. CI compilation does not qualify a real Desktop
installation. See [validation](docs/validation.md) for exactly what was and was not
executed for this source increment.

## Read-only Q0 inventory

```console
python tools/q0_inventory.py --desktop-file "/absolute/path/to/desktop/executable" --runtime-file "/absolute/path/to/bundled/runtime" --codex-home "/absolute/path/to/candidate/home"
```

On Windows, use the full Windows paths discovered for the selected installation.
The tool never invokes either nominated executable, never reads `auth.json`, and
never writes to the nominated home. It reads executable metadata/content for
hashing and an optional bounded `config.toml` to classify the declared backend.
It does not establish effective configuration, signature trust, package identity,
process quiescence or policy. Those remain explicit Q0 evidence gaps.

Default output omits local paths and all arbitrary config/environment values.
`--show-local-paths` is an explicit local-display mode; do not upload that output.
Exit 0 means inventory collected, **not** qualified or switch-ready. Every report
has `qualified: false` and `credential_mutation_enabled: false`.

Follow [the Q0 procedure](docs/q0-qualification.md), not a copy-and-replace script.
The compatibility catalog is deliberately empty.

## Source layout

| Path | Purpose |
| --- | --- |
| `crates/core` | Fail-closed availability and separate verification statuses |
| `crates/platform` | Platform classification only; no native mutation adapter |
| `crates/runtime` | Method allowlist and bounded byte framing; no subprocess or JSON-RPC session |
| `tools` | Developer-only inventory and symbol-scoped spec retrieval |
| `tests/python` | Executed bootstrap/inventory/spec tests |
| `compatibility` | Empty catalog and evidence boundaries |
| `app` | Q4 boundary, not an implemented Tauri application |
| `docs/implementation-plan.html` | Source-scoped work packages and acceptance status |

## Safety and ownership

Changing accounts will not isolate history or workspace data. The specification
requires encrypted inactive credentials, preservation of refreshed generations,
normal app shutdown and recoverable writes before enabling any live handoff.
None of those future capabilities is simulated as a successful production action.

No release license has been selected. `publish = false` prevents accidental crate
publication; repository visibility is not a license grant. See `SECURITY.md` before
sharing reports. No real credentials or machine-specific evidence belong in Git.
