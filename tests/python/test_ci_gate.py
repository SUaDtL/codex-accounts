"""Executable truth-table and hostile-input checks for CI evidence assessment."""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from ci_gate import MAX_INPUT_BYTES, REQUIRED, assess, host_matches


def results(scope: str) -> dict:
    field = "result" if scope == "jobs" else "outcome"
    return {name: {field: "success", "outputs": {}} for name in REQUIRED[scope]}


class CiGateTests(unittest.TestCase):
    def test_exact_success_of_every_required_check_passes(self) -> None:
        for scope in REQUIRED:
            with self.subTest(scope=scope):
                self.assertTrue(assess(scope, json.dumps(results(scope))))

    def test_every_failure_skip_cancellation_and_missing_result_blocks(self) -> None:
        for scope, names in REQUIRED.items():
            field = "result" if scope == "jobs" else "outcome"
            for name in names:
                for value in ["failure", "skipped", "cancelled", "neutral", "", None, True, 0, "Success"]:
                    with self.subTest(scope=scope, name=name, result=value):
                        sample = results(scope)
                        sample[name][field] = value
                        self.assertFalse(assess(scope, json.dumps(sample)))
                sample = results(scope)
                del sample[name]
                self.assertFalse(assess(scope, json.dumps(sample)))

    def test_success_conclusion_cannot_hide_failed_outcome(self) -> None:
        sample = results("rust")
        sample["tests"] = {"conclusion": "success", "outcome": "failure"}
        self.assertFalse(assess("rust", json.dumps(sample)))

    def test_additional_success_cannot_replace_a_required_job(self) -> None:
        sample = results("jobs")
        del sample["rust-models"]
        sample["unrelated"] = {"result": "success"}
        self.assertFalse(assess("jobs", json.dumps(sample)))

    def test_malformed_oversized_duplicate_and_deep_input_refuse(self) -> None:
        for sample in ["", "null", "true", "[]", "{", "[" * 2000 + "]" * 2000, " " * (MAX_INPUT_BYTES + 1)]:
            self.assertFalse(assess("jobs", sample))
        sample = json.dumps(results("jobs"))
        self.assertFalse(assess("jobs", sample[:-1] + ',"rust-models":{"result":"success"}}'))
        self.assertFalse(assess("jobs", sample.replace('"result": "success"', '"result":"failure","result":"success"')))
        self.assertFalse(assess("unexpected", sample))

    def test_wrong_object_shapes_refuse(self) -> None:
        for value in [[], "success", None, True]:
            sample = results("jobs")
            sample["rust-models"] = value
            self.assertFalse(assess("jobs", json.dumps(sample)))

    def test_host_coverage_is_explicit_and_windows_arm_does_not_pass(self) -> None:
        self.assertTrue(host_matches("windows-latest", "Windows", "AMD64"))
        self.assertTrue(host_matches("ubuntu-24.04", "Linux", "x86_64"))
        self.assertTrue(host_matches("macos-latest", "Darwin", "arm64"))
        self.assertFalse(host_matches("windows-latest", "Windows", "ARM64"))
        self.assertFalse(host_matches("windows-latest", "Linux", "x86_64"))
        self.assertFalse(host_matches("unknown", "Windows", "AMD64"))

    def test_executable_exit_status_and_input_redaction(self) -> None:
        canary = "SYNTHETIC_CI_INPUT_NOT_A_REAL_SECRET"
        for payload, status in [(json.dumps(results("jobs")), 0), (canary, 1), ("", 1)]:
            env = dict(os.environ, CI_RESULTS=payload)
            result = subprocess.run([sys.executable, str(ROOT / "tools/ci_gate.py"), "jobs"],
                                    capture_output=True, text=True, env=env, timeout=10, check=False)
            self.assertEqual(result.returncode, status)
            self.assertNotIn(canary, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
