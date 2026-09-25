from __future__ import annotations

import ast
import json
from pathlib import Path
import unittest

from tools import spec_index

ROOT = Path(__file__).resolve().parents[2]


class SpecificationTests(unittest.TestCase):
    def test_preserved_sources_and_topology(self):
        result = spec_index.check()
        self.assertEqual(result["requirements"], 43)
        self.assertEqual(result["acceptance_scenarios"], 34)
        self.assertEqual(result["qualified_records"], 0)

    def test_can_retrieve_requirement_without_full_spec(self):
        data, _ = spec_index.model()
        item = spec_index.symbols(data)["F-020"]
        self.assertEqual(item["title"], "Preserve before replace")
        self.assertNotIn("SPEC-SOURCES", json.dumps(item))

    def test_all_acceptance_ids_are_present(self):
        data, _ = spec_index.model()
        table = spec_index.symbols(data)
        self.assertTrue(all(f"T-{i:02d}" in table for i in range(1, 35)))

    def test_safe_crates_keep_exact_dependency_and_native_boundaries(self):
        import tomllib
        for name in ["core", "platform", "runtime"]:
            path = ROOT / "crates" / name / "Cargo.toml"
            parsed = tomllib.loads(path.read_text())
            if name == "runtime":
                self.assertEqual(parsed["dependencies"], {
                    "codex-accounts-vault": {"path": "../vault", "version": "=0.1.0"},
                    "zeroize": {"version": "=1.8.2", "default-features": False, "features": ["alloc"]},
                })
            else:
                self.assertNotIn("dependencies", parsed)
            self.assertIs(parsed["package"]["publish"]["workspace"], True)
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
        self.assertFalse(workspace["workspace"]["package"]["publish"])
        self.assertEqual(workspace["workspace"]["lints"]["rust"]["unsafe_code"], "forbid")

        discovery = tomllib.loads((ROOT / "crates/discovery/Cargo.toml").read_text())
        self.assertEqual(discovery["lints"]["rust"]["unsafe_code"], "deny")
        self.assertEqual(discovery["dependencies"]["serde_json"], "=1.0.145")
        self.assertEqual(discovery["dependencies"]["toml"]["version"], "=0.8.23")
        self.assertIs(discovery["package"]["publish"]["workspace"], True)

    def test_inventory_has_no_subprocess_network_or_write_imports(self):
        tree = ast.parse((ROOT / "tools/q0_inventory.py").read_text())
        banned = {"subprocess", "socket", "requests", "urllib", "http", "shutil"}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                self.assertTrue(all(alias.name.split(".")[0] not in banned for alias in node.names))
            elif isinstance(node, ast.ImportFrom):
                self.assertNotIn((node.module or "").split(".")[0], banned)
        # This is a source lint, not a substitute for native actor-attributed tracing.

    def test_no_packaged_live_credential_filenames(self):
        prohibited = {"auth.json", "cap_sid", ".env"}
        for path in ROOT.rglob("*"):
            if any(part in {".git", "target", "__pycache__"} for part in path.parts):
                continue
            self.assertNotIn(path.name, prohibited)


if __name__ == "__main__":
    unittest.main()
