"""Named real Rust bootstrap; an outer pass does not qualify Desktop."""
from __future__ import annotations

import io
import json
from pathlib import Path
import unittest

from tools import windows_test_proof as proof

ROOT = Path(__file__).resolve().parents[2]


class WindowsCompatibilityExecution(unittest.TestCase):
    def __str__(self) -> str:
        return self.id()

    def test_profile_and_legacy_preservation(self) -> None:
        for profile in ("debug", "release"):
            for case in proof.PROFILE_LEGACY_CASES:
                with self.subTest(profile=profile, case=case):
                    record = proof.run_case(profile, case)
                    self.assertEqual(record.get("result"), "passed")
                    self.assertEqual(record["inner_case"], case)
                    self.assertEqual(record["profile"], profile)
                    self.assertEqual(record["named_passes"], 1)
                    self.assertEqual(record["toolchain"], "1.90.0")
                    self.assertEqual(record["desktop_qualification"], "not_established")
                    self.assertEqual(record["evidence_kind"], "native_case")
                    print(json.dumps(record, sort_keys=True), flush=True)

    def test_location_lock_collision(self) -> None:
        for profile in ("debug", "release"):
            for case in proof.LOCATION_CASES:
                with self.subTest(profile=profile, case=case):
                    record = proof.run_case(profile, case)
                    self.assertEqual(record.get("result"), "passed")
                    self.assertEqual(record["inner_case"], case)
                    self.assertEqual(record["profile"], profile)
                    self.assertEqual(record["named_passes"], 1)
                    self.assertEqual(record["toolchain"], "1.90.0")
                    self.assertEqual(record["desktop_qualification"], "not_established")
                    self.assertEqual(record["evidence_kind"], "native_case")
                    print(json.dumps(record, sort_keys=True), flush=True)

    def test_slot_decision_table(self) -> None:
        for profile in ("debug", "release"):
            for case in proof.SLOT_CASES:
                with self.subTest(profile=profile, case=case):
                    record = proof.run_case(profile, case)
                    self.assertEqual(record.get("result"), "passed")
                    self.assertEqual(record["inner_case"], case)
                    self.assertEqual(record["profile"], profile)
                    self.assertEqual(record["named_passes"], 1)
                    self.assertEqual(record["toolchain"], "1.90.0")
                    self.assertEqual(record["desktop_qualification"], "not_established")
                    self.assertEqual(record["evidence_kind"], "pure_slot_policy")
                    print(json.dumps(record, sort_keys=True), flush=True)

    def test_fixture_parent(self) -> None:
        for profile in ("debug", "release"):
            for case in (proof.FIXTURE_CASE, proof.FIXTURE_REFUSAL_CASE):
                with self.subTest(profile=profile, case=case):
                    record = proof.run_case(profile, case)
                    self.assertEqual(record.get("result"), "passed")
                    self.assertEqual(record["inner_case"], case)
                    self.assertEqual(record["profile"], profile)
                    self.assertEqual(record["named_passes"], 1)
                    self.assertEqual(record["toolchain"], "1.90.0")
                    self.assertEqual(record["desktop_qualification"], "not_established")
                    self.assertEqual(record["evidence_kind"], "native_case")
                    print(json.dumps(record, sort_keys=True), flush=True)

    def test_bridge_contract(self) -> None:
        for profile in ("debug", "release"):
            with self.subTest(profile=profile):
                record = proof.run_case(profile)
                self.assertEqual(record.get("result"), "passed")
                self.assertEqual(record["inner_case"], proof.BOOTSTRAP_CASE)
                self.assertEqual(record["profile"], profile)
                self.assertEqual(record["named_passes"], 1)
                self.assertEqual(record["toolchain"], "1.90.0")
                self.assertEqual(record["desktop_qualification"], "not_established")
                self.assertEqual(record["evidence_kind"], "portable_bootstrap")
                print(json.dumps(record, sort_keys=True), flush=True)
        # The outer -k selector must not suppress the nested adverse controls.
        suite = unittest.TestLoader().discover(
            str(ROOT / "tests" / "python"), pattern="test_windows_proof_runner.py")
        result = unittest.TextTestRunner(stream=io.StringIO(), verbosity=2).run(suite)
        self.assertEqual(result.testsRun, 27)
        self.assertEqual(result.skipped, [])
        self.assertTrue(result.wasSuccessful(), "bridge adverse controls must pass")


if __name__ == "__main__":
    unittest.main()
