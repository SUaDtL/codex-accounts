"""In-place specification migration regressions; no qualification authority."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from tools import spec_proposal

ROOT = Path(__file__).resolve().parents[2]


class InPlaceProvenanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for path in [spec_proposal.SPEC_PATH, 'docs/source-inputs.json',
                     *spec_proposal.UNCHANGED]:
            destination = self.root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / path, destination)
        self.manifest = json.loads((self.root / 'docs/source-inputs.json').read_text())

    def write_manifest(self, manifest: dict) -> None:
        (self.root / 'docs/source-inputs.json').write_text(json.dumps(manifest), encoding='utf-8')

    def test_checked_in_candidate_and_manifest_agree(self) -> None:
        spec_proposal.check_candidate(self.root)
        candidate = (self.root / spec_proposal.SPEC_PATH).read_text(encoding='utf-8')
        original = spec_proposal.reconstruct_original(candidate)
        self.assertEqual(hashlib.sha256(original.encode()).hexdigest(),
                         spec_proposal.ORIGINAL_SHA256)
        self.assertEqual(spec_proposal.render(original), candidate)

    def test_v1_manifest_cannot_authorize_the_in_place_candidate(self) -> None:
        self.write_manifest(spec_proposal.original_manifest())
        with self.assertRaisesRegex(ValueError, 'v2 provenance'):
            spec_proposal.check_candidate(self.root)

    def test_original_digest_cannot_be_changed_or_omitted(self) -> None:
        for value in ['0' * 64, None]:
            with self.subTest(value=value):
                manifest = json.loads(json.dumps(self.manifest))
                entry = manifest['files'][0]
                if value is None:
                    del entry['original_sha256']
                else:
                    entry['original_sha256'] = value
                self.write_manifest(manifest)
                with self.assertRaisesRegex(ValueError, 'provenance migration'):
                    spec_proposal.check_candidate(self.root)

    def test_revision_and_amendment_reference_are_pinned(self) -> None:
        for field in ['revision', 'amendment']:
            with self.subTest(field=field):
                manifest = json.loads(json.dumps(self.manifest))
                manifest['files'][0][field] = 'UNREVIEWED'
                self.write_manifest(manifest)
                with self.assertRaisesRegex(ValueError, 'provenance migration'):
                    spec_proposal.check_candidate(self.root)

    def test_research_drift_cannot_be_hidden_by_updating_its_digest(self) -> None:
        entry = self.manifest['files'][1]
        path = self.root / entry['path']
        path.write_bytes(path.read_bytes() + b'UNREVIEWED')
        entry['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        self.write_manifest(self.manifest)
        with self.assertRaisesRegex(ValueError, 'provenance migration'):
            spec_proposal.check_candidate(self.root)

    def test_spec_drift_cannot_be_hidden_by_updating_its_digest(self) -> None:
        path = self.root / spec_proposal.SPEC_PATH
        path.write_bytes(path.read_bytes() + b'UNREVIEWED')
        self.manifest['files'][0]['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        self.write_manifest(self.manifest)
        with self.assertRaisesRegex(ValueError, 'reviewed T-07 candidate'):
            spec_proposal.check_candidate(self.root)

    def test_missing_or_extra_manifest_entries_are_rejected(self) -> None:
        for added in [False, True]:
            with self.subTest(added=added):
                manifest = json.loads(json.dumps(self.manifest))
                if added:
                    manifest['files'].append({'path': 'UNREVIEWED', 'sha256': '0' * 64})
                else:
                    manifest['files'].pop()
                self.write_manifest(manifest)
                with self.assertRaisesRegex(ValueError, 'provenance migration'):
                    spec_proposal.check_candidate(self.root)


if __name__ == '__main__':
    unittest.main()
