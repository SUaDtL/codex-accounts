# Architecture

Codex Accounts is a Windows Tauri application with bundled HTML, CSS and JavaScript.
The renderer receives typed display data through a small Rust IPC surface. It never
receives credential files, bearer tokens or vault keys.

| Component | Responsibility |
| --- | --- |
| `desktop-app` | Native window, embedded UI, typed commands, display projections and vault worker. |
| `discovery` | Official package inventory, installation identity and runtime selection. |
| `runtime` | Bounded official-helper protocol, framing, configuration and login sessions. |
| `vault-storage` | Native filesystem/ACL checks, profile operations, transactions, recovery and usage transport. |
| `vault` / `vault-crypto` | Strict record format, authenticated encryption and protected key handling. |
| `core` / `platform` | Shared models and platform classification. |
| `shell-preparation` | Historical shell preparation and portable boundary checks. |

## Native operations

A dedicated worker owns the vault because its native location mutex is thread-affine.
Bounded admission prevents overlapping account operations. A disconnected renderer does
not abandon a running native mutation or start a second operation against it.

Saving captures the current sign-in after runtime, backend, policy and protection checks.
Adding uses an owned isolated home and the official browser login. Switching coordinates
client shutdown checks, encrypted recovery records, credential installation and official
runtime validation. Recovery retains enough state to resolve interrupted mutations.
No arbitrary command, path or credential-export API is exposed to the renderer.

The active Codex file credential remains in the format Codex expects. Inactive profiles
are encrypted using a key protected for the current Windows user by DPAPI. Storage
validation permits the specifically verified Codex-managed permissions documented in
[the permission amendment](spec-amendments/managed-parent-permissions-2026-10-21.md).
The app does not rewrite Windows permissions to force a configuration to pass.

## Usage observation

The native usage adapter sends bounded HTTPS GET requests to the fixed ChatGPT usage
endpoint with existing credentials. It parses optional windows and available-reset counts
into non-secret metadata. It does not perform direct OAuth refresh. Observations expire,
and authentication failure clears visible usage. Polling pauses when the window is hidden
or the vault is closed. No extra request is made for available resets.

## Updates and compatibility

Compatibility follows runtime, storage and policy capabilities rather than exact Desktop
version approval. Retained installation identity prevents an update midway through an
operation from silently redirecting credential actions. Unknown contracts refuse safely.
The [specification](local-codex-switcher-spec.html) owns normative requirements; dated
amendments record the owner's decisions. Linux/macOS CI exercises portable code, not
native account-switching support on those systems.
