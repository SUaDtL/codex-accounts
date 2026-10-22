"""CA06B exact additive review mutations; no acquisition or product authority."""
import copy
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from tools import check_dependencies as checker

ROOT = Path(__file__).resolve().parents[2]
REVIEW = 'docs/ca-08-dependencies.json'


class Ca06Dependencies(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.review = json.loads((ROOT / REVIEW).read_text(encoding='utf8'))
        self.ca06 = json.loads((ROOT / 'docs/ca-06b-dependencies.json').read_text(encoding='utf8'))
        for relative in [*self.review['manifest_sha256'],
                         *self.review['predecessor_sha256'], REVIEW, 'Cargo.lock']:
            dest = self.root / relative
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, dest)

    def refuse(self, category):
        with self.assertRaisesRegex(ValueError, category):
            checker.check(self.root)

    def write_review(self, review):
        (self.root / REVIEW).write_text(json.dumps(review), encoding='utf8')

    def test_exact_421_packages_and_application_manifests_pass(self):
        self.assertEqual(checker.check(self.root)['reviewed_packages'], 421)
        self.assertEqual(len(self.review['manifest_sha256']), 10)

    def test_each_historical_review_cannot_be_laundered_with_a_new_claimed_hash(self):
        for relative in self.review['predecessor_sha256']:
            with self.subTest(relative=relative):
                path = self.root / relative
                original = path.read_bytes()
                path.write_bytes(original + b'\n')
                changed = copy.deepcopy(self.review)
                changed['predecessor_sha256'][relative] = hashlib.sha256(path.read_bytes()).hexdigest()
                self.write_review(changed)
                self.refuse('Prior review bytes')
                path.write_bytes(original)
        self.assertEqual(len(self.review['predecessor_sha256']), 10)

    def test_ca06_review_status_packages_features_and_manifest_claims_remain_immutable(self):
        for mutate in [
            lambda r: r.update(status='pending'),
            lambda r: r['packages'].pop(),
            lambda r: r['packages'].append(copy.deepcopy(r['packages'][0])),
            lambda r: r['feature_scope']['metadata_union_changes'][0]['after'].append('unreviewed'),
            lambda r: r['manifest_sha256'].update({'Cargo.toml': '0' * 64}),
            lambda r: r['predecessor_sha256'].pop('docs/ca-05f-dependencies.json'),
        ]:
            changed = copy.deepcopy(self.ca06)
            mutate(changed)
            (self.root / 'docs/ca-06b-dependencies.json').write_text(json.dumps(changed), encoding='utf8')
            self.refuse('Prior review bytes')

    def test_duplicate_json_keys_never_collapse(self):
        path = self.root / REVIEW
        text = path.read_text(encoding='utf8')
        path.write_text(text.replace('"packet": "CA-08",',
                        '"packet": "CA-08", "packet": "CA-08",'), encoding='utf8')
        self.refuse('Duplicate review key')

    def test_lock_checksum_source_version_and_duplicate_entries_refuse(self):
        path = self.root / 'Cargo.lock'
        original = path.read_text(encoding='utf8')
        first = self.ca06['packages'][0]
        for altered in [
            original.replace(first['checksum'], '0' * 64),
            original.replace(first['source'], 'git+https://synthetic.invalid/repo', 1),
            original.replace('name = "adler2"', 'name = "unreviewed"', 1),
            original.replace('name = "adler2"\nversion = "2.0.1"',
                             'name = "adler2"\nversion = "2.0.2"', 1),
            original + '\n[[package]]\nname = "adler2"\nversion = "2.0.1"\n'
            + f'source = "{first["source"]}"\nchecksum = "{first["checksum"]}"\n',
        ]:
            path.write_text(altered, encoding='utf8')
            self.refuse('Dependency graph changed')

    def test_even_local_only_lock_edits_need_exact_review(self):
        path = self.root / 'Cargo.lock'
        path.write_bytes(path.read_bytes().replace(b'codex-accounts-shell-preparation',
                                                 b'codex-accounts-other-shell'))
        self.refuse('Reviewed lock bytes')

    def test_manifest_bytes_and_complete_member_inventory_remain_pinned(self):
        path = self.root / 'crates/discovery/Cargo.toml'
        path.write_bytes(path.read_bytes() + b'\n# changed\n')
        self.refuse('manifest changed')
        shutil.copyfile(ROOT / 'crates/discovery/Cargo.toml', path)
        workspace = self.root / 'Cargo.toml'
        workspace.write_text(workspace.read_text().replace('"crates/discovery", ', ''), encoding='utf8')
        self.refuse('manifest review set')

    def test_shell_edges_cannot_be_unconditional_or_enable_extra_features(self):
        path = self.root / 'crates/shell-preparation/Cargo.toml'
        original = path.read_text()
        for altered in [
            original.replace("[target.'cfg(windows)'.dependencies]", '[dependencies]'),
            original.replace('default-features = false', 'default-features = true', 1),
            original.replace('["wry", "custom-protocol"]', '["wry", "custom-protocol", "devtools"]'),
            original.replace('=2.12.4', '=2.12.5'),
            original.replace("cfg(windows)", 'cfg(unix)'),
            original + '\n[dev-dependencies]\nserde = "=1.0.229"\n',
            original + '\n[features]\ndefault = ["tauri/devtools"]\n',
        ]:
            path.write_text(altered, encoding='utf8')
            self.refuse('Shell dependency boundary')

    def test_runtime_json_features_cannot_use_transitive_tauri_permission(self):
        path = self.root / 'crates/runtime/Cargo.toml'
        original = path.read_text()
        for feature in ['raw_value', 'unbounded_depth', 'preserve_order']:
            path.write_text(original.replace('serde_json = "=1.0.145"',
                'serde_json = { version = "=1.0.145", features = ["' + feature + '"] }'))
            self.refuse('Runtime dependency boundary')

    def test_safe_crates_cannot_add_target_or_build_edges(self):
        for name in ['core', 'platform']:
            path = self.root / f'crates/{name}/Cargo.toml'
            original = path.read_bytes()
            for section in ['build-dependencies', "target.'cfg(windows)'.dependencies"]:
                path.write_bytes(original + f'\n[{section}]\nserde = "=1.0.229"\n'.encode())
                self.refuse('Safe bootstrap crate')
            path.write_bytes(original)


if __name__ == '__main__':
    unittest.main()
