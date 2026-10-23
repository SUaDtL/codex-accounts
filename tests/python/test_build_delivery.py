"""Build identity and retention behavior, including failed retries of a good run."""
import json
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from retain_latest_build import PREFIX, deletion_ids
from package_windows import package, sha, sbom


def fixture(identifier, number, attempt=1, conclusion="success", status="completed"):
    artifact = {"id": identifier, "name": f"{PREFIX}{number}-{attempt}", "expired": False,
                "workflow_run": {"id": number}}
    run = {"workflow_id": 7, "run_number": number, "run_attempt": attempt,
           "status": status, "conclusion": conclusion, "owner_build": True}
    return artifact, ((number, attempt), run)


class DeliveryTests(unittest.TestCase):
    def select(self, *cases):
        artifacts, runs = zip(*cases)
        return deletion_ids(list(artifacts), dict(runs), 7)

    def test_latest_success_survives_failed_new_run_and_retry(self):
        self.assertEqual(self.select(fixture(1, 1), fixture(2, 2), fixture(3, 3, conclusion="failure")), [1, 3])
        self.assertEqual(self.select(fixture(1, 1), fixture(2, 1, 2, "failure")), [2])

    def test_running_unknown_and_other_workflow_artifacts_are_untouched(self):
        artifact, (key, run) = fixture(3, 3)
        run["workflow_id"] = 99
        self.assertEqual(self.select(fixture(1, 1), fixture(2, 2, status="in_progress"), (artifact, (key, run))), [])
        artifact["name"] = "unrelated-report"
        self.assertEqual(deletion_ids([artifact], {key: run}, 7), [])
        self.assertEqual(deletion_ids([fixture(1, 1)[0]], {}, 7), [])

    def test_late_completion_does_not_replace_newer_success(self):
        self.assertEqual(self.select(fixture(8, 2), fixture(9, 1)), [9])
        self.assertEqual(self.select(fixture(1, 1, conclusion="failure")), [1])

    def test_fork_success_is_removed_without_replacing_owner_build(self):
        artifact, (key, run) = fixture(2, 2)
        run["owner_build"] = False
        self.assertEqual(self.select(fixture(1, 1), (artifact, (key, run))), [2])
        self.assertEqual(self.select((artifact, (key, run))), [2])

    def test_expired_or_mismatched_artifact_is_not_replacement(self):
        artifact, entry = fixture(2, 2)
        artifact["expired"] = True
        self.assertEqual(self.select(fixture(1, 1), (artifact, entry)), [2])
        artifact["name"] = PREFIX + "9-9"
        self.assertEqual(self.select(fixture(1, 1), (artifact, entry)), [])

    def test_package_inventory_checksums_and_stale_output_refusal(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            executable = root / "fixture.exe"
            executable.write_bytes(b"MZ synthetic package fixture, not executable")
            output = root / "package"
            package(executable, output)
            metadata = json.loads((output / "BUILD.json").read_text())
            self.assertEqual(metadata["version"], tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"])
            self.assertEqual(metadata["publication"], "ci-candidate")
            self.assertEqual(metadata["payload_sha256"], sha(executable))
            for line in (output / "SHA256SUMS").read_text().splitlines():
                digest, name = line.split("  ")
                self.assertEqual(digest, sha(output / name))
            inventory = json.loads((output / "SBOM.cdx.json").read_text())
            self.assertEqual(inventory, sbom(ROOT, metadata["version"]))
            self.assertGreater(len(inventory["components"]), 0)
            with self.assertRaises(FileExistsError):
                package(executable, output)


if __name__ == "__main__":
    unittest.main()
