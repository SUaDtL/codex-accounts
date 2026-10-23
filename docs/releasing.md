# Releasing Codex Accounts

Codex Accounts has two delivery channels:

- **Preview** is a rolling prerelease updated from the newest successful `main` build.
- **Versioned releases** are immutable tags such as `v1.0.0-rc.1`.

The versioned publisher reuses the Windows package produced by the exact successful
`Repository checks` run. It does not compile a second binary. It verifies the Actions
artifact digest, package inventory, checksums, PE architecture, source commit, workflow
run, version, and dependency lock before signing or publishing anything.

## Prepare a version

Use a pull request to update all version sources together:

- `Cargo.toml`
- the internal version pin in `crates/runtime/Cargo.toml`
- `crates/desktop-app/tauri.conf.json`
- `crates/shell-preparation/tauri.conf.json`
- `package.json` and `package-lock.json`
- the matching dated section in `CHANGELOG.md`

Cargo refreshes only the workspace package versions in `Cargo.lock`. External dependency
versions and checksums remain governed by the dependency review.

Merge only after `CI gate` passes. Wait for the successful `main` run and its Preview
publisher to finish so the exact Windows artifact remains available.

## Publish

In GitHub Actions, run **Publish versioned release** from `main` and enter the exact
version without the leading `v`, for example `1.0.0-rc.1`.

The workflow:

1. Checks out the dispatch commit and refuses any ref other than `main`.
2. Requires every committed version source and changelog section to match the input.
3. Selects the one successful owner-repository `main` CI run for that exact commit.
4. Verifies the package, signs its executable and checksum manifest, then signs the
   complete release archive with a separate Cosign bundle.
5. Uploads both assets to a draft, creates an annotated `vVERSION` tag at the exact
   commit, publishes the GitHub Release, and verifies the immutable release by reading
   its tag and asset digests back.

Release candidates are marked as prereleases and never take GitHub's Latest badge. A
stable semantic version may take Latest. A rerun after successful publication performs
verification only and needs no retained CI artifact. If a run stops after uploading the
archive, a later run verifies that signed archive and completes the draft without rebuilding.

The workflow needs no repository secret. GitHub supplies a short-lived workflow token and
OIDC identity. Job permissions are limited to `actions: read`, `contents: write`, and
`id-token: write`; checkout credentials are not persisted.

Never move, delete, or recreate a published version tag. If a release is bad, publish a
new version and mark the older release as superseded.

## Repository settings

`main` requires the `CI gate`, blocks force pushes and deletion, and requires resolved
review conversations. A tag ruleset for `refs/tags/v*` blocks tag updates and deletions
while allowing the release workflow to create new versions. Repository release immutability
is enabled. GitHub locks the tag and both assets when the draft is published, and the
publisher refuses a final read-back unless GitHub reports the release as immutable.

No Authenticode certificate is configured. Cosign proves the GitHub workflow identity and
artifact integrity, but Windows can still display SmartScreen or unknown-publisher warnings.
