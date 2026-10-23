# Verify a download

Published downloads include a detached Cosign bundle for the complete archive plus
signatures for the executable and checksum manifest inside it. They identify the GitHub
publishing workflow and detect modified or added files.
They are **not Windows Authenticode signatures** and do not remove SmartScreen warnings.

Download [1.0.0-rc.1](https://github.com/SUaDtL/codex-accounts/releases/download/v1.0.0-rc.1/codex-accounts-windows-x64.zip)
and its [Cosign bundle](https://github.com/SUaDtL/codex-accounts/releases/download/v1.0.0-rc.1/codex-accounts-windows-x64.zip.sigstore.json).
Install Cosign using its [official instructions](https://docs.sigstore.dev/cosign/system_config/installation/).
The publisher currently uses the checksum-pinned Cosign 3.1.4 release.

From PowerShell in the download folder, verify the complete archive before extracting it:

```powershell
cosign verify-blob --bundle codex-accounts-windows-x64.zip.sigstore.json --certificate-identity https://github.com/SUaDtL/codex-accounts/.github/workflows/release.yml@refs/heads/main --certificate-oidc-issuer https://token.actions.githubusercontent.com codex-accounts-windows-x64.zip
if ($LASTEXITCODE -ne 0) { throw 'Release archive signature failed' }
Expand-Archive -LiteralPath codex-accounts-windows-x64.zip -DestinationPath codex-accounts
Set-Location codex-accounts
```

Require a successful exit and `Verified OK`. This archive signature covers the exact
file inventory and every byte before extraction. Then verify the executable before
running it:

```powershell
cosign verify-blob --bundle codex-accounts-desktop.exe.sigstore.json --certificate-identity https://github.com/SUaDtL/codex-accounts/.github/workflows/release.yml@refs/heads/main --certificate-oidc-issuer https://token.actions.githubusercontent.com codex-accounts-desktop.exe
```

The identity and issuer must match
exactly; do not replace them with permissive regular expressions or disable
transparency-log/certificate verification. A missing signature bundle is not a
verified download. Older CI artifacts created before Preview publication may lack it.

To verify the source, metadata and dependency inventory individually, first verify
their checksum manifest with the same identity:

```powershell
cosign verify-blob --bundle SHA256SUMS.sigstore.json --certificate-identity https://github.com/SUaDtL/codex-accounts/.github/workflows/release.yml@refs/heads/main --certificate-oidc-issuer https://token.actions.githubusercontent.com SHA256SUMS
if ($LASTEXITCODE -ne 0) { throw 'Checksum manifest signature failed' }
Get-Content -LiteralPath SHA256SUMS | ForEach-Object {
    $digest, $name = $_ -split '  ', 2
    if ($name -notmatch '^[A-Za-z0-9_.-]+$') { throw 'Unexpected filename' }
    if ((Get-FileHash -LiteralPath $name -Algorithm SHA256).Hash -ne $digest) {
        throw "Checksum mismatch: $name"
    }
}
```

`BUILD.json` names the source revision. `source.tar.gz` contains that committed
source and build instructions. A versioned tag points to that same revision. If you
download from the rolling Preview channel instead, use the
`artifact-retention.yml@refs/heads/main` certificate identity and the explicit source
revision in its release notes.

Cosign's public transparency log records the signing identity and artifact digest.
No application credentials or account data participate in signing. A valid signature
proves artifact integrity and the expected signing identity; it is not a claim that
the program is vulnerability-free or approved by Microsoft.
