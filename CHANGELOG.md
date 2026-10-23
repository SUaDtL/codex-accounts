# Changelog

This file records user-visible changes in versioned Codex Accounts releases.

## [1.0.0-rc.1] - 2026-10-23

### Added

- Save, label, monitor, repair, and switch multiple Codex Desktop accounts from one Windows app.
- Protect inactive credentials with Windows DPAPI and an optional Argon2id passphrase.
- Keep the app available from the system tray, including quick account switching.
- Show live usage windows and available reset credits for saved accounts.
- Publish an immutable Windows release candidate with checksums, SBOM, corresponding source, and Cosign verification bundles.

### Security

- Refuse unsupported runtimes, unsafe storage layouts, unexpected permissions, active clients, and incomplete recovery state before account changes.
- Verify release inputs against a successful exact-commit CI run before creating a version tag or GitHub Release.

### Known limits

- Windows x64 and the Microsoft Store Codex Desktop installation are the supported release target.
- The executable is not Authenticode-signed, so Windows may show an unknown-publisher or SmartScreen warning.
- This release candidate does not provide automatic account rotation, concurrent account use, or an API proxy.
