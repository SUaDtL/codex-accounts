"""Versioned releases are exact-source, immutable and recoverable before publication."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
import publish_release as release


def bundle(run, version='1.0.0-rc.1'):
    executable = bytearray(96)
    executable[:2] = b'MZ'
    executable[60:64] = (64).to_bytes(4, 'little')
    executable[64:70] = b'PE\0\0\x64\x86'
    files = {'codex-accounts-desktop.exe': bytes(executable), 'Cargo.lock': b'lock',
             'LICENSE': b'AGPL', 'source.tar.gz': b'source', 'README.txt': b'readme',
             'SBOM.cdx.json': b'{}'}
    files['BUILD.json'] = json.dumps({'version': version, 'source_commit': run['head_sha'],
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


def signed_bundle(raw):
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(raw)) as source, zipfile.ZipFile(output, 'w') as target:
        for entry in source.infolist():
            target.writestr(entry.filename, source.read(entry.filename))
        target.writestr('codex-accounts-desktop.exe.sigstore.json', b'exe-bundle')
        target.writestr('SHA256SUMS.sigstore.json', b'sums-bundle')
    return output.getvalue()


class FakeAPI:
    repository = 'owner/repo'

    def __init__(self, raw):
        self.raw = raw
        self.release = None
        self.tag_object = None
        self.created_releases = 0
        self.uploads = 0
        self.asset_bytes = {}

    def download(self, identifier):
        return self.raw

    def download_release_asset(self, identifier):
        return self.asset_bytes[identifier]

    def optional_json(self, path):
        if path.startswith('/releases/tags/'):
            return copy.deepcopy(self.release) if self.release and not self.release['draft'] else None
        if path.startswith('/git/ref/tags/'):
            return None if self.tag_object is None else {'object': {'type': 'tag', 'sha': self.tag_object['sha']}}
        raise AssertionError(path)

    def json(self, path, method='GET', data=None):
        if path.startswith('/releases?'):
            return [] if self.release is None else [copy.deepcopy(self.release)]
        if path == '/releases' and method == 'POST':
            self.created_releases += 1
            self.release = dict(data, id=7, assets=[], immutable=False)
            return copy.deepcopy(self.release)
        if path == '/releases/7' and method == 'PATCH':
            self.release.update(data)
            if not self.release['draft']:
                self.release['immutable'] = True
            return copy.deepcopy(self.release)
        if path.startswith('/releases/tags/'):
            return copy.deepcopy(self.release)
        if path == '/git/tags' and method == 'POST':
            self.tag_object = {'sha': 'b' * 40, 'tag': data['tag'], 'message': data['message'],
                               'object': {'type': data['type'], 'sha': data['object']}}
            return copy.deepcopy(self.tag_object)
        if path == '/git/refs' and method == 'POST':
            return {'object': {'sha': data['sha']}}
        if path == '/git/tags/' + 'b' * 40:
            return copy.deepcopy(self.tag_object)
        raise AssertionError((path, method, data))

    def upload(self, release_id, name, raw, label):
        self.uploads += 1
        identifier = 10 + self.uploads
        asset = {'id': identifier, 'name': name, 'label': label, 'state': 'uploaded',
                 'size': len(raw), 'digest': 'sha256:' + hashlib.sha256(raw).hexdigest()}
        self.asset_bytes[identifier] = raw
        self.release['assets'].append(asset)
        return copy.deepcopy(asset)


class VersionedReleaseTests(unittest.TestCase):
    def setUp(self):
        self.run = {'id': 20, 'run_number': 8, 'run_attempt': 1, 'head_sha': 'a' * 40}
        self.raw = bundle(self.run)
        self.signed = signed_bundle(self.raw)
        self.artifact = {'id': 30, 'name': 'codex-accounts-windows-x64-8-1', 'expired': False,
                         'digest': 'sha256:' + hashlib.sha256(self.raw).hexdigest()}
        self.api = FakeAPI(self.raw)
        patches = [
            patch.object(release, 'sign_package', return_value=self.signed),
            patch.object(release, 'sign_blob', return_value=b'whole-bundle'),
            patch.object(release, 'verify_blob'),
        ]
        self.mocks = [item.start() for item in patches]
        for item in patches:
            self.addCleanup(item.stop)

    def publish(self):
        release.publish_new(self.api, None, self.run, self.artifact, '1.0.0-rc.1',
                            '## [1.0.0-rc.1]\n\n- Ready.')

    def test_manifest_changelog_and_strict_semver_bind_release(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'crates/desktop-app').mkdir(parents=True)
            (root / 'crates/shell-preparation').mkdir(parents=True)
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion = "1.0.0-rc.1"\n')
            (root / 'crates/desktop-app/tauri.conf.json').write_text('{"version":"1.0.0-rc.1"}')
            (root / 'crates/shell-preparation/tauri.conf.json').write_text('{"version":"1.0.0-rc.1"}')
            (root / 'package.json').write_text('{"version":"1.0.0-rc.1"}')
            (root / 'package-lock.json').write_text('{"packages":{"":{"version":"1.0.0-rc.1"}}}')
            (root / 'CHANGELOG.md').write_text('# Changelog\n\n## [1.0.0-rc.1] - 2026-10-23\n\n- First candidate.\n')
            self.assertEqual(release.release_tag('1.0.0-rc.1', root), 'v1.0.0-rc.1')
            self.assertIn('First candidate', release.changelog_notes('1.0.0-rc.1', root))
            for invalid in ('1.0.0', '1.0.0-..', '1.0.0-01', '01.0.0'):
                with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                    release.release_tag(invalid, root)

    def test_publish_creates_annotated_tag_two_assets_and_immutable_prerelease(self):
        self.publish()
        self.assertFalse(self.api.release['draft'])
        self.assertTrue(self.api.release['immutable'])
        self.assertTrue(self.api.release['prerelease'])
        self.assertEqual({a['name'] for a in self.api.release['assets']},
                         {release.ASSET, release.BUNDLE_ASSET})
        self.assertEqual(self.api.tag_object['object']['sha'], self.run['head_sha'])
        self.mocks[1].assert_called_once_with(
            self.signed, release.ASSET, 'owner/repo', 'release.yml')

    def test_retry_after_publication_needs_no_ci_artifact(self):
        self.publish()
        self.api.raw = None
        source = release.verify_published(self.api, self.api.release, '1.0.0-rc.1')
        self.assertEqual(source, self.run['head_sha'])

    def test_published_release_rejects_unavailable_or_changed_asset_bytes(self):
        self.publish()
        archive = next(a for a in self.api.release['assets'] if a['name'] == release.ASSET)
        original = self.api.asset_bytes.pop(archive['id'])
        with self.assertRaises(KeyError):
            release.verify_published(self.api, self.api.release, '1.0.0-rc.1')
        self.api.asset_bytes[archive['id']] = original + b'tampered'
        with self.assertRaises(ValueError):
            release.verify_published(self.api, self.api.release, '1.0.0-rc.1')

    def test_main_rerun_verifies_published_release_before_candidate_lookup(self):
        self.publish()
        environment = {'RELEASE_VERSION': '1.0.0-rc.1', 'GITHUB_SHA': self.run['head_sha'],
                       'GITHUB_ACTIONS': 'true', 'GITHUB_REF': 'refs/heads/main'}
        with (patch.dict(os.environ, environment, clear=True),
              patch.object(release, 'GitHub', return_value=self.api),
              patch.object(release.subprocess, 'check_output', return_value=self.run['head_sha'] + '\n'),
              patch.object(release, 'select_candidate', side_effect=AssertionError('candidate lookup'))):
            release.main()

    def test_partial_managed_draft_recovers_without_ci_artifact(self):
        self.publish()
        self.api.release['draft'] = True
        self.api.release['immutable'] = False
        bundle = next(a for a in self.api.release['assets'] if a['name'] == release.BUNDLE_ASSET)
        self.api.release['assets'].remove(bundle)
        self.api.asset_bytes.pop(bundle['id'])
        self.api.raw = None
        self.mocks[1].reset_mock()
        self.mocks[2].reset_mock()
        release.resume_draft(self.api, self.api.release, '1.0.0-rc.1',
                             '## [1.0.0-rc.1]\n\n- Ready.')
        self.assertFalse(self.api.release['draft'])
        self.assertTrue(self.api.release['immutable'])
        self.assertEqual(self.api.created_releases, 1)
        self.mocks[1].assert_called_once_with(
            self.signed, release.ASSET, 'owner/repo', 'release.yml')
        self.mocks[2].assert_any_call(
            self.signed, b'whole-bundle', release.ASSET, 'owner/repo', 'release.yml')

    def test_unmanaged_draft_and_conflicting_tag_are_refused_before_mutation(self):
        self.api.release = {'id': 7, 'tag_name': 'v1.0.0-rc.1', 'draft': True,
                            'body': 'foreign draft', 'assets': []}
        with self.assertRaises(ValueError):
            release.publish_new(self.api, self.api.release, self.run, self.artifact,
                                '1.0.0-rc.1', 'notes')
        self.api.release = None
        self.api.tag_object = {'sha': 'b' * 40, 'tag': 'v1.0.0-rc.1',
                               'object': {'type': 'commit', 'sha': 'c' * 40}}
        with self.assertRaises(ValueError):
            release.publish_new(self.api, None, self.run, self.artifact,
                                '1.0.0-rc.1', 'notes')
        self.assertEqual(self.api.created_releases, 0)
        self.assertEqual(self.api.uploads, 0)

    def test_published_release_requires_tag_and_immutability(self):
        self.publish()
        self.api.tag_object = None
        with self.assertRaises(ValueError):
            release.verify_published(self.api, self.api.release, '1.0.0-rc.1')
        self.api.release['immutable'] = False
        with self.assertRaises(ValueError):
            release.verify_published(self.api, self.api.release, '1.0.0-rc.1')

    def test_build_version_must_match_release(self):
        bad = bundle(self.run, '1.0.0-rc.2')
        self.api.raw = bad
        artifact = dict(self.artifact, digest='sha256:' + hashlib.sha256(bad).hexdigest())
        with self.assertRaises(ValueError):
            release.publish_new(self.api, None, self.run, artifact, '1.0.0-rc.1', 'notes')

    def test_candidate_selection_is_exact_and_unambiguous(self):
        older = dict(self.run, id=19, run_number=7, workflow_id=4, status='completed',
                     conclusion='success', head_branch='main', event='push',
                     head_repository={'full_name': 'owner/repo'})
        newer = dict(older, id=20, run_number=8, run_attempt=2)
        wrong_sha = dict(newer, id=21, run_number=99, head_sha='d' * 40)
        artifact = dict(self.artifact, name='codex-accounts-windows-x64-8-2')

        class CandidateAPI:
            repository = 'owner/repo'

            def __init__(self, runs, artifacts):
                self.runs = runs
                self.artifacts = artifacts

            def json(self, path):
                if path == '/actions/workflows/ci.yml':
                    return {'id': 4}
                if path.startswith('/actions/workflows/ci.yml/runs?'):
                    return {'workflow_runs': self.runs}
                if path.startswith('/actions/runs/'):
                    return {'artifacts': self.artifacts}
                raise AssertionError(path)

        selected, selected_artifact = release.select_candidate(
            CandidateAPI([older, newer, wrong_sha], [artifact]), self.run['head_sha'])
        self.assertEqual((selected['id'], selected_artifact['id']), (20, 30))
        for runs, artifacts in [([], [artifact]), ([newer], []),
                                ([wrong_sha], [artifact]),
                                ([newer], [artifact, dict(artifact, id=31)]),
                                ([newer], [dict(artifact, expired=True)])]:
            with self.subTest(runs=len(runs), artifacts=len(artifacts)), self.assertRaises(ValueError):
                release.select_candidate(CandidateAPI(runs, artifacts), self.run['head_sha'])


if __name__ == '__main__':
    unittest.main()
