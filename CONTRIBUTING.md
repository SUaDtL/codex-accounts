# Contributing

Codex Accounts is a Windows Rust application with an embedded HTML, CSS, and JavaScript interface. Small, focused pull requests are easiest to review.

Never use real credentials, account identifiers, authentication URLs, or personal screenshots in tests, logs, commits, or CI.

## Development setup

Install:

- Rust 1.90.0
- Python 3.11 or newer
- Visual Studio C++ Build Tools and the Windows SDK
- Microsoft Edge WebView2
- Git

Node is only needed for browser-based interface checks.

Start by checking the pinned dependencies:

\`\`\`console
python tools/check_dependencies.py
cargo fetch --locked
\`\`\`

Build or run the desktop app:

\`\`\`console
cargo build -p codex-accounts-desktop --locked --offline
cargo run -p codex-accounts-desktop --locked --offline
\`\`\`

Close an older Codex Accounts process before starting a new build.

## Checks

Run the smallest useful check while working. Before opening a pull request, run the checks that cover the files you changed.

Rust changes:

\`\`\`console
cargo fmt --all -- --check
cargo test -p codex-accounts-desktop --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
\`\`\`

Native storage or process changes:

\`\`\`console
python tools/check_windows_process.py
python tools/ci_workspace_tests.py debug
python tools/run_native_tests.py debug
\`\`\`

Interface changes:

\`\`\`console
npm ci --ignore-scripts --no-audit --no-fund
node node_modules/playwright/cli.js install chromium
npm run test:ui
\`\`\`

Repository checks:

\`\`\`console
python tools/spec_index.py check
python -m unittest discover -s tests/python
\`\`\`

CI runs the full set of platform, dependency, UI, and security checks. See [CI and releases](docs/ci.md) for the job layout.

## Dependency changes

Keep external versions exact. Update the relevant lockfile and dependency review record in the same pull request. Do not relax certificate checks or replace a checksum-verified download with an unverified installer.

The project uses Trivy and OpenGrep in CI. Scanner versions and rules are documented in [security tooling](security/README.md).

## Pull requests

Describe the behavior that changed, the checks you ran, and any limit a reviewer should know about. Keep generated account data, build outputs, local review evidence, and unrelated work out of the commit.

All changes go through a pull request. Required CI must pass on the current head before merge. Successful builds from \`main\` update the unsigned rolling Preview release.

The project stays in the 0.x version range until the owner explicitly chooses a 1.0 release.

## License

Contributions are accepted under [GNU AGPL version 3 only](LICENSE). Preserve third-party notices and licenses when changing or redistributing dependencies.
