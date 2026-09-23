"""Static CA-03A boundary checks; not runtime/native qualification."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class VaultBoundary(unittest.TestCase):
    def test_library_only_and_pinned_existing_serialization_dependencies(self):
        root = ROOT / 'crates/vault'
        manifest = tomllib.loads((root / 'Cargo.toml').read_text())
        self.assertEqual(manifest['dependencies'],
                         {'serde': '=1.0.229', 'serde_json': '=1.0.145'})
        self.assertTrue(manifest['lints']['workspace'])
        self.assertFalse((root / 'build.rs').exists())
        self.assertFalse((root / 'src/main.rs').exists())
        self.assertFalse((root / 'src/bin').exists())
        self.assertIn('#![forbid(unsafe_code)]', (root / 'src/lib.rs').read_text())

    def test_no_io_or_unsafe_dispatch_in_the_generic_slice(self):
        sources = list((ROOT / 'crates/vault/src').glob('*.rs'))
        self.assertTrue(sources)
        for path in sources:
            source = path.read_text()
            for forbidden in ['std::fs', 'std::net', 'std::process', 'unsafe {',
                              'extern "C"', 'include_bytes!', 'env::var']:
                self.assertNotIn(forbidden, source, (path.name, forbidden))


if __name__ == '__main__':
    unittest.main()
