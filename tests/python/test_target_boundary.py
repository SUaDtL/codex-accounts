"""CA-04C evidence cannot become Desktop authority or a zero-test success."""
from pathlib import Path
import re
import unittest
from tools import run_native_tests as native

ROOT = Path(__file__).resolve().parents[2]

class TargetEvidenceTests(unittest.TestCase):
    def output(self):
        rows = [f'test {native.TARGET_PREFIX}{name} ... ok' for name in native.TARGET_CASES]
        rows += [f'test result: ok. {len(native.TARGET_CASES)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s']
        return ('\n'.join(rows) + '\n').encode()

    def test_inventory_excludes_child_harness_and_matches_all_behavior_tests(self):
        source = (ROOT / 'crates/vault-storage/src/native/lifecycle/target_tests.rs').read_text(encoding='utf-8')
        names = re.findall(r'fn (ca04c_[a-z_]+)\(', source)
        self.assertEqual(set(names), set(native.TARGET_CASES))
        self.assertEqual(len(names), len(native.TARGET_CASES))
        self.assertNotIn('#[ignore', source)
        self.assertEqual(len(native.CASES), 11)
        self.assertEqual(len(native.TARGET_CASES), 12)

    def test_target_proof_rejects_missing_ignored_duplicate_and_legacy_rows(self):
        good = self.output()
        native.verify(good, 0, native.TARGET_CASES, prefix=native.TARGET_PREFIX)
        native.verify(good.replace(b'\n', b'\r\n'), 0, native.TARGET_CASES, prefix=native.TARGET_PREFIX)
        for bad in [good.split(b'\n',1)[1], good + good.splitlines()[0] + b'\n',
                    good.replace(b' ... ok', b' ... ignored', 1),
                    good.replace(b' ... ok', b' ... FAILED', 1),
                    good.replace(native.TARGET_PREFIX.encode(), native.PREFIX.encode()),
                    b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\n']:
            with self.assertRaises(ValueError):
                native.verify(bad, 0, native.TARGET_CASES, prefix=native.TARGET_PREFIX)

    def test_native_target_has_no_production_constructor_or_public_dispatch(self):
        source = (ROOT / 'crates/vault-storage/src/native/lifecycle/target.rs').read_text(encoding='utf-8')
        self.assertIn('#[cfg(test)]\n    pub(super) fn open', source)
        self.assertNotIn('pub fn ', source)
        self.assertNotIn('impl Effects for', source)
        self.assertNotIn('TRUNCATE_EXISTING', source)
        self.assertNotIn('std::fs::remove_file', source)
        self.assertIn('Zeroizing<Vec<u8>>', source)
        self.assertIn('FILE_SHARE_READ | FILE_SHARE_DELETE', source)
        self.assertIn('not provide an atomic namespace compare-and-swap', source)
        home = (ROOT / 'crates/vault-storage/src/native/lifecycle/home.rs').read_text(encoding='utf-8')
        self.assertIn('Self::acquire_with_leaf_share(path, FILE_SHARE_READ)', home)
        self.assertIn('#[cfg(test)]\n    pub(super) fn for_synthetic_target', home)

    def test_cleanup_authority_comes_from_all_authenticated_references(self):
        source = (ROOT / 'crates/vault-storage/src/coordinator.rs').read_text(encoding='utf-8')
        self.assertIn('j.staging_intent | j.restore_intent', source)
        self.assertIn('generation(&self.registry, j.original_target)', source)
        self.assertIn('allowed: &[CredentialSet]', source)
        self.assertIn('#![forbid(unsafe_code)]', source)

if __name__ == '__main__':
    unittest.main()
