"""CA-05B boundary and source-review mutation tests, not protocol qualification."""
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from tools import check_dependencies, macos_preparation

ROOT = Path(__file__).resolve().parents[2]


class JsonRuntimeBoundaryTests(unittest.TestCase):
    def test_runtime_reuses_parser_without_io_or_qualification(self):
        sources = list((ROOT / 'crates/runtime/src').glob('*.rs'))
        for path in sources:
            source = path.read_text(encoding='utf-8')
            self.assertIn('#![forbid(unsafe_code)]', source)
            for forbidden in ('std::fs', 'std::net', 'std::process', 'unsafe {',
                              'env::var', 'println!', 'eprintln!', 'impl Effects'):
                self.assertNotIn(forbidden, source)
        wrapper = (ROOT / 'crates/runtime/src/json_object.rs').read_text()
        self.assertIn('validate_json_object(frame.as_bytes())', wrapper)
        self.assertIn('pub struct JsonObjectFrame(UntrustedFrame);', wrapper)
        self.assertIn('self.decoder.fail(FrameError::Poisoned)', wrapper)
        raw = (ROOT / 'crates/runtime/src/lib.rs').read_text()
        self.assertIn('UntrustedFrame(Zeroizing<Vec<u8>>)', raw)
        self.assertIn('partial: Zeroizing<Vec<u8>>', raw)
        self.assertIn('self.partial.zeroize()', raw)
        for path in ('build.rs', 'src/bin', 'src/main.rs'):
            self.assertFalse((ROOT / 'crates/runtime' / path).exists())

    def test_new_review_retains_all_predecessors_and_external_records(self):
        current = json.loads((ROOT / 'docs/ca-05b-dependencies.json').read_text())
        previous = json.loads((ROOT / 'docs/ca-04b-dependencies.json').read_text())
        self.assertEqual(current['packages'], [])
        self.assertEqual(set(current['predecessor_sha256']),
                         set(previous['predecessor_sha256']) | {'docs/ca-04b-dependencies.json'})
        self.assertEqual(check_dependencies.check()['reviewed_packages'], 58)

    def test_runtime_cannot_gain_unreviewed_dependency_even_with_updated_manifest_hash(self):
        review = json.loads((ROOT / 'docs/ca-05b-dependencies.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in [*review['manifest_sha256'], *review['predecessor_sha256'],
                         'docs/ca-05b-dependencies.json', 'Cargo.lock']:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / path, root / path)
            path = 'crates/runtime/Cargo.toml'
            target = root / path
            target.write_text(target.read_text().replace('[dependencies]',
                              '[dependencies]\nunreviewed = "=9.9.9"'), encoding='utf-8')
            review['manifest_sha256'][path] = hashlib.sha256(target.read_bytes()).hexdigest()
            (root / 'docs/ca-05b-dependencies.json').write_text(json.dumps(review))
            with self.assertRaisesRegex(ValueError, 'Runtime dependency boundary'):
                check_dependencies.check(root)

    def test_macos_join_preserves_review_and_predecessor_not_just_new_hashes(self):
        record = json.loads((ROOT / macos_preparation.PACKET).read_text())
        self.assertEqual(record['native_status'], 'not-run')
        self.assertIn('crates/vault-storage/src/stage_repair.rs', record['interfaces'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = [*macos_preparation.SOURCES, macos_preparation.PACKET,
                     'docs/macos/interface-baseline-ca04c.json',
                     'docs/macos/ca-04d-interface-review.md']
            for path in paths:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / path, root / path)
            macos_preparation.check(root)
            for path in paths[-2:]:
                original = (root / path).read_bytes()
                (root / path).write_bytes(original + b'changed')
                with self.assertRaisesRegex(macos_preparation.PacketError, 'provenance changed'):
                    macos_preparation.check(root)
                (root / path).write_bytes(original)


if __name__ == '__main__':
    unittest.main()
