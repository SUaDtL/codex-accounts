# Desktop application

Windows x64 Tauri host for Codex Accounts. It embeds `assets/` and exposes typed native
commands for installation status, vault lifecycle, profile management, isolated sign-in,
switching, recovery, usage observation and sanitized diagnostics.

`src/main.rs` owns the native window and command registration. `src/vault.rs` dispatches
account work to the dedicated thread that owns the thread-affine native vault mutex.
Bounded admission prevents overlapping operations and remains held until work finishes.
`src/metadata.rs` projects native results into display-only values. Credential values and
vault keys never cross renderer IPC.

The renderer is framework-free HTML/CSS/JavaScript. Browser checks in `tests/ui/` use the
actual assets and synthetic IPC; authentication remains owner-operated. The Windows
build uses WebView2 already installed on the machine. There is no embedded downloader,
frontend server, remote page or automatic app updater.

```console
cargo build -p codex-accounts-desktop --release --locked --offline
```

Portable CI does not provide a Linux or macOS desktop app. See the root [README](../../README.md),
[architecture](../../docs/architecture.md) and [security boundary](../../SECURITY.md).
