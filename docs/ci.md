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
Native cases have a 180-second process deadline, except the restoration batch's
300 seconds for its control plus 17 crash/recovery fixtures. Its individual child
watchdogs remain 75 seconds; no case is retried after failure. The Windows job has
a separate 30-minute limit.

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
TLS-validation bypass patterns. These are structural rules; cross-function and cross-file
taint coverage is not claimed. The scan requires every Rust/JavaScript/Python source in
`crates/` and `tools/`, including first-party tests, and rejects missing targets or reported
analysis errors. JSON, SARIF and scan metadata are retained together. No floating public
rule registry is used. SARIF appears
in GitHub code scanning where permissions allow; fork PRs still execute blocking scans
and upload short-lived reports without requesting write access. Tool errors fail the job.

A separate daily workflow reruns both scanners on the default branch with fresh advisory
lookups, without rebuilding the app. See [tooling policy](../security/README.md).

## Download and retention

A successful Windows release shard uploads `codex-accounts-windows-x64-RUN-ATTEMPT`:

- `codex-accounts-desktop.exe`: Windows executable without an Authenticode signature.
- `BUILD.json`: source revision, version, payload and lock hashes, run identity.
- `Cargo.lock` and `SBOM.cdx.json`: exact dependency inventory.
- `SHA256SUMS` and `README.txt`: verification and usage notes.
- `LICENSE` and `source.tar.gz`: AGPL-3.0-only terms and corresponding committed source.

Only choose an artifact from a **fully successful Repository checks run**. Upload can
finish before other jobs, so its presence alone is not a passing CI result.

`Publish Preview and retain latest build` runs after CI completes, or manually. Both jobs
use trusted default-branch code with checkout credentials disabled. The Preview job
selects the newest successful owner-repository `main` push/manual run of `ci.yml`,
checks its artifact digest, file list, checksums, PE architecture and source/run identity,
then signs the EXE and checksum manifest with pinned Cosign and publishes one prerelease.
Both signatures must verify with the exact publishing workflow identity and GitHub OIDC
issuer. Their bundles are included in the ZIP; no Authenticode signing is performed.
PR and fork artifacts cannot be published.
Artifact files are read as data and never extracted or executed by the publisher.
The GitHub token is not forwarded to the signed artifact-storage URL.

The README download button uses a fixed `preview` tag and asset name. The tag anchors
the channel's first publication and is not moved; release notes and `BUILD.json` identify
the actual binary revision, and the ZIP includes that revision's source archive. GitHub's
automatically generated tag-source links refer to the anchor, so use the bundled source
or the explicit source link in the release notes for the current binary.

The publisher uploads and verifies a candidate before renaming the old download and
promoting the candidate. GitHub does not offer an atomic asset-name swap, so the fixed URL
can briefly be unavailable during those two renames. An ordinary promotion failure restores
the old name; an interruption preserves both assets for retry. No old bytes are deleted
until the replacement is published. Repeated and late runs cannot roll the channel backward.
It uses the public Sigstore service with ephemeral GitHub OIDC credentials and no paid
subscription. The workflow does not change Windows publisher trust.

`Publish versioned release` is a separate manual workflow on `main`. It accepts an exact
committed semantic version, selects the successful `Repository checks` artifact for that
same commit, repeats the complete package verification, and signs with the distinct
`release.yml@refs/heads/main` identity. It uploads the signed archive to a managed draft,
creates an annotated version tag, publishes the Release, and verifies the final tag and
asset by reading them back. Release candidates remain prereleases and do not take the
Latest badge. Published version assets are never replaced.

Retention runs only after publication succeeds or finds no eligible candidate, so a failed
publication keeps its input artifact available for retry. Its separate token has `actions:write`
but no repository-content write permission. The metadata-only retention tool preserves
the newest successful owner-repository run/attempt and deletes older build artifacts and
failed completed candidates. A failed rerun cannot discard the preceding successful
attempt. Running/unknown runs are untouched. Fork candidates are removed after completion
and cannot replace the owner's last good build. Reports expire after two days; committed
source evidence after seven. The retained binary has GitHub's 90-day expiry if no newer
successful build arrives. The rolling release asset does not inherit Actions artifact expiry.

The publishing/cleanup workflow activates only after it exists on the default branch. Its
manual dispatch retries publication and cleanup; `python tools/retain_latest_build.py` defaults
to a read-only preview with `GITHUB_REPOSITORY` and `GH_TOKEN` supplied locally.

## Merge and release boundaries

`main` requires the `CI gate`, blocks force pushes and deletion, enforces administrator
rules, and requires resolved review conversations. Version tags use a repository ruleset
that blocks updates and deletion while allowing new tags. The owner selected
AGPL-3.0-only and authorized automatic Preview delivery plus manually dispatched,
versioned releases from successful main CI. Authenticode signing and Store distribution
are not configured.
