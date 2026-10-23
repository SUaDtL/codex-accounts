import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "ci_prohibitions", ROOT / "tools" / "ci_prohibitions.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ProductProhibitions(unittest.TestCase):
    def test_repository_respects_product_boundaries(self):
        MODULE.check(ROOT)

    def test_force_termination_is_rejected_in_application_code(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            crate = root / "crates" / "sample" / "src" / "native" / "lifecycle"
            crate.mkdir(parents=True)
            (crate / "process.rs").write_text("fn x() { TerminateProcess(); }", encoding="utf-8")
            manifest = root / "crates" / "sample" / "Cargo.toml"
            manifest.write_text(
                '[package]\nname = "sample"\nversion = "0.1.0"\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "force termination"):
                MODULE.check(root)


if __name__ == "__main__":
    unittest.main()
