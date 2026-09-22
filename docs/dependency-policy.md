# Dependency and build boundary

Rust remains pinned to 1.90.0. Core, platform classification and runtime framing
remain dependency-free and forbid unsafe code. CA-02 adds one discovery-only crate:
`serde_json = 1.0.145`, `toml = 0.8.23`, `windows = 0.62.2` and
`windows-sys = 0.61.2`, with exact direct pins and a Cargo-generated lockfile.
No Tauri/npm dependency or product network client is added.

## CA-02 review

`ca-02-dependencies.json` records all 32 resolved external packages, registry
checksums, licenses, minimum Rust versions, upstream repositories and the five
build-script hashes. Package/file checksums were compared with the acquired vendor
sources. The selected graph supports the pinned compiler. This is a scoped review,
not a claim that every dependency was comprehensively audited.

The five build scripts were read in full. serde/serde_core generate private modules
inside Cargo OUT_DIR and query the compiler. quote queries the compiler version.
proc-macro2 compiles static feature probes using Cargo's compiler/wrappers and
cleans its own OUT_DIR/probe. serde_json emits target-width configuration. None of
these inspected scripts downloads executable code or reads application accounts.
The build environment and configured compiler/wrappers remain trusted inputs.

At RustSec advisory database commit
`f7dc4b2860b29978f400fda0aab31cc4dbd21134`, package-name matching found historical
RUSTSEC-2024-0402 for hashbrown and RUSTSEC-2022-0008 for windows. Selected versions
0.17.1 and 0.62.2 respectively are in their patched ranges. No other matching
package advisory was found at that snapshot. This does not promise future freedom
from vulnerabilities; review the database again before distribution.

Run `python tools/check_dependencies.py` before dependency acquisition. Changes to
any resolved package/version/checksum require another scoped review. Acquisition
uses `cargo fetch --locked`; subsequent compilation/tests/Clippy use
`--locked --offline`. The temporary source-preparation workflow is removed before
handoff. Its prior runs prepared artifacts, not passing product validation.

## Native boundary

Only `crates/discovery/src/native.rs` permits application-authored unsafe code.
Its neighboring code denies unsafe and the existing safe crates retain forbid.
See `ca-02-native-boundary.md`. There is no blanket safety-lint exception.
No owner project license is selected by recording dependency licenses.

CI retains the existing checkout commit pin and read-only repository permissions.
Hosted runner images and compiler downloads remain external dependencies. A clean
source build is not T-34 reproducibility or Desktop qualification. Python developer
tools remain standard-library-only and require Python 3.11 or newer.

Sources: acquired Cargo registry source/checksum manifests; the recorded RustSec
snapshot; https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/ ;
https://github.com/microsoft/windows-rs ; https://github.com/RustSec/advisory-db .
The current official configuration reference is a key-name reference only, not an
installed-build precedence rule: https://developers.openai.com/codex/config-reference/ .
