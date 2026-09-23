"""CA-03B source boundaries and fail-closed acquisition regressions, not native proof."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest
from tools import check_dependencies

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / 'crates/vault-crypto'


class CryptoBoundary(unittest.TestCase):
    def test_library_only_and_native_unsafe_is_scoped(self):
        manifest = tomllib.loads((CRATE / 'Cargo.toml').read_text())
        self.assertEqual(manifest['lints']['rust']['unsafe_code'], 'deny')
        for path in ['build.rs', 'src/main.rs', 'src/bin', 'examples']:
            self.assertFalse((CRATE / path).exists())
        lib = (CRATE / 'src/lib.rs').read_text()
        self.assertIn('#[allow(unsafe_code)]\nmod dpapi;', lib)
        self.assertEqual(lib.count('allow(unsafe_code)'), 1)
        for path in (CRATE / 'src').glob('*.rs'):
            text = path.read_text()
            for prohibited in ['std::fs', 'std::net', 'std::process', 'println!', 'eprintln!',
                               'Serialize', 'Deserialize', 'pub unsafe', 'env::var']:
                self.assertNotIn(prohibited, text, (path.name, prohibited))
            if path.name not in {'dpapi.rs', 'lib.rs'}:
                self.assertIn('#![forbid(unsafe_code)]', text)
                self.assertNotIn('unsafe {', text)
        dpapi = (CRATE / 'src/dpapi.rs').read_text()
        self.assertNotIn('CRYPTPROTECT_LOCAL_MACHINE', dpapi)
        self.assertNotIn('CRYPTPROTECT_PROMPTSTRUCT', dpapi)
        self.assertIn('CRYPTPROTECT_UI_FORBIDDEN', dpapi)
        self.assertIn('LocalFree(', dpapi)
        self.assertIn('.zeroize()', dpapi)

    def test_no_public_rng_or_raw_root_key_injection(self):
        text = '\n'.join(p.read_text() for p in (CRATE / 'src').glob('*.rs'))
        for prohibited in ['pub trait Entropy', 'pub fn generate_using', 'pub fn seal_using',
                           'pub fn derive', 'pub secret:', 'pub id:', 'impl Clone for RootKey']:
            self.assertNotIn(prohibited, text)
        self.assertIn('getrandom::fill(output)', text)
        self.assertNotIn('getrandom_backend', text)
        self.assertIn('output.zeroize()', text)

    def test_existing_data_buffers_use_vetted_zeroization(self):
        resources = (ROOT / 'crates/vault/src/resources.rs').read_text()
        self.assertEqual(resources.count('bytes.zeroize()'), 2)
        self.assertNotIn('bytes.fill(0)', resources)
        identity = (ROOT / 'crates/vault/src/generations.rs').read_text()
        self.assertIn('impl Drop for Identity', identity)
        for field in ['issuer', 'subject', 'workspace']:
            self.assertIn(f'self.{field}.zeroize()', identity)
        parser = (ROOT / 'crates/vault/src/strict_json.rs').read_text()
        self.assertIn('impl Drop for DecodedKey', parser)
        self.assertIn('keys.insert(DecodedKey(key))', parser)

    def test_two_user_evidence_is_explicit_not_an_automatic_pass(self):
        source = (CRATE / 'tests/native_users.rs').read_text()
        self.assertEqual(source.count('#[ignore ='), 3)
        self.assertIn('create_new(true)', source)
        self.assertIn('Err(CryptoError::KeyProtectionUnavailable)', source)
        self.assertIn('CA03B-SYNTHETIC-FIXTURE', source)
        self.assertIn('/crates/vault-crypto/ca03b-synthetic.dpapi',
                      (ROOT / '.gitignore').read_text())
        self.assertFalse((CRATE / 'ca03b-synthetic.dpapi').exists())


class DependencyReviewTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        manifest = json.loads((ROOT / 'docs/ca-03b-dependencies.json').read_text())
        paths = list(manifest['manifest_sha256']) + ['Cargo.lock', 'docs/ca-03b-dependencies.json', 'docs/ca-02-dependencies.json']
        for relative in paths:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)
        self.review = manifest

    def fail_with(self, review):
        (self.root / 'docs/ca-03b-dependencies.json').write_text(json.dumps(review))
        with self.assertRaises(ValueError):
            check_dependencies.check(self.root)

    def test_current_exact_review_passes(self):
        self.assertEqual(check_dependencies.check(self.root)['reviewed_packages'], 58)

    def test_unreviewed_or_wrong_lock_refuses(self):
        review = copy.deepcopy(self.review)
        review['status'] = 'pending'
        self.fail_with(review)
        review = copy.deepcopy(self.review)
        review['lock_sha256'] = '0' * 64
        self.fail_with(review)

    def test_duplicate_or_missing_packages_refuse(self):
        review = copy.deepcopy(self.review)
        review['packages'].append(review['packages'][0])
        self.fail_with(review)
        review = copy.deepcopy(self.review)
        review['packages'].pop()
        self.fail_with(review)

    def test_altered_checksum_or_inherited_override_refuses(self):
        review = copy.deepcopy(self.review)
        review['packages'][0]['checksum'] = '0' * 64
        self.fail_with(review)
        review = copy.deepcopy(self.review)
        inherited = json.loads((self.root / 'docs/ca-02-dependencies.json').read_text())
        review['packages'].append(inherited['packages'][0])
        self.fail_with(review)

    def test_manifest_features_and_membership_are_reviewed(self):
        path = self.root / 'crates/vault-crypto/Cargo.toml'
        path.write_text(path.read_text() + '\n# changed review input\n')
        with self.assertRaisesRegex(ValueError, 'manifest changed'):
            check_dependencies.check(self.root)
        review = copy.deepcopy(self.review)
        review['manifest_sha256'].pop('crates/vault-crypto/Cargo.toml')
        self.fail_with(review)


if __name__ == '__main__':
    unittest.main()
