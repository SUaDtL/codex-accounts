# Security tooling

These tools run in development/CI and are not shipped in Codex Accounts.

| Tool | Pin | License | Purpose |
| --- | --- | --- | --- |
| Trivy | 0.76.1 | Apache-2.0 | Cargo/npm CVEs, secrets, supported configuration and package SBOM scans. |
| OpenGrep | 1.31.0 | LGPL-2.1 | Repository-owned static rules and their positive/negative fixtures. |
| Playwright | 1.64.1 | Apache-2.0 | Synthetic Chromium UI checks. |
| Cosign | 3.1.4 | Apache-2.0 | Keyless Preview signatures and exact workflow-identity verification. |

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
All Rust, JavaScript and Python files in those roots must appear in the scanner's target
inventory, including first-party tests. `.semgrepignore` replaces implicit test exclusions
and excludes only generated Tauri schemas. Build output, dependencies and scanner rule
fixtures live outside these source roots; the rule fixtures run separately.
`--strict`, `--error` and disabled inline suppression keep failures visible.

OpenGrep writes JSON and SARIF from the same scan. Empty or incomplete target inventories,
reported parse/time/resource errors, malformed reports and failed execution block CI.
Every finding blocks. Metadata records actual engine/rule/report hashes, source counts,
analysis mode, elapsed time and the CI commit. Reports are cleared before installation;
failed reruns leave incomplete metadata. Scanner cache, logs and settings use a fresh
temporary runtime, separate from an authenticated developer session.

### OpenGrep coverage contract

These eight rules use **structural SAST**, not source-to-sink taint analysis. A prohibited
construct is rejected regardless of the input's origin. No intrafile or cross-file flow
coverage is claimed. Keep the stable pinned engine; an experimental interfile engine is
not part of the required scan. Changes to analysis mode need representative flow fixtures.

| Component | Inputs and operations | Wrapper boundary and limitation | Proof |
| --- | --- | --- | --- |
| Renderer JavaScript | Account labels, sanitized IPC and form input; DOM, code execution and storage | Direct dangerous HTML/code/storage APIs are prohibited. Does not prove native IPC authorization. | `opengrep/javascript.js`: positive and clean alternatives for three rules. |
| Python developer tools | CLI arguments, GitHub metadata, downloaded bytes; subprocess and deserialization | Direct shell/pickle/eval constructs are prohibited. Does not prove archive/path validation or argument safety. | `opengrep/python.py`: positive and clean alternatives for two rules. |
| Native Rust | Files, runtime replies and usage responses; process launch and HTTPS | Direct shell launch and TLS bypass patterns are prohibited. Does not prove FFI safety, credential ownership or recovery logic. | `opengrep/rust.rs`: positive and clean alternatives for three rules. |

Repository rules are locally authored; no external rule pack or floating registry is
downloaded. Review source, runtime behavior and dependency advisories separately. The
2026-10-22 local expanded scan covered 97 Rust, one JavaScript and 24 Python files with
zero findings and zero reported errors. These counts are evidence, not a fixed allowlist:
new source files become required targets automatically.

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

## Preview signatures

The owner selected Cosign as a free verification mechanism, accepting that it does not
provide Windows Authenticode or SmartScreen trust. The Preview job alone receives
`id-token:write`. Cosign requests a short-lived GitHub Actions identity token, signs
the EXE and checksum manifest, and verifies both against the exact default-branch
publishing workflow identity before uploading. Verification bundles travel inside
the same ZIP, avoiding separately updated binary/signature release assets.
Signing/verification failures leave the previous download in place. No persistent
signing key, paid service, local developer sign-in or application credential is used.

Cosign 3.1.4 acquisition uses the existing immutable URL/checksum installer. Its
Apache-2.0 license, official release/checksum metadata and published advisories were
reviewed on 2026-10-22. Trivy 0.76.1 scanned upstream checksum-verified SBOMs covering
256 Linux and 266 Windows packages with no HIGH/CRITICAL findings reported. This is
bounded SBOM evidence, not an exhaustive audit. See the [official release](https://github.com/sigstore/cosign/releases/tag/v3.1.4)
and [advisory fixed before this version](https://github.com/sigstore/cosign/security/advisories/GHSA-fx35-mq7g-6g98).
