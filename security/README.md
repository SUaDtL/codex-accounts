# Security tooling

These tools run in development/CI and are not shipped in Codex Accounts.

| Tool | Pin | License | Purpose |
| --- | --- | --- | --- |
| Trivy | 0.76.1 | Apache-2.0 | Cargo/npm CVEs, secrets, supported configuration and package SBOM scans. |
| OpenGrep | 1.31.0 | LGPL-2.1 | Repository-owned static rules and their positive/negative fixtures. |
| Playwright | 1.64.1 | Apache-2.0 | Synthetic Chromium UI checks. |

[tools.json](tools.json) records exact official release URLs and SHA-256 hashes for Linux
and Windows x64 scanner assets. `tools/security_tools.py` verifies the downloaded bytes
on every invocation. It extracts only the named scanner executable, never an entire
archive or arbitrary archive paths. npm uses the committed lockfile and lifecycle scripts
are disabled. GitHub Actions are pinned to full commit SHAs.

The 2026-10-22 scoped dependency review checked official release metadata, checksums,
licenses and relevant advisories. Trivy 0.76.1 is later than the compromised 0.69.4 release
and the subsequently fixed archive traversal range. That review is not an exhaustive audit
of every scanner dependency. Do not replace pins with floating tags or curl-piped scripts.

Official sources: [Trivy releases](https://github.com/aquasecurity/trivy/releases),
[Trivy advisories](https://github.com/aquasecurity/trivy/security/advisories),
[OpenGrep releases](https://github.com/opengrep/opengrep/releases), and
[Playwright releases](https://github.com/microsoft/playwright/releases).

## Rules and failures

Rules and fixture pairs live in `opengrep/`. Add a positive and a negative example when
changing a rule. Run `python tools/security_scan.py opengrep`; fixtures run before the
source scan. Rules inspect `crates/` and `tools/`, including new untracked source locally.
Build output, generated dependencies, reports and test rule fixtures are excluded.
`--strict`, `--error` and disabled inline suppression keep failures visible.

Trivy blocks on HIGH/CRITICAL CVE/configuration findings without `--ignore-unfixed`.
Secrets run separately at all severities, including Markdown and test fixtures. A
synthetic unusable JWT verifies detection before each scan. There are currently no
advisory suppressions. A necessary exception must identify the exact finding, affected
scope, justification, owner and expiry in a reviewed change; do not disable the scanner.
OpenGrep rules are deliberately scoped and do not replace code review, dependency CVE
scanning or runtime validation.

## Updating

Review upstream provenance, license and advisories first. Update `tools.json` version,
asset URLs and SHA-256 values together, then run both scanner fixtures and source scans.
For browser updates, update package.json and package-lock.json together and rerun UI checks.
Action updates require a verified upstream commit and review of permissions/events.
Record actual output and limitations. See [CI](../docs/ci.md) for trust and retention policy.
