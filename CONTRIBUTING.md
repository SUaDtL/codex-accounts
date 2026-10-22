# Contributing

Use Rust **1.90.0**, Python **3.11+** and Git. Building the desktop app also requires
Windows x64, Visual Studio C++ Build Tools, the Windows SDK and WebView2.
Node/npm are needed only for browser checks, not for running or building the embedded UI.

Read [AGENTS.md](AGENTS.md) and the relevant specification symbols before changing
credential or recovery behavior. Keep unrelated work intact. Do not use real account
material in tests, logs, screenshots, commits or CI.

## Setup and focused checks

```console
python tools/check_dependencies.py
cargo fetch --locked
cargo fmt --all -- --check
cargo test -p codex-accounts-desktop --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
python tools/spec_index.py check
python -m unittest discover -s tests/python
```

Run the affected crate or Python module while iterating. Full platform/profile coverage
belongs in CI. Native storage changes additionally need the named Windows fixture tests:

```console
python tools/check_windows_process.py
python tools/ci_workspace_tests.py debug
python tools/run_native_tests.py debug
```

`run_native_tests.py` accepts `debug` or `release`; omitting the argument runs both.
It uses owned synthetic fixtures and verifies actual named executions. Do not replace
these with an aggregate zero-test success.

## UI and security checks

```console
npm ci --ignore-scripts --no-audit --no-fund
node node_modules/playwright/cli.js install chromium
npm run test:ui
python tools/security_scan.py trivy
python tools/security_scan.py opengrep
```

UI tests serve the real bundled assets with synthetic IPC. They cover compact layouts,
account controls, usage visibility and recovery presentation, not real authentication.
Screenshots go into ignored `reports/ui/`. Security tools download exact checksum-verified
upstream binaries into ignored `.ci-tools/`; see [tooling policy](security/README.md).

## Changes and review

Keep external versions exact and update their lockfiles and dependency review records.
Do not relax certificate checks, log credentials, introduce remote renderer content,
or bypass compatibility checks to make a test pass. A failing check needs a diagnosis.

PRs should explain the resulting behavior, focused local evidence and remaining limits.
CI builds a downloadable Windows candidate and must pass before merge. Opening a PR
does not authorize merging, signing or publishing a release. Product versions remain
in the 0.x series until the owner explicitly chooses otherwise.

Documentation lives in [docs](docs/README.md). Keep current guidance separate from dated
historical evidence. Do not copy locally generated `.review-evidence`, account files or
build outputs into a commit. No project license is selected yet; do not add one on the
owner's behalf.
