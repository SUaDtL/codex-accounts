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

## CA-03B additive review

`ca-03b-dependencies.json` supplements the unchanged 32-entry CA-02 record with
26 checksum-bound packages. Direct pins are chacha20poly1305 0.10.1, hkdf 0.12.4,
sha2 0.10.9, getrandom 0.3.4 and zeroize 1.8.2; windows-sys 0.61.2 is reused for
the narrow root-key API. Exact manifests/features, workspace membership and full
Cargo.lock bytes are checked before acquisition. No reduced-round cipher feature,
custom random backend, Tauri/npm package or network client is added.

The supplemental record binds source-preparation run 35861770176 to its source
commit and artifact digest, and records all four new build scripts: compiler/target
probes in generic-array/getrandom/libc and a WASI-only bundled archive in wit-bindgen.
The libc script may query emcc on PATH; compiler wrappers, PATH and the controlled
build host remain trusted inputs. Target-only WASI/UEFI entries are not product
support. The bundled WASI archive is not claimed independently audited.

At the recorded RustSec commit `6477ec04375b913e13f38d966dc49eba9d178cb8`, the
source review matched all 58 locked names and inspected five advisory ranges;
the selected versions are in their patched ranges. This is the recorded source
review, not a claim that cargo-audit or a full independent security audit ran.
The exact advisory IDs, archive digest and limitations remain in the JSON record.
No new dependency or manifest change is introduced by the formatting/closeout fix.
Recheck advisories before distribution and review any future lock/feature change.

## CA-03C reuse and native scope

`ca-03c-dependencies.json` preserves the exact CA-02/CA-03B review bytes and all
58 external package/version/checksum entries. No package version or build script
is added. The local storage crate and narrowly selected Windows SDK features are
bound through complete manifest and lock hashes. Direct windows-collections 0.3.2
reuses the existing locked package for the generated WinRT collection output type.
The inherited advisory snapshot covers identical versions, not a new cargo-audit
execution or a guarantee against subsequent advisories.

Application-authored unsafe is limited to the existing discovery boundary, crypto
DPAPI module and private vault-storage native module/children. The storage adapter
uses documented SDK security/file/known-folder/CloudFilter/WinRT APIs. Additional
IO/Ioctl declarations construct junctions only inside owned synthetic tests;
production code never creates a reparse point or repairs existing ACLs. Drive type
constants come from the SDK rather than guessed values. COM initialization is
balanced on its owning thread, with uncached per-query activation factories and
no heuristic DLL-search fallback. See ca-03c-storage.md for lifetime and durability
limitations. Adjacent storage engine/codec/records/recovery and existing safe core
restrictions remain intact; there is no blanket safety-lint exception.

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
