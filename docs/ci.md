# CI and build delivery

`Repository checks` runs for pull requests, pushes to main, merge queues and manual
requests. Newer commits cancel obsolete runs on the same PR. Main and merge-queue
runs are not cancelled by unrelated work. The stable aggregate check is **CI gate**.

## Parallel work

| Job | Work |
| --- | --- |
| Source integrity | Specification/dependency/workflow checks, Python tools and committed source evidence. |
| Rust, six shards | Ubuntu, Windows and macOS, each with independent debug and release profiles. |
| Isolated build | Fresh Linux target, offline dependency use and direct IP network isolation. |
| Security, two shards | Trivy CVE/secrets/configuration and OpenGrep source rules with rule fixtures. |
| Desktop UI | Chromium against bundled assets and synthetic IPC, including narrow-window regressions. |
| CI gate | Requires every prerequisite job to succeed, including all matrix children. |

Debug shards run formatting, doctests and strict Clippy. Both profiles run workspace
tests; Windows also runs the process supervisor and the complete named native fixture
partition for that profile. Release shards build optimized binaries. The Windows release
shard packages and scans the dependency SBOM before uploading the executable.

Compiler caches are separated by OS, architecture, toolchain, profile and dependency
identity. Cache hits never skip checks. The isolated lane uses a fresh target and does
not restore compiled output. Cargo offline controls dependency acquisition; only the
isolated lane also removes direct IPv4/IPv6 connectivity from build/test processes.

The three most recent successful baseline runs took roughly 14 to 19 minutes. Splitting
profiles removes sequential Windows debug/release work from the critical path, but the
new pipeline's actual duration must be measured on hosted runners. Queue time, cache
warmth and added desktop compilation can outweigh the split; no speedup is claimed yet.

## Security

Trivy scans Cargo.lock and package-lock.json, including development dependencies, for
high/critical advisories. It scans secrets separately at all severities, including documentation and fixtures,
and enables supported configuration scanning.
It separately scans the packaged CycloneDX dependency inventory before binary upload.
The SBOM includes all Cargo.lock packages, including build and other-platform dependencies;
it is a conservative inventory, not a claim every listed library is linked into the EXE.
Trivy does not decompile the PE to prove all machine-code components are safe.

OpenGrep runs eight repository-owned rules with positive/negative fixtures. They cover
renderer injection/storage, unsafe Python execution/deserialization, shell spawning and
TLS-validation bypass patterns. No floating public rule registry is used. SARIF appears
in GitHub code scanning where permissions allow; fork PRs still execute blocking scans
and upload short-lived reports without requesting write access. Tool errors fail the job.

A separate daily workflow reruns both scanners on the default branch with fresh advisory
lookups, without rebuilding the app. See [tooling policy](../security/README.md).

## Download and retention

A successful Windows release shard uploads `codex-accounts-windows-x64-RUN-ATTEMPT`:

- `codex-accounts-desktop.exe`: unsigned pre-1.0 Windows executable.
- `BUILD.json`: source revision, version, payload and lock hashes, run identity.
- `Cargo.lock` and `SBOM.cdx.json`: exact dependency inventory.
- `SHA256SUMS` and `README.txt`: verification and usage notes.

Only choose an artifact from a **fully successful Repository checks run**. Upload can
finish before other jobs, so its presence alone is not a passing CI result.

`Keep latest successful build` runs after CI completes. It uses code from the default
branch and only GitHub API metadata, never PR code or downloaded artifacts. It preserves
the newest successful owner-repository run/attempt and deletes older build artifacts and
failed completed candidates. A failed rerun cannot discard the preceding successful
attempt. Running/unknown runs are untouched. Fork candidates are removed after completion
and cannot replace the owner's last good build. Reports expire after two days; committed
source evidence after seven. The retained binary has GitHub's 90-day expiry if no newer
successful build arrives. No tags or releases are created.

The cleanup workflow activates only after it exists on the default branch. The manual
workflow dispatch runs the same cleanup; `python tools/retain_latest_build.py` defaults
to a read-only preview with `GITHUB_REPOSITORY` and `GH_TOKEN` supplied locally.

## Merge and release boundaries

At inspection on 2026-10-22, main had no branch protection or rulesets. The workflow
provides a stable CI gate but does not itself enforce merge policy. A maintainer can
configure that check as required separately. This work does not change repository
protections, merge PRs, select a license, sign an executable or publish a release.
