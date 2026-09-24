"""Mutation regressions for the exact workflow and dependency-acquisition contract."""
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from ci_contract import check, check_contract

WORKFLOW = ROOT / ".github/workflows/ci.yml"


class DependencyAcquisitionContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def reject(self, old: str, new: str) -> None:
        changed = self.text.replace(old, new)
        self.assertNotEqual(changed, self.text, "Mutation must actually change the workflow")
        with self.assertRaises(ValueError):
            check_contract(changed)

    def test_current_workflow_keeps_review_before_acquisition(self) -> None:
        check(ROOT)

    def test_combined_native_commands_are_rejected(self) -> None:
        self.reject("run: python tools/check_dependencies.py", "run: |\n          python tools/check_dependencies.py\n          cargo fetch --locked")

    def test_acquisition_cannot_ignore_review_failure(self) -> None:
        self.reject("id: dependencies\n", "id: dependencies\n        if: ${{ always() }}\n")

    def test_independent_checks_require_review_success(self) -> None:
        self.reject(" && steps.dependency_review.outcome == 'success'", "")

    def test_failures_cannot_be_marked_nonblocking(self) -> None:
        self.reject("id: dependency_review\n", "id: dependency_review\n        continue-on-error: true\n")

    def test_source_tests_do_not_disappear_after_integrity_failure(self) -> None:
        self.reject("${{ !cancelled() && steps.checkout.outcome == 'success' }}", "${{ success() }}")

    def test_missing_documentation_release_or_host_check_refuses(self) -> None:
        for identifier in ["doctests", "release_build", "host"]:
            with self.subTest(identifier=identifier):
                self.reject(f"id: {identifier}\n", f"id: disabled_{identifier}\n")

    def test_cargo_must_continue_after_a_test_executable_failure(self) -> None:
        self.reject(" --no-fail-fast", "")

    def test_offline_locked_and_warning_gates_remain(self) -> None:
        for flag in [" --locked", " --offline", " -- -D warnings"]:
            with self.subTest(flag=flag):
                self.reject(flag, "")

    def test_aggregate_gate_cannot_skip_failed_jobs(self) -> None:
        self.reject("    if: ${{ always() }}\n", "    if: ${{ success() }}\n")
        self.reject("needs: [source-and-python, rust-models]", "needs: [source-and-python]")

    def test_outcomes_cannot_be_replaced_by_literal_success(self) -> None:
        self.reject("${{ toJSON(needs) }}", "success")
        self.reject("${{ toJSON(steps) }}", "success")

    def test_event_filters_permissions_and_checkout_are_reviewed(self) -> None:
        for old, new in [
            ("branches: [main]", "branches: ['**']"),
            ("  pull_request:\n", "  pull_request:\n    paths: ['README.md']\n"),
            ("contents: read", "contents: write"),
            ("persist-credentials: false", "persist-credentials: true"),
            ("@11bd71901bbe5b1630ceea73d27597364c9af683", "@main"),
        ]:
            with self.subTest(old=old):
                self.reject(old, new)

    def test_matrix_cannot_silently_lose_windows_or_fail_fast(self) -> None:
        self.reject("ubuntu-24.04, windows-latest, macos-latest", "ubuntu-24.04, macos-latest")
        self.reject("fail-fast: false", "fail-fast: true")

    def test_persistent_temporary_workflow_and_toolchain_drift_refuse(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            workflows = root / ".github/workflows"
            workflows.mkdir(parents=True)
            (workflows / "ci.yml").write_text(self.text, encoding="utf-8")
            pin = (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
            (root / "rust-toolchain.toml").write_text(pin, encoding="utf-8")
            check(root)
            (workflows / "temporary.yml").write_text("name: not validation\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                check(root)
            (workflows / "temporary.yml").unlink()
            (root / "rust-toolchain.toml").write_text(pin.replace("1.90.0", "stable"), encoding="utf-8")
            with self.assertRaises(ValueError):
                check(root)


if __name__ == "__main__":
    unittest.main()
