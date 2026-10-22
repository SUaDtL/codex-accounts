"""Consumer delivery must preserve the last good download on failed replacement."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
import publish_preview as preview


def bundle(run):
    exe = bytearray(96)
    exe[:2] = b'MZ'
    exe[60:64] = (64).to_bytes(4, 'little')
    exe[64:70] = b'PE\0\0\x64\x86'
    files = {'codex-accounts-desktop.exe': bytes(exe), 'Cargo.lock': b'synthetic lock',
             'LICENSE': b'GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3',
             'source.tar.gz': b'synthetic source archive', 'README.txt': b'Unsigned preview',
             'SBOM.cdx.json': json.dumps({'bomFormat': 'CycloneDX', 'specVersion': '1.6'}).encode()}
    files['BUILD.json'] = json.dumps({'version': '0.1.0', 'source_commit': run['head_sha'],
        'workflow_run': str(run['id']), 'platform': 'windows-x86_64',
        'payload_sha256': hashlib.sha256(files['codex-accounts-desktop.exe']).hexdigest(),
        'lock_sha256': hashlib.sha256(files['Cargo.lock']).hexdigest()}).encode()
    files['SHA256SUMS'] = ''.join(f'{hashlib.sha256(data).hexdigest()}  {name}\n'
                               for name, data in sorted(files.items())).encode()
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w') as archive:
        for name, data in files.items():
            archive.writestr(name, data)
    return output.getvalue()


class FakeAPI:
    repository = 'owner/repo'

    def __init__(self, raw):
        self.raw = raw
        self.release = {'id': 7, 'tag_name': 'preview', 'draft': False,
                        'body': preview.MARKER, 'assets': []}
        self.fail_upload = False
        self.fail_promote = False
        self.created = 0

    def download(self, identifier):
        return self.raw

    def json(self, path, method='GET', data=None):
        if path == '/releases/tags/preview':
            return copy.deepcopy(self.release) if self.release and not self.release['draft'] else None
        if path.startswith('/releases?'):
            return [copy.deepcopy(self.release)] if self.release else []
        if path == '/releases' and method == 'POST':
            self.created += 1
            self.release = dict(data, id=7, assets=[])
            return copy.deepcopy(self.release)
        if path == '/releases/7' and method == 'PATCH':
            self.release.update(data)
            return copy.deepcopy(self.release)
        identifier = int(path.rsplit('/', 1)[1])
        asset = next(a for a in self.release['assets'] if a['id'] == identifier)
        if method == 'DELETE':
            self.release['assets'].remove(asset)
            return None
        if method == 'PATCH':
            if self.fail_promote and asset['name'].startswith('candidate-'):
                raise RuntimeError('synthetic promotion failure')
            asset.update(data)
        return copy.deepcopy(asset)

    def upload(self, release_id, name, raw, label):
        if self.fail_upload:
            raise RuntimeError('synthetic upload failure')
        asset = {'id': 100 + len(self.release['assets']), 'name': name, 'label': label,
                 'state': 'uploaded', 'size': len(raw), 'digest': 'sha256:' + hashlib.sha256(raw).hexdigest()}
        self.release['assets'].append(asset)
        return copy.deepcopy(asset)


class PreviewTests(unittest.TestCase):
    def setUp(self):
        self.run = {'id': 20, 'run_number': 20, 'run_attempt': 1, 'head_sha': 'a' * 40,
                    'head_branch': 'main', 'event': 'push', 'status': 'completed',
                    'conclusion': 'success', 'workflow_id': 8,
                    'head_repository': {'full_name': 'owner/repo'}}
        self.raw = bundle(self.run)
        self.artifact = {'id': 30, 'name': 'codex-accounts-windows-x64-20-1', 'expired': False,
                         'digest': 'sha256:' + hashlib.sha256(self.raw).hexdigest()}
        self.api = FakeAPI(self.raw)
        signer = patch.object(preview, 'sign_package', side_effect=lambda raw, repository: raw)
        self.signer = signer.start()
        self.addCleanup(signer.stop)
        self.old = {'id': 1, 'name': preview.ASSET, 'label': '19:1:' + 'b' * 40 + ':' + 'c' * 64}
        self.api.release['assets'].append(self.old)

    def test_only_successful_owner_main_runs_are_eligible(self):
        self.assertTrue(preview.eligible(self.run, 'owner/repo', 8))
        for key, value in [('event', 'pull_request'), ('head_branch', 'feature'),
                           ('status', 'in_progress'), ('conclusion', 'failure'),
                           ('workflow_id', 99), ('head_repository', {'full_name': 'fork/repo'})]:
            with self.subTest(key=key):
                self.assertFalse(preview.eligible(dict(self.run, **{key: value}), 'owner/repo', 8))

    def test_bundle_identity_checksums_and_flat_paths(self):
        preview.verify_bundle(self.raw, self.run, self.artifact)
        for run in [dict(self.run, head_sha='b' * 40), dict(self.run, id=21)]:
            with self.assertRaises(ValueError):
                preview.verify_bundle(self.raw, run, self.artifact)
        damaged = self.raw + b'tampered'
        with self.assertRaises(ValueError):
            preview.verify_bundle(damaged, self.run, self.artifact)
        output = io.BytesIO(self.raw)
        with zipfile.ZipFile(output, 'a') as archive:
            archive.writestr('../escape', b'bad')
        raw = output.getvalue()
        with self.assertRaises(ValueError):
            preview.verify_bundle(raw, self.run, dict(self.artifact, digest='sha256:' + hashlib.sha256(raw).hexdigest()))

    def test_failed_upload_leaves_previous_download_unchanged(self):
        self.api.fail_upload = True
        with self.assertRaises(RuntimeError):
            preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(self.api.release['assets'], [self.old])

    def test_failed_promotion_restores_previous_download(self):
        self.api.fail_promote = True
        with self.assertRaises(RuntimeError):
            preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(self.old['name'], preview.ASSET)

    def test_success_replaces_old_asset_and_retry_is_idempotent(self):
        preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(len(self.api.release['assets']), 1)
        asset = copy.deepcopy(self.api.release['assets'][0])
        self.assertEqual(asset['name'], preview.ASSET)
        self.assertIn('a' * 40, self.api.release['body'])
        preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(self.api.release['assets'], [asset])

    def test_late_run_and_foreign_release_cannot_overwrite(self):
        self.old['label'] = '21:1:' + 'b' * 40 + ':' + 'c' * 64
        preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(self.api.release['assets'], [self.old])
        self.api.release['body'] = 'Unrelated manually maintained release'
        with self.assertRaises(ValueError):
            preview.publish(self.api, self.run, self.artifact)

    def test_starter_left_by_failed_upload_is_recovered(self):
        self.api.release['assets'].append({'id': 2, 'name': 'candidate-20-1.zip',
            'label': '20:1:' + 'a' * 40 + ':' + hashlib.sha256(self.raw).hexdigest(),
            'state': 'starter', 'size': 0})
        preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(len(self.api.release['assets']), 1)
        self.assertEqual(self.api.release['assets'][0]['name'], preview.ASSET)

    def test_signing_failure_never_retires_current_download(self):
        self.signer.side_effect = ValueError('synthetic signing failure')
        with self.assertRaises(ValueError):
            preview.publish(self.api, self.run, self.artifact)
        self.assertEqual(self.api.release['assets'], [self.old])

    def test_failed_first_publish_recovers_existing_draft(self):
        self.api.release = None
        self.api.fail_upload = True
        with self.assertRaises(RuntimeError):
            preview.publish(self.api, self.run, self.artifact)
        self.assertTrue(self.api.release['draft'])
        self.api.fail_upload = False
        preview.publish(self.api, self.run, self.artifact)
        self.assertFalse(self.api.release['draft'])
        self.assertEqual(self.api.created, 1)


class SigningTests(unittest.TestCase):
    def test_local_or_nonmain_context_refuses_before_tool_acquisition(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(preview, 'install') as install:
            with self.assertRaises(ValueError):
                preview.sign_package(b'', 'owner/repo')
            install.assert_not_called()

    def test_both_blobs_require_exact_identity_verification_and_token_is_scrubbed(self):
        run = {'id': 20, 'head_sha': 'a' * 40}
        environment = {'GITHUB_ACTIONS': 'true', 'GITHUB_REF': 'refs/heads/main',
            'GITHUB_WORKFLOW_REF': 'owner/repo/.github/workflows/artifact-retention.yml@refs/heads/main',
            'ACTIONS_ID_TOKEN_REQUEST_URL': 'https://example.invalid/synthetic-oidc',
            'ACTIONS_ID_TOKEN_REQUEST_TOKEN': 'synthetic-test-only', 'GH_TOKEN': 'synthetic-test-only'}
        commands = []

        def invoke(command, **kwargs):
            commands.append(command)
            self.assertNotIn('GH_TOKEN', kwargs['env'])
            self.assertTrue(kwargs['capture_output'])
            if command[1] == 'sign-blob':
                self.assertIn('--oidc-provider=github-actions', command)
                Path(command[command.index('--bundle') + 1]).write_text('{}')
            else:
                self.assertEqual(command[command.index('--certificate-identity') + 1], preview.workflow_identity('owner/repo'))
                self.assertEqual(command[command.index('--certificate-oidc-issuer') + 1], 'https://token.actions.githubusercontent.com')
            return SimpleNamespace(returncode=0)

        with patch.dict(os.environ, environment, clear=True), patch.object(preview, 'install', return_value='cosign-fixture'), patch.object(preview.subprocess, 'run', side_effect=invoke):
            output = preview.sign_package(bundle(run), 'owner/repo')
        self.assertEqual([c[1] for c in commands], ['sign-blob', 'verify-blob', 'sign-blob', 'verify-blob'])
        with zipfile.ZipFile(io.BytesIO(output)) as archive:
            self.assertEqual(set(archive.namelist()), preview.FILES | {'SHA256SUMS.sigstore.json', 'codex-accounts-desktop.exe.sigstore.json'})
        with patch.dict(os.environ, environment, clear=True), patch.object(preview, 'install', return_value='cosign-fixture'), patch.object(preview.subprocess, 'run', return_value=SimpleNamespace(returncode=1)):
            with self.assertRaisesRegex(ValueError, 'Cosign signing or identity verification failed'):
                preview.sign_package(bundle(run), 'owner/repo')


if __name__ == '__main__':
    unittest.main()
