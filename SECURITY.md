# Security

Codex Accounts manages authentication material locally. Inactive accounts are encrypted
in a per-user vault, with key protection provided by Windows DPAPI. The installed Codex
file credential remains in Codex's expected format, including plaintext where required.
Vault encryption is not protection against same-user malware, administrators or a
compromised operating system.

## Boundaries

- Account mutations require supported official installation, runtime, policy, storage
  protection and client-state checks. The app never force-closes Codex clients.
- Browser authentication uses the official runtime in an isolated temporary home.
  The app does not implement its own OAuth refresh flow or export credentials.
- The renderer receives display metadata, not tokens or vault keys. Bundled assets are
  local; no frontend development server is required.
- Usage monitoring sends read-only HTTPS requests with existing tokens to the fixed
  ChatGPT endpoint. It can run while Codex is open and never consumes available resets.
- Journals preserve recovery across interrupted account changes. Completed records are
  retired only after verified cleanup; pending recovery is retained.
- Specifically verified Codex-managed sandbox permissions are supported. Arbitrary
  readers/writers and unexpected links are refused; the app does not alter ACLs to pass.

Account switching does not isolate Codex projects or conversation history. Unsupported
setups remain unsupported even when a UI or library test passes. The current distribution
is unsigned and pre-1.0; no complete independent security audit or signed release is claimed.

## Reporting

No private vulnerability intake channel has been configured. A GitHub issue is public.
Report non-sensitive defects with a minimal synthetic reproduction. For a potentially
exploitable vulnerability, first ask the maintainer for a private reporting channel
without posting exploit details or secrets.

Never share tokens, authentication URLs, raw helper output, `auth.json`, `cap_sid`,
staging homes, vault files, emails, real account/workspace IDs or local project paths.
Settings > Diagnostics exports minimized operation codes and timestamps, excluding
labels, IDs, credentials, paths and login URLs. Review anything added to that export.

## Supply chain

CI blocks on high/critical Trivy findings in Cargo/npm dependency inventories and the
packaged Cargo SBOM, plus enabled secret/configuration detections. OpenGrep runs tested
repository-owned rules against Rust, JavaScript and Python. Scanners are version- and
SHA-256-pinned; actions are commit-pinned. A scheduled scan checks new advisories daily.
These controls reduce risk but do not prove an executable is vulnerability-free.
See [security tooling](security/README.md) and [CI](docs/ci.md) for scope and limitations.
