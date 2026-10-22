# Codex Accounts

A compact Windows app for switching between saved ChatGPT accounts in Codex Desktop.

**0.1.0 · Pre-1.0 · Windows x64**

[![Download for Windows](https://img.shields.io/badge/Download-Windows_x64-2c9b80?style=for-the-badge)](https://github.com/SUaDtL/codex-accounts/releases/download/preview/codex-accounts-windows-x64.zip)

Save the account you already use, add another through the official browser sign-in,
and switch when you need to. See each account's remaining usage and reset countdowns
without closing Codex. Credentials stay on your computer.

![Accounts with synthetic Personal and Work profiles](docs/images/accounts.png)

*Preview uses synthetic accounts and usage values.*

## Get a build

Download the [Windows Preview](https://github.com/SUaDtL/codex-accounts/releases/download/preview/codex-accounts-windows-x64.zip),
extract the archive and run `codex-accounts-desktop.exe`. No GitHub sign-in is required.
**No Windows signature:** Windows may show an unknown-publisher or SmartScreen warning.
Cosign verification is available for the executable and checksum manifest;
[verify your download](docs/verify-download.md). This does not change Windows trust.

The archive includes `SHA256SUMS`, `BUILD.json`, `Cargo.lock`, `SBOM.cdx.json`,
`LICENSE`, Cosign verification bundles and the exact corresponding source in `source.tar.gz`.
One rolling [Preview release](https://github.com/SUaDtL/codex-accounts/releases/tag/preview)
holds the latest successfully published main build. Failed checks leave the previous
download available. The release notes identify the exact source revision.

The first Preview becomes available after this workflow reaches `main` and its CI passes.
Until then, developers can download the Windows artifact from a successful
[Repository checks](https://github.com/SUaDtL/codex-accounts/actions/workflows/ci.yml)
run; Actions artifact downloads require GitHub sign-in and expire after 90 days.

Requirements: Windows x64, the official Microsoft Store Codex Desktop installation,
and Microsoft Edge WebView2. No Node installation or frontend server is needed.

## Use it

1. Create your vault on first use, or open the existing one.
2. Use **+** to save the current account or sign in to another. Give it any label.
3. Quit Codex clients normally before saving, signing in or switching.
4. Select **Switch** beside an account. Codex opens after the switch succeeds.

The lit indicator marks the last known active account. **Open Codex** reopens it.
The header refresh button reloads accounts and usage; the cog opens Settings.
Account menus provide rename, removal and **Sign in again**.

Usage refreshes once per minute while this window is visible and the vault is open.
Codex can stay open during monitoring. Expired credentials clear usage instead of
showing stale balances. Only reported quota windows appear. Available banked resets
appear when the service reports them; the app never consumes resets.

Accounts share this computer's Codex projects and conversation history. Switching
changes the sign-in, not the history. Vault encryption uses your Windows account;
it does not protect against software already running as you or an administrator.

## Compatibility

Normal Codex updates do not need a version allowlist. Operations check the actual
installation, runtime, storage, policy and running clients. Unsupported configurations
are refused without changing credentials. This build supports native Windows
file-backed ChatGPT sign-ins. WSL, API-key accounts, custom runtimes/providers and
macOS account switching are not supported.

## Build from source

Install Rust 1.90.0 and Visual Studio C++ Build Tools with the Windows SDK, then:

```console
python tools/check_dependencies.py
cargo fetch --locked
cargo build -p codex-accounts-desktop --release --locked --offline
```

Run `target/release/codex-accounts-desktop.exe`. Close any older Accounts window first.
The UI is embedded in the executable.

## Documentation

- [User guide](docs/user-guide.md): account management, usage and recovery.
- [Development](CONTRIBUTING.md): setup, focused checks and contribution workflow.
- [Architecture](docs/architecture.md): crate responsibilities and credential boundaries.
- [CI and build delivery](docs/ci.md): parallel jobs, security scans and artifact retention.
- [Security](SECURITY.md): protection limits and safe reporting.
- [Documentation index](docs/README.md): specification, roadmap and historical evidence.

## License

Codex Accounts is licensed under [GNU AGPL version 3 only](LICENSE)
(`AGPL-3.0-only`). Third-party dependencies retain their own licenses.

The owner has confirmed a real account round trip and accepted the current UI.
Preview builds remain unsigned and pre-1.0; a passing pipeline is not a complete
independent security audit.
