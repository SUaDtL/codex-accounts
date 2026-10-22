# Implemented toolchain and verification commands

Source-backed documentation for the existing Windows application, recorded 2026-10-22.
This is not a release approval or native-host acceptance receipt.

Rust 1.90.0; Tauri 2.12.4/tauri-build 2.7.3 on Windows; embedded framework-free assets;
Python 3.11+ developer tools; Playwright 1.64.1 synthetic Chromium checks.
Versions/features are governed by Cargo.toml, Cargo.lock, package-lock.json and
`tools/check_dependencies.py`. See CONTRIBUTING.md and docs/ci.md.

Focused test commands:
- `python -m unittest discover -s tests/python -p test_ci*.py -q`
- `python -m unittest discover -s tests/python -p test_build_delivery.py -q`
- `npm run test:ui`
- `cargo test -p codex-accounts-runtime --locked --offline`
- `cargo test -p codex-accounts-desktop --locked --offline`
- Affected Windows native tests use `python tools/run_native_tests.py debug`.

Lint/type/build:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo build -p codex-accounts-desktop --release --locked --offline`
- `python tools/spec_index.py check`
- `python tools/ci_contract.py`

Security/source scan, including staged text in the working tree:
- `python tools/security_scan.py trivy`
- `python tools/security_scan.py opengrep`

Full Python checks: `python -m unittest discover -s tests/python -q`.
Exhaustive platform/profile coverage runs on the exact PR source in hosted CI.

## Coverage measurement

Rust crates, Python tools/tests and bundled JavaScript currently have no configured
line/branch coverage measurement. Percentages are unmeasured. Behavioral suites and
named native execution checks remain mandatory; no coverage percentage is claimed.
