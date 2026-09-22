# Dependency and build boundary

Rust is pinned to 1.90.0, an existing upstream release verified against the Rust
project's release announcement. This is a deliberate initial toolchain pin, not a
claim that it is the latest version. The initial workspace uses only std and has
no build.rs, third-party crates, npm packages or product network clients.

Cargo.lock lists only the three local workspace packages. Its content was assembled
for this dependency-free bootstrap; Cargo was not available in the creation
runtime, so lockfile/compiler acceptance remains NOT RUN until CI or owner testing.

Python developer tools use the standard library, require 3.11+ for tomllib, and were
executed under Python 3.13.5. They are not a second product runtime.

CI pins actions/checkout v4.2.2 to
`11bd71901bbe5b1630ceea73d27597364c9af683`, verified through its upstream tag ref.
That verifies tag identity only, not a complete action supply-chain audit. Hosted
runner images remain external dependencies and are not reproducible OS images.
Toolchain acquisition precedes offline workspace checks. Workflow permissions are
contents:read; no deploy, publish, release, credential or secret access is needed.

Q4 must add exact reviewed Rust/Tauri/npm pins and generated lockfiles. Native
libraries, transitive dependencies, licensing and advisories must then be reviewed.
S-015/S-016 and T-34 remain open: no native package, SBOM, two-build reproducibility,
code signing, offline toolchain cache or complete dependency audit is claimed.

Sources used for this implementation decision (separate from the supplied spec):
- https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/
- https://api.github.com/repos/actions/checkout/git/ref/tags/v4.2.2

The inventory's `cli_auth_credentials_store` classification key was verified against
the official configuration reference on 2026-09-22. This is a declared top-level
setting only, never evidence of effective configuration precedence:
https://developers.openai.com/codex/config-reference/
