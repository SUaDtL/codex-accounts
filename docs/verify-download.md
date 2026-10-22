# Verify a Preview download

Preview downloads include Cosign signatures for the executable and the checksum
manifest. They identify the GitHub publishing workflow and detect modified files.
They are **not Windows Authenticode signatures** and do not remove SmartScreen warnings.

Download the [Windows Preview](https://github.com/SUaDtL/codex-accounts/releases/download/preview/codex-accounts-windows-x64.zip)
and extract it. Install Cosign using its [official instructions](https://docs.sigstore.dev/cosign/system_config/installation/).
The publisher currently uses the checksum-pinned Cosign 3.1.4 release.

From PowerShell in the extracted folder, verify the executable before running it:

```powershell
cosign verify-blob --bundle codex-accounts-desktop.exe.sigstore.json --certificate-identity https://github.com/SUaDtL/codex-accounts/.github/workflows/artifact-retention.yml@refs/heads/main --certificate-oidc-issuer https://token.actions.githubusercontent.com codex-accounts-desktop.exe
```

Require a successful exit and `Verified OK`. The identity and issuer must match
exactly; do not replace them with permissive regular expressions or disable
transparency-log/certificate verification. A missing signature bundle is not a
verified download. Older CI artifacts created before Preview publication may lack it.

To also verify the source, metadata and dependency inventory, first verify their
checksum manifest with the same identity:

```powershell
cosign verify-blob --bundle SHA256SUMS.sigstore.json --certificate-identity https://github.com/SUaDtL/codex-accounts/.github/workflows/artifact-retention.yml@refs/heads/main --certificate-oidc-issuer https://token.actions.githubusercontent.com SHA256SUMS
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
source and build instructions. The fixed `preview` Git tag marks the channel's
original publication, so use the bundled archive or the explicit source link in
the release notes for the current binary.

Cosign's public transparency log records the signing identity and artifact digest.
No application credentials or account data participate in signing. A valid signature
proves artifact integrity and the expected signing identity; it is not a claim that
the program is vulnerability-free or approved by Microsoft.
