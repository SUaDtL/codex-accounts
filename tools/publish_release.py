"""Publish an immutable, Cosign-verifiable release from an exact successful main build."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.parse
import zipfile

from publish_preview import (
    FILES, GitHub, eligible, read_package, sign_blob, sign_package, verify_blob,
    verify_bundle, verify_package_files,
)
from release_version import parse_semver

ROOT = Path(__file__).resolve().parents[1]
ASSET = 'codex-accounts-windows-x64.zip'
BUNDLE_ASSET = ASSET + '.sigstore.json'
INTERNAL_BUNDLES = {'codex-accounts-desktop.exe.sigstore.json', 'SHA256SUMS.sigstore.json'}
MARKER = '<!-- codex-accounts-managed-release -->'


def manifest_version(root: Path = ROOT) -> str:
    cargo = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
    versions = {
        cargo,
        json.loads((root / 'crates/desktop-app/tauri.conf.json').read_text(encoding='utf-8'))['version'],
        json.loads((root / 'crates/shell-preparation/tauri.conf.json').read_text(encoding='utf-8'))['version'],
        json.loads((root / 'package.json').read_text(encoding='utf-8'))['version'],
        json.loads((root / 'package-lock.json').read_text(encoding='utf-8'))['packages']['']['version'],
    }
    if len(versions) != 1:
        raise ValueError('Release manifests disagree')
    parse_semver(cargo)
    return cargo


def release_tag(version: str, root: Path = ROOT) -> str:
    parse_semver(version)
    if version != manifest_version(root):
        raise ValueError('Requested release does not match committed manifests')
    return 'v' + version


def changelog_notes(version: str, root: Path = ROOT) -> str:
    text = (root / 'CHANGELOG.md').read_text(encoding='utf-8')
    heading = re.compile(rf'^## \[{re.escape(version)}\] - \d{{4}}-\d{{2}}-\d{{2}}$', re.MULTILINE)
    matches = list(heading.finditer(text))
    if len(matches) != 1:
        raise ValueError('Changelog must contain one dated section for the release')
    start = matches[0].start()
    following = re.search(r'^## \[', text[matches[0].end():], re.MULTILINE)
    end = matches[0].end() + following.start() if following else len(text)
    notes = text[start:end].strip()
    if '\n- ' not in notes:
        raise ValueError('Release notes must contain at least one change')
    return notes


def find_release(api, tag: str) -> dict | None:
    release = api.optional_json('/releases/tags/' + urllib.parse.quote(tag, safe=''))
    if release:
        return release
    matches = []
    for page in range(1, 11):
        entries = api.json(f'/releases?per_page=100&page={page}')
        matches.extend(item for item in entries if item.get('tag_name') == tag)
        if len(entries) < 100:
            break
    else:
        raise ValueError('Release inventory exceeds the recovery bound')
    if len(matches) > 1:
        raise ValueError('Ambiguous release state')
    return matches[0] if matches else None


def existing_tag(api, tag: str, commit: str) -> str | None:
    ref = api.optional_json('/git/ref/tags/' + urllib.parse.quote(tag, safe=''))
    if not ref:
        return None
    if ref.get('object', {}).get('type') != 'tag':
        raise ValueError('Version tag is not annotated')
    tag_object = api.json('/git/tags/' + ref['object']['sha'])
    if (tag_object.get('tag') != tag or tag_object.get('object', {}).get('type') != 'commit'
            or tag_object.get('object', {}).get('sha') != commit):
        raise ValueError('Version tag points to different source')
    return ref['object']['sha']


def ensure_tag(api, tag: str, commit: str, create: bool = True) -> str:
    current = existing_tag(api, tag, commit)
    if current:
        return current
    if not create:
        raise ValueError('Published release is missing its immutable version tag')
    tag_object = api.json('/git/tags', 'POST', {
        'tag': tag, 'message': f'Codex Accounts {tag}', 'object': commit, 'type': 'commit',
    })
    if not re.fullmatch(r'[0-9a-f]{40}', tag_object.get('sha', '')):
        raise ValueError('GitHub did not return an annotated tag object')
    created = api.json('/git/refs', 'POST', {'ref': 'refs/tags/' + tag, 'sha': tag_object['sha']})
    if created.get('object', {}).get('sha') != tag_object['sha']:
        raise ValueError('GitHub did not create the expected tag reference')
    return tag_object['sha']


def asset_identity(asset: dict, version: str) -> tuple[str, str]:
    match = re.fullmatch(rf'{re.escape(version)}:([0-9a-f]{{40}}):([0-9a-f]{{64}})', asset.get('label') or '')
    if (not match or asset.get('state') != 'uploaded'
            or asset.get('digest') != 'sha256:' + (match[2] if match else '')):
        raise ValueError('Release asset identity is invalid')
    return match[1], match[2]


def release_assets(release: dict, version: str) -> tuple[dict, dict, str]:
    assets = release.get('assets', [])
    by_name = {asset.get('name'): asset for asset in assets}
    if len(assets) != 2 or set(by_name) != {ASSET, BUNDLE_ASSET}:
        raise ValueError('Release must contain the archive and its Cosign bundle')
    archive_commit, _ = asset_identity(by_name[ASSET], version)
    bundle_commit, _ = asset_identity(by_name[BUNDLE_ASSET], version)
    if bundle_commit != archive_commit:
        raise ValueError('Release assets disagree on source commit')
    return by_name[ASSET], by_name[BUNDLE_ASSET], archive_commit


def verify_published(api, release: dict, version: str) -> str:
    prerelease = parse_semver(version).group('prerelease') is not None
    if (release.get('draft') or release.get('immutable') is not True
            or release.get('tag_name') != 'v' + version
            or release.get('prerelease') != prerelease
            or MARKER not in release.get('body', '')):
        raise ValueError('Published release does not match the immutable release contract')
    archive_asset, bundle_asset, commit = release_assets(release, version)
    archive = download_asset(api, archive_asset)
    bundle = download_asset(api, bundle_asset)
    verify_blob(archive, bundle, ASSET, api.repository, 'release.yml')
    verify_signed_package(archive, version, commit, api.repository)
    ensure_tag(api, 'v' + version, commit, create=False)
    return commit


def download_asset(api, asset: dict) -> bytes:
    raw = api.download_release_asset(asset['id'])
    if (len(raw) != asset.get('size')
            or asset.get('digest') != 'sha256:' + hashlib.sha256(raw).hexdigest()):
        raise ValueError('Release asset download did not match GitHub metadata')
    return raw


def verify_signed_package(raw: bytes, version: str, commit: str, repository: str) -> dict:
    files = read_package(raw, FILES | INTERNAL_BUNDLES)
    metadata = verify_package_files(files, commit)
    if metadata.get('version') != version:
        raise ValueError('Signed package version does not match release')
    for name in ('codex-accounts-desktop.exe', 'SHA256SUMS'):
        verify_blob(files[name], files[name + '.sigstore.json'], name, repository, 'release.yml')
    return metadata


def upload_asset(api, release_id: int, name: str, raw: bytes, version: str, commit: str) -> dict:
    digest = hashlib.sha256(raw).hexdigest()
    asset = api.upload(release_id, name, raw, f'{version}:{commit}:{digest}')
    if (asset.get('name') != name or asset.get('state') != 'uploaded'
            or asset.get('size') != len(raw) or asset.get('digest') != 'sha256:' + digest):
        raise ValueError('Uploaded release asset verification failed')
    return asset


def release_body(api, version: str, commit: str, notes: str, run: dict | None) -> str:
    base = 'https://github.com/' + api.repository
    checks = '' if run is None else (
        f'\n\nChecks: [run {run["run_number"]}, attempt {run["run_attempt"]}]'
        f'({base}/actions/runs/{run["id"]})')
    return (f'{MARKER}\n\n{notes}\n\n'
            f'[Download for Windows]({base}/releases/download/v{version}/{ASSET})\n\n'
            'Windows x64. The executable is not Authenticode-signed, so Windows may show an '
            'unknown-publisher or SmartScreen warning. The archive and its contents are signed '
            'with Cosign and the release is immutable.\n\n'
            f'Source: [{commit}]({base}/tree/{commit})\n\n'
            f'[Verification instructions]({base}/blob/{commit}/docs/verify-download.md)'
            f'{checks}\n')


def finalize(api, release: dict, version: str, commit: str, notes: str,
             run: dict | None = None) -> None:
    tag = 'v' + version
    prerelease = parse_semver(version).group('prerelease') is not None
    existing_tag(api, tag, commit)
    ensure_tag(api, tag, commit)
    api.json(f"/releases/{release['id']}", 'PATCH', {
        'name': f'Codex Accounts {version}',
        'body': release_body(api, version, commit, notes, run),
        'draft': False,
        'prerelease': prerelease,
        'make_latest': 'false' if prerelease else 'true',
    })
    published = api.json('/releases/tags/' + urllib.parse.quote(tag, safe=''))
    if verify_published(api, published, version) != commit:
        raise ValueError('Published release source changed during publication')
    print(f'Published and verified immutable {tag} from {commit}')


def resume_draft(api, release: dict, version: str, notes: str) -> None:
    if MARKER not in release.get('body', ''):
        raise ValueError('Refusing to reuse an unmanaged draft')
    by_name = {asset.get('name'): asset for asset in release.get('assets', [])}
    if len(by_name) != len(release.get('assets', [])) or not set(by_name) <= {ASSET, BUNDLE_ASSET}:
        raise ValueError('Managed draft contains unexpected assets')
    archive_asset = by_name.get(ASSET)
    if archive_asset is None:
        raise ValueError('Managed draft cannot recover without its verified archive')
    commit, _ = asset_identity(archive_asset, version)
    existing_tag(api, 'v' + version, commit)
    raw = download_asset(api, archive_asset)
    metadata = verify_signed_package(raw, version, commit, api.repository)
    bundle_asset = by_name.get(BUNDLE_ASSET)
    if bundle_asset is None:
        bundle = sign_blob(raw, ASSET, api.repository, 'release.yml')
        upload_asset(api, release['id'], BUNDLE_ASSET, bundle, version, commit)
    else:
        bundle_commit, _ = asset_identity(bundle_asset, version)
        if bundle_commit != commit:
            raise ValueError('Managed draft assets disagree on source commit')
        bundle = download_asset(api, bundle_asset)
    verify_blob(raw, bundle, ASSET, api.repository, 'release.yml')
    run = {'id': int(metadata['workflow_run']), 'run_number': 'recorded', 'run_attempt': 'recorded'}
    finalize(api, release, version, commit, notes, run)


def publish_new(api, release: dict | None, run: dict, artifact: dict,
                version: str, notes: str) -> None:
    tag = 'v' + version
    existing_tag(api, tag, run['head_sha'])
    if release and MARKER not in release.get('body', ''):
        raise ValueError('Refusing to reuse an unmanaged draft')
    if release and release.get('assets'):
        resume_draft(api, release, version, notes)
        return
    if not release:
        release = api.json('/releases', 'POST', {
            'tag_name': tag,
            'target_commitish': run['head_sha'],
            'name': f'Codex Accounts {version}',
            'body': MARKER,
            'draft': True,
            'prerelease': parse_semver(version).group('prerelease') is not None,
            'make_latest': 'false',
        })
    raw = api.download(artifact['id'])
    metadata = verify_bundle(raw, run, artifact)
    if metadata['version'] != version:
        raise ValueError('Build version does not match release version')
    signed = sign_package(raw, api.repository, 'release.yml')
    upload_asset(api, release['id'], ASSET, signed, version, run['head_sha'])
    bundle = sign_blob(signed, ASSET, api.repository, 'release.yml')
    upload_asset(api, release['id'], BUNDLE_ASSET, bundle, version, run['head_sha'])
    finalize(api, release, version, run['head_sha'], notes, run)


def select_candidate(api, commit: str) -> tuple[dict, dict]:
    workflow = api.json('/actions/workflows/ci.yml')['id']
    runs = api.json('/actions/workflows/ci.yml/runs?branch=main&status=success&per_page=100')['workflow_runs']
    matches = [run for run in runs if eligible(run, api.repository, workflow) and run['head_sha'] == commit]
    if not matches:
        raise ValueError('Release source needs a successful main CI run')
    run = max(matches, key=lambda item: (item['run_number'], item['run_attempt']))
    artifacts = api.json(f"/actions/runs/{run['id']}/artifacts?per_page=100")['artifacts']
    name = f"codex-accounts-windows-x64-{run['run_number']}-{run['run_attempt']}"
    candidates = [artifact for artifact in artifacts if artifact.get('name') == name and not artifact.get('expired', True)]
    if len(candidates) != 1:
        raise ValueError('Release source needs one retained Windows artifact')
    return run, candidates[0]


def main() -> None:
    version = os.environ['RELEASE_VERSION']
    parse_semver(version)
    commit = os.environ['GITHUB_SHA']
    if (os.environ.get('GITHUB_ACTIONS') != 'true' or os.environ.get('GITHUB_REF') != 'refs/heads/main'
            or not re.fullmatch(r'[0-9a-f]{40}', commit)
            or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() != commit):
        raise ValueError('Release publication requires the exact main workflow checkout')
    api = GitHub()
    release = find_release(api, 'v' + version)
    if release and not release.get('draft'):
        source = verify_published(api, release, version)
        print(f'v{version} is already published, immutable and verified at {source}')
        return
    notes = changelog_notes(version)
    if release and release.get('assets'):
        resume_draft(api, release, version, notes)
        return
    release_tag(version)
    run, artifact = select_candidate(api, commit)
    publish_new(api, release, run, artifact, version, notes)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile,
            subprocess.SubprocessError) as error:
        raise SystemExit(f'Versioned release publication failed: {error}') from None
