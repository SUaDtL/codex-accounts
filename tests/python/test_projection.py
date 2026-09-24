"""Document migration tests; none are Desktop qualification evidence."""
from __future__ import annotations
import copy
import hashlib
from html import escape
import json
from pathlib import Path
import tempfile
import unittest
from tools import spec_index, spec_projection, spec_proposal, plan_projection

ROOT = Path(__file__).resolve().parents[2]


class ProjectionTests(unittest.TestCase):
    def setUp(self):
        self.current = spec_index.SPEC.read_text(encoding='utf-8')
        self.html = spec_proposal.render(self.current)
        self.model = json.loads(spec_proposal.MODEL.search(self.html).group(2))

    def test_every_normative_section_and_source_agrees(self):
        spec_projection.check(self.html, self.model)

    def test_proposal_is_idempotent(self):
        self.assertEqual(spec_proposal.render(self.html), self.html)

    def test_original_reconstructs_byte_for_byte_and_reapplies(self):
        data = copy.deepcopy(self.model)
        data.pop('document_revision')
        data.pop('amendment_status')
        rows = next(b['rows'] for s in data['sections'] if s['id'] == 'SPEC-TESTS'
                    for b in s['blocks'] if b['type'] == 'table')
        next(r for r in rows if r[0] == 'T-07')[1] = spec_proposal.OLD
        original = spec_proposal.MODEL.sub(
            lambda m: m[1] + json.dumps(data, ensure_ascii=False, separators=(',', ':')) + m[3], self.html)
        original = spec_proposal.ROW.sub(
            lambda m: m[1] + escape(spec_proposal.OLD) + m[3], original)
        self.assertEqual(hashlib.sha256(original.encode()).hexdigest(), spec_proposal.ORIGINAL_SHA256)
        self.assertEqual(spec_proposal.render(original), self.html)

    def test_model_only_drift_fails(self):
        data = copy.deepcopy(self.model)
        data['sections'][0]['blocks'][0]['text'] += ' UNAUTHORIZED'
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            spec_projection.check(self.html, data)

    def test_visible_only_drift_fails(self):
        html = self.html.replace('No inference or routing</h3>', 'UNAUTHORIZED</h3>', 1)
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            spec_projection.check(html, self.model)

    def test_source_note_drift_fails(self):
        data = copy.deepcopy(self.model)
        data['sources'][0]['note'] += ' UNAUTHORIZED'
        with self.assertRaisesRegex(ValueError, 'mismatch'):
            spec_projection.check(self.html, data)

    def test_duplicate_projection_fails(self):
        block = '<section data-panel id="SPEC-OVERVIEW"></section>'
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            spec_projection.check(self.html + block, self.model)

    def test_unknown_block_type_fails(self):
        data = copy.deepcopy(self.model)
        data['sections'][0]['blocks'][0]['type'] = 'unknown'
        with self.assertRaisesRegex(ValueError, 'Unsupported'):
            spec_projection.check(self.html, data)

    def test_proposed_provenance_keeps_the_original_digest(self):
        manifest = spec_proposal.proposed_manifest(ROOT, self.html)
        entry = manifest['files'][0]
        self.assertEqual(manifest['schema'], 'codex-accounts/source-inputs/v2')
        self.assertEqual(entry['original_sha256'], spec_proposal.ORIGINAL_SHA256)
        self.assertEqual(entry['sha256'], spec_proposal.CANDIDATE_SHA256)
        self.assertEqual(hashlib.sha256(spec_index.SPEC.read_bytes()).hexdigest(),
                         spec_proposal.CANDIDATE_SHA256)
        self.assertEqual(manifest, json.loads((ROOT / 'docs/source-inputs.json').read_text()))
        self.assertEqual(spec_proposal.reconstruct_original(self.html),
                         spec_proposal.reconstruct_original(self.current))

    def test_unchanged_research_and_index_retain_original_digests(self):
        manifest = json.loads((ROOT / 'docs/source-inputs.json').read_text())
        expected = {
            'docs/codex-switchers-comparison.html': '3928997418de0c6871f7af4edbf8760ca1d7c049fd1f99850e5580c08aeebb57',
            'docs/maintainers/GITHUB_CONNECTOR_INDEX.md': '9cb896bd347db0c566e007fc846f4a2bb10f0880feb498371827e8a07ddc730c',
        }
        actual = {e['path']: e['sha256'] for e in manifest['files']}
        for path, digest in expected.items():
            self.assertEqual(actual[path], digest)
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), digest)

    def test_working_plan_projection_and_requirement_ownership(self):
        plan_projection.check(ROOT)
        model = json.loads(plan_projection.MODEL.search(
            (ROOT / 'docs/implementation-plan.html').read_text()).group(1))
        expected = {k: v['tests'] for k, v in spec_index.symbols(self.model).items()
                    if k.startswith(('F-', 'S-'))}
        actual = {r['id']: r['tests'] for r in model['requirements']}
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), len(model['requirements']))

    def test_plan_visible_drift_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'docs').mkdir()
            text = (ROOT / 'docs/implementation-plan.html').read_text()
            (root / 'docs/implementation-plan.html').write_text(
                text.replace('Delivery and evidence gates</h1>', 'Unauthorized</h1>', 1))
            with self.assertRaisesRegex(ValueError, 'mismatch'):
                plan_projection.check(root)

    def test_no_requirement_or_acceptance_is_removed(self):
        entries = spec_index.symbols(self.model)
        self.assertEqual(sum(k.startswith(('F-', 'S-')) for k in entries), 43)
        self.assertTrue(all(f'T-{i:02}' in entries for i in range(1, 35)))
        self.assertEqual(self.model['version'], '0.1.0')
        self.assertEqual(self.model['amendment_status'], 'proposed_for_owner_review')


if __name__ == '__main__':
    unittest.main()
