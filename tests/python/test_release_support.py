"""Release signing and GitHub transport regressions."""
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
import release_support as support


def bundle(run):
    exe = bytearray(96)
    exe[:2] = b'MZ'
    exe[60:64] = (64).to_bytes(4, 'little')
    exe[64:70] = b'PE\0\0\x64\x86'
    files = {'codex-accounts-desktop.exe': bytes(exe), 'Cargo.lock': b'synthetic lock',
             'LICENSE': b'GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3',
             'source.tar.gz': b'synthetic source archive', 'README.txt': b'Unsigned release',
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


class GitHubDownloadTests(unittest.TestCase):
    def test_artifact_and_release_asset_use_endpoint_specific_media_types(self):
        api = support.GitHub.__new__(support.GitHub)
        api.repository = 'owner/repo'
        api.base = 'https://api.github.com/repos/owner/repo'
        calls = []

        def request(url, **kwargs):
            calls.append((url, kwargs))
            return b'payload'

        api.request = request
        self.assertEqual(api.download(42), b'payload')
        self.assertEqual(api.download_release_asset(43), b'payload')
        self.assertEqual(calls[0][1]['accept'], 'application/vnd.github+json')
        self.assertEqual(calls[1][1]['accept'], 'application/octet-stream')


class SigningTests(unittest.TestCase):
    def test_local_or_nonmain_context_refuses_before_tool_acquisition(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(support, 'install') as install:
            with self.assertRaises(ValueError):
                support.sign_package(b'', 'owner/repo')
            install.assert_not_called()

    def test_both_blobs_require_exact_identity_verification_and_token_is_scrubbed(self):
        run = {'id': 20, 'head_sha': 'a' * 40}
        environment = {'GITHUB_ACTIONS': 'true', 'GITHUB_REF': 'refs/heads/main',
            'GITHUB_WORKFLOW_REF': 'owner/repo/.github/workflows/release.yml@refs/heads/main',
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
                self.assertEqual(command[command.index('--certificate-identity') + 1], support.workflow_identity('owner/repo'))
                self.assertEqual(command[command.index('--certificate-oidc-issuer') + 1], 'https://token.actions.githubusercontent.com')
            return SimpleNamespace(returncode=0)

        with patch.dict(os.environ, environment, clear=True), patch.object(support, 'install', return_value='cosign-fixture'), patch.object(support.subprocess, 'run', side_effect=invoke):
            output = support.sign_package(bundle(run), 'owner/repo')
        self.assertEqual([c[1] for c in commands], ['sign-blob', 'verify-blob', 'sign-blob', 'verify-blob'])
        with zipfile.ZipFile(io.BytesIO(output)) as archive:
            self.assertEqual(set(archive.namelist()), support.FILES | {'SHA256SUMS.sigstore.json', 'codex-accounts-desktop.exe.sigstore.json'})
        with patch.dict(os.environ, environment, clear=True), patch.object(support, 'install', return_value='cosign-fixture'), patch.object(support.subprocess, 'run', return_value=SimpleNamespace(returncode=1)):
            with self.assertRaisesRegex(ValueError, 'Cosign signing or identity verification failed'):
                support.sign_package(bundle(run), 'owner/repo')


if __name__ == '__main__':
    unittest.main()
