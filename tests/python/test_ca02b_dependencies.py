"""Current dependency review plus immutable CA02B history."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from tools import check_dependencies as checker

ROOT = Path(__file__).resolve().parents[2]
REVIEW = 'docs/ca-08-dependencies.json'


class Ca02bDependencies(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.review = json.loads((ROOT / REVIEW).read_text())
        for relative in [*self.review['manifest_sha256'],
                         *self.review['predecessor_sha256'], REVIEW, 'Cargo.lock']:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)

    def test_current_ten_new_421_total_and_ten_predecessors_pass(self):
        self.assertEqual(checker.check(self.root)['reviewed_packages'], 421)
        self.assertEqual(len(self.review['packages']), 10)
        self.assertEqual(self.review['external_counts']['retained'], 411)
        self.assertEqual(len(self.review['predecessor_sha256']), 10)

    def test_license_amendment_does_not_allow_other_manifest_changes(self):
        path = self.root / 'Cargo.toml'
        original = path.read_bytes()
        for altered in [original.replace(b'AGPL-3.0-only', b'MIT'),
                        original.replace(b'license = "AGPL-3.0-only"\n', b''),
                        original.replace(b'edition = "2021"', b'edition = "2024"')]:
            path.write_bytes(altered)
            with self.assertRaisesRegex(ValueError, 'Reviewed dependency manifest changed'):
                checker.check(self.root)

    def test_either_missing_sdk_flag_extra_flag_and_sdk_version_refuse(self):
        path = self.root / 'crates/discovery/Cargo.toml'
        original = path.read_text()
        for altered in [
            original.replace('"Win32_Security_Cryptography_Catalog", ', ''),
            original.replace('"Win32_Security_Cryptography_Sip", ', ''),
            original.replace('"Win32_Security_Cryptography_Sip",',
                             '"Win32_Security_Cryptography_Sip", "Win32_Networking",'),
            original.replace('=0.61.2', '=0.61.3'),
            original.replace('cfg(windows)', 'cfg(unix)'),
            original.replace('windows-sys = { version',
                             'windows-sys = { default-features = false, version'),
        ]:
            self.assertNotEqual(altered, original)
            path.write_text(altered, encoding='utf8')
            with self.assertRaisesRegex(ValueError, 'Discovery SDK dependency boundary'):
                checker.check(self.root)

    def test_current_contributions_retention_and_feature_claims_cannot_change(self):
        for mutate in [
            lambda r: r.update(status='pending'),
            lambda r: r['packages'].append(copy.deepcopy(r['packages'][0])),
            lambda r: r['packages'].pop(),
            lambda r: r['feature_scope'].update(feature='unreviewed'),
            lambda r: r['manifest_sha256'].update({'Cargo.toml': '0' * 64}),
            lambda r: r['predecessor_sha256'].pop('docs/ca-06b-dependencies.json'),
        ]:
            changed = copy.deepcopy(self.review)
            mutate(changed)
            (self.root / REVIEW).write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError, 'Current review bytes'):
                checker.check(self.root)

    def test_ca06b_is_immutable_history_not_a_second_retained_contribution(self):
        path = self.root / 'docs/ca-06b-dependencies.json'
        historical = json.loads(path.read_text())
        self.assertEqual(len(historical['packages']), 349)
        self.assertEqual(len(historical['retained_package_identity']), 58)
        self.assertEqual(checker.check(self.root)['reviewed_packages'], 421)
        historical['packages'] += historical['retained_package_identity']
        path.write_text(json.dumps(historical))
        with self.assertRaisesRegex(ValueError, 'Prior review bytes'):
            checker.check(self.root)


if __name__ == '__main__':
    unittest.main()
