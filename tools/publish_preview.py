"""Publish one Cosign-verifiable Preview; no artifact execution or Authenticode trust."""
from __future__ import annotations

import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import struct
import subprocess
import tempfile
import urllib.error
import urllib.parse
import urllib.request
import zipfile

from release_version import parse_semver
from security_tools import install

ASSET = 'codex-accounts-windows-x64.zip'
MARKER = '<!-- codex-accounts-managed-preview -->'
MAX_BYTES = 64 * 1024 * 1024
FILES = {'codex-accounts-desktop.exe', 'BUILD.json', 'Cargo.lock', 'SBOM.cdx.json',
         'SHA256SUMS', 'README.txt', 'LICENSE', 'source.tar.gz'}


def eligible(run: dict, repository: str, workflow: int) -> bool:
    return (run.get('workflow_id') == workflow and run.get('status') == 'completed'
            and run.get('conclusion') == 'success' and run.get('head_branch') == 'main'
            and run.get('event') in {'push', 'workflow_dispatch'}
            and run.get('head_repository', {}).get('full_name') == repository
            and re.fullmatch(r'[0-9a-f]{40}', run.get('head_sha', '')) is not None)


def read_package(raw: bytes, names: set[str]) -> dict[str, bytes]:
    if len(raw) > MAX_BYTES:
        raise ValueError('Package exceeded size bound')
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        entries = archive.infolist()
        if len(entries) != len(names) or {e.filename for e in entries} != names:
            raise ValueError('Unexpected package contents')
        if sum(e.file_size for e in entries) > MAX_BYTES or any(stat.S_ISLNK(e.external_attr >> 16) for e in entries):
            raise ValueError('Unsupported package size or links')
        return {name: archive.read(name) for name in names}


def verify_package_files(files: dict[str, bytes], commit: str, workflow_run: int | None = None) -> dict:
    sums = files['SHA256SUMS'].decode('utf-8').splitlines()
    expected = {name: hashlib.sha256(value).hexdigest() for name, value in files.items() if name != 'SHA256SUMS'}
    expected.pop('codex-accounts-desktop.exe.sigstore.json', None)
    expected.pop('SHA256SUMS.sigstore.json', None)
    if len(sums) != len(expected) or dict(reversed(line.split('  ', 1)) for line in sums) != expected:
        raise ValueError('Package checksum mismatch')
    meta = json.loads(files['BUILD.json'])
    recorded_run = meta.get('workflow_run')
    if (meta.get('source_commit') != commit
            or (workflow_run is not None and recorded_run != str(workflow_run))
            or (workflow_run is None and re.fullmatch(r'[1-9]\d*', recorded_run or '') is None)
            or not isinstance(meta.get('version'), str)
            or meta.get('platform') != 'windows-x86_64'
            or meta.get('payload_sha256') != expected['codex-accounts-desktop.exe']
            or meta.get('lock_sha256') != expected['Cargo.lock']):
        raise ValueError('Build identity mismatch')
    parse_semver(meta['version'])
    exe = files['codex-accounts-desktop.exe']
    if len(exe) < 64 or exe[:2] != b'MZ':
        raise ValueError('Missing Windows executable')
    offset = struct.unpack_from('<I', exe, 60)[0]
    if exe[offset:offset + 6] != b'PE\0\0\x64\x86':
        raise ValueError('Expected a Windows x64 executable')
    return meta


def verify_bundle(raw: bytes, run: dict, artifact: dict) -> dict:
    if artifact.get('expired', True) or artifact.get('name') != f"codex-accounts-windows-x64-{run['run_number']}-{run['run_attempt']}":
        raise ValueError('Unexpected build artifact')
    if artifact.get('digest') != 'sha256:' + hashlib.sha256(raw).hexdigest():
        raise ValueError('Artifact digest mismatch')
    return verify_package_files(read_package(raw, FILES), run['head_sha'], run['id'])


def rank(asset: dict) -> tuple[int, int, str, str]:
    match = re.fullmatch(r'(\d+):(\d+):([0-9a-f]{40}):([0-9a-f]{64})', asset.get('label') or '')
    if not match:
        raise ValueError('Unrecognized Preview asset identity')
    return int(match[1]), int(match[2]), match[3], match[4]


def workflow_identity(repository: str, workflow: str = 'artifact-retention.yml') -> str:
    if re.fullmatch(r'[A-Za-z0-9_.-]+\.yml', workflow) is None:
        raise ValueError('Invalid workflow identity')
    return f'https://github.com/{repository}/.github/workflows/{workflow}@refs/heads/main'


def signing_environment(repository: str, workflow: str) -> dict[str, str]:
    if (os.environ.get('GITHUB_ACTIONS') != 'true' or os.environ.get('GITHUB_REF') != 'refs/heads/main'
            or os.environ.get('GITHUB_WORKFLOW_REF') != workflow_identity(repository, workflow).removeprefix('https://github.com/')
            or not os.environ.get('ACTIONS_ID_TOKEN_REQUEST_URL')
            or not os.environ.get('ACTIONS_ID_TOKEN_REQUEST_TOKEN')):
        raise ValueError('Keyless publication requires the main GitHub workflow identity')
    environment = os.environ.copy()
    environment.pop('GH_TOKEN', None)
    environment.pop('GITHUB_TOKEN', None)
    return environment


def verify_blob(raw: bytes, bundle: bytes, name: str, repository: str,
                workflow: str = 'artifact-retention.yml') -> None:
    cosign = str(install('cosign'))
    with tempfile.TemporaryDirectory(prefix='codex-publish-') as folder:
        directory = Path(folder)
        environment = os.environ.copy()
        environment.pop('GH_TOKEN', None)
        environment.pop('GITHUB_TOKEN', None)
        environment['XDG_CACHE_HOME'] = str(directory / 'cache')
        payload = directory / name
        payload.write_bytes(raw)
        bundle_path = directory / (name + '.sigstore.json')
        bundle_path.write_bytes(bundle)
        result = subprocess.run(
            [cosign, 'verify-blob', '--bundle', str(bundle_path), '--certificate-identity',
             workflow_identity(repository, workflow), '--certificate-oidc-issuer',
             'https://token.actions.githubusercontent.com', str(payload)],
            env=environment, capture_output=True, timeout=180, check=False)
        if result.returncode:
            raise ValueError('Cosign signing or identity verification failed')


def sign_blob(raw: bytes, name: str, repository: str,
              workflow: str = 'artifact-retention.yml') -> bytes:
    environment = signing_environment(repository, workflow)
    cosign = str(install('cosign'))
    with tempfile.TemporaryDirectory(prefix='codex-publish-') as folder:
        directory = Path(folder)
        environment['XDG_CACHE_HOME'] = str(directory / 'cache')
        payload = directory / name
        payload.write_bytes(raw)
        bundle_path = directory / (name + '.sigstore.json')
        result = subprocess.run(
            [cosign, 'sign-blob', '--yes', '--oidc-provider=github-actions', '--bundle',
             str(bundle_path), str(payload)], env=environment, capture_output=True,
            timeout=180, check=False)
        if result.returncode or not bundle_path.is_file() or not 0 < bundle_path.stat().st_size <= 1024 * 1024:
            raise ValueError('Cosign signing or identity verification failed')
        bundle = bundle_path.read_bytes()
        verify_blob(raw, bundle, name, repository, workflow)
        return bundle


def sign_package(raw: bytes, repository: str, workflow: str = 'artifact-retention.yml') -> bytes:
    """Sign the executable and checksum manifest; keep both bundles inside one ZIP."""
    signing_environment(repository, workflow)
    files = read_package(raw, FILES)
    for name in ('codex-accounts-desktop.exe', 'SHA256SUMS'):
        files[name + '.sigstore.json'] = sign_blob(files[name], name, repository, workflow)
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(files.items()):
            archive.writestr(name, data)
    return output.getvalue()


def find_release(api) -> dict | None:
    release = api.json('/releases/tags/preview')
    if release:
        return release
    # Pending drafts have no published tag lookup. Recover a failed first publish
    # through the authenticated release inventory instead of creating another draft.
    matches = []
    for page in range(1, 11):
        entries = api.json(f'/releases?per_page=100&page={page}')
        matches.extend(r for r in entries if r.get('tag_name') == 'preview')
        if len(entries) < 100:
            break
    else:
        raise ValueError('Release inventory exceeds the recovery bound')
    if len(matches) > 1:
        raise ValueError('Ambiguous Preview release')
    return matches[0] if matches else None


def publish(api, run: dict, artifact: dict) -> None:
    raw = api.download(artifact['id'])
    meta = verify_bundle(raw, run, artifact)
    release = find_release(api)
    if release and MARKER not in release.get('body', ''):
        raise ValueError('Refusing to replace an unmanaged release')
    if not release:
        release = api.json('/releases', 'POST', {'tag_name': 'preview', 'target_commitish': 'main',
            'name': 'Preview', 'body': MARKER, 'draft': True, 'prerelease': True, 'make_latest': 'false'})
    release_id = release['id']
    assets = release.get('assets', [])
    current = next((a for a in assets if a['name'] == ASSET), None)
    wanted = (run['run_number'], run['run_attempt'], run['head_sha'], hashlib.sha256(raw).hexdigest())
    if current and rank(current)[:2] > wanted[:2]:
        print('Preview already has a newer successful build')
        return
    if current and rank(current)[:2] == wanted[:2] and rank(current) != wanted:
        raise ValueError('Conflicting Preview source identity')
    label = ':'.join(map(str, wanted))
    if current and rank(current) == wanted:
        if current.get('state') != 'uploaded' or not re.fullmatch(r'sha256:[0-9a-f]{64}', current.get('digest', '')):
            raise ValueError('Published asset is incomplete')
        candidate = current
    else:
        raw = sign_package(raw, api.repository)
        if len(raw) > MAX_BYTES:
            raise ValueError('Signed package exceeded the upload bound')
        digest = 'sha256:' + hashlib.sha256(raw).hexdigest()
        pending_name = f"candidate-{run['id']}-{run['run_attempt']}.zip"
        candidate = next((a for a in assets if a['name'] == pending_name), None)
        if candidate and rank(candidate) == wanted:
            # A failed upload may leave a starter asset. A fresh keyless signature
            # also changes ZIP bytes. Only retire this owned pending candidate.
            if candidate.get('state') == 'starter' or candidate.get('digest') != digest:
                api.json(f"/releases/assets/{candidate['id']}", 'DELETE')
                candidate = None
        if candidate is None:
            candidate = api.upload(release_id, pending_name, raw, label)
        if (candidate.get('state') != 'uploaded' or candidate.get('size') != len(raw)
                or candidate.get('digest') != digest or rank(candidate) != wanted):
            raise ValueError('Uploaded asset verification failed')
        # Upload and verify first. Retain the old bytes until promotion completes.
        if current:
            api.json(f"/releases/assets/{current['id']}", 'PATCH', {'name': f"previous-{current['id']}.zip"})
        try:
            candidate = api.json(f"/releases/assets/{candidate['id']}", 'PATCH', {'name': ASSET})
        except Exception:
            if current:
                api.json(f"/releases/assets/{current['id']}", 'PATCH', {'name': ASSET})
            raise
    base = 'https://github.com/' + api.repository
    body = (f'{MARKER}\n\nWindows x64 | {meta["version"]} | AGPL-3.0-only\n\n'
            f'[Download for Windows]({base}/releases/download/preview/{ASSET})\n\n'
            'No Windows Authenticode signature. Windows may show an unknown-publisher or SmartScreen warning. '
            'Cosign verifies the executable and checksum manifest using the publishing workflow identity; '
            'this does not establish Windows publisher trust.\n\n'
            'Extract the ZIP and run `codex-accounts-desktop.exe`. Official Codex Desktop and WebView2 are required.\n\n'
            'Includes checksums, Cosign verification bundles, build metadata, dependency inventory, AGPL license and the exact source archive. '
            'This is a rolling channel; the Preview tag is its original anchor, not the current binary revision. '
            'Use the source link below or the bundled `source.tar.gz` for this build.\n\n'
            f'Source: [{run["head_sha"]}]({base}/tree/{run["head_sha"]})\n\n'
            f'[Verification instructions]({base}/blob/{run["head_sha"]}/docs/verify-download.md)\n\n'
            f'Checks: [run {run["run_number"]}, attempt {run["run_attempt"]}]({base}/actions/runs/{run["id"]})\n')
    release = api.json(f'/releases/{release_id}', 'PATCH', {'name': 'Preview', 'body': body,
        'draft': False, 'prerelease': True, 'make_latest': 'false'})
    for old in release.get('assets', []):
        if old['id'] != candidate['id'] and re.fullmatch(r'(?:candidate-\d+-\d+|previous-\d+)\.zip', old['name']):
            rank(old)  # Never delete an unrelated asset that only matches the filename.
            api.json(f"/releases/assets/{old['id']}", 'DELETE')
    print(f"Published Cosign-verifiable Preview from {run['head_sha']}")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class GitHub:
    def __init__(self):
        self.repository = os.environ['GITHUB_REPOSITORY']
        if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', self.repository):
            raise ValueError('Invalid repository')
        self.base = 'https://api.github.com/repos/' + self.repository
        self.opener = urllib.request.build_opener(NoRedirect())

    def request(self, url, method='GET', data=None, content_type='application/json', authenticated=True,
                accept='application/vnd.github+json'):
        headers = {'Accept': accept, 'X-GitHub-Api-Version': '2026-03-10',
                   'Content-Type': content_type}
        if authenticated:
            headers['Authorization'] = 'Bearer ' + os.environ['GH_TOKEN']
        request = urllib.request.Request(url, data=data, method=method, headers=headers)
        with self.opener.open(request, timeout=60) as response:
            raw = response.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError('GitHub response exceeded size bound')
        return raw

    def json(self, path, method='GET', data=None):
        try:
            raw = self.request(self.base + path, method, None if data is None else json.dumps(data).encode())
            return json.loads(raw) if raw else None
        except urllib.error.HTTPError as error:
            if error.code == 404 and path == '/releases/tags/preview' and method == 'GET':
                return None
            raise

    def optional_json(self, path):
        try:
            return self.json(path)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            raise

    def _download(self, path):
        try:
            return self.request(self.base + path, accept='application/octet-stream')
        except urllib.error.HTTPError as error:
            if error.code != 302:
                raise
            location = error.headers['Location']
            parsed = urllib.parse.urlsplit(location)
            if (parsed.scheme != 'https' or parsed.username or parsed.password or parsed.port not in (None, 443)
                    or not (parsed.hostname or '').endswith(('.blob.core.windows.net', '.githubusercontent.com'))):
                raise ValueError('Unexpected artifact download host') from None
            # The signed storage URL receives no GitHub token, and cannot redirect again.
            return self.request(location, authenticated=False)

    def download(self, identifier):
        return self._download(f'/actions/artifacts/{int(identifier)}/zip')

    def download_release_asset(self, identifier):
        return self._download(f'/releases/assets/{int(identifier)}')

    def upload(self, release_id, name, raw, label):
        query = urllib.parse.urlencode({'name': name, 'label': label})
        url = f'https://uploads.github.com/repos/{self.repository}/releases/{release_id}/assets?{query}'
        return json.loads(self.request(url, 'POST', raw, 'application/zip'))


def main():
    api = GitHub()
    workflow = api.json('/actions/workflows/ci.yml')['id']
    runs = api.json('/actions/workflows/ci.yml/runs?branch=main&status=success&per_page=100')['workflow_runs']
    good = [r for r in runs if eligible(r, api.repository, workflow)]
    if not good:
        print('No successful main build to publish')
        return
    run = max(good, key=lambda r: (r['run_number'], r['run_attempt']))
    artifacts = api.json(f"/actions/runs/{run['id']}/artifacts?per_page=100")['artifacts']
    name = f"codex-accounts-windows-x64-{run['run_number']}-{run['run_attempt']}"
    matches = [a for a in artifacts if a['name'] == name and not a['expired']]
    if not matches:
        print('Successful main build has no retained Preview package yet')
        return
    if len(matches) != 1:
        raise ValueError('Ambiguous build artifact')
    publish(api, run, matches[0])


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile, subprocess.SubprocessError):
        raise SystemExit('Preview publication failed; inspect the workflow step and retained assets') from None
