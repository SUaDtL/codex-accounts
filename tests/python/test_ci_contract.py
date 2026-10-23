"""Mutation regressions for the exact workflow and dependency-acquisition contract."""
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from ci_contract import check, check_contract

WORKFLOW = ROOT / ".github/workflows/ci.yml"
RELEASE = ROOT / ".github/workflows/release.yml"


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

    def test_real_rust_proof_fixture_requires_reviewed_pinned_toolchain(self) -> None:
        self.assertIn('id: proof_toolchain\n', self.text)
        self.reject('id: proof_toolchain\n', 'id: omitted_proof_toolchain\n')
        self.reject('run: rustup toolchain install 1.90.0 --profile minimal\n',
                    'run: rustup toolchain install stable --profile minimal\n')
        self.reject(" && steps.proof_toolchain.outcome == 'success'", '')

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
        for identifier in ["doctests", "release_build", "package_scan", "native_tests", "isolated", "source_evidence", "source_upload", "host"]:
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
        self.reject("needs: [source-and-python, rust-models, isolated-build, security, desktop-ui]", "needs: [source-and-python]")

    def test_outcomes_cannot_be_replaced_by_literal_success(self) -> None:
        self.reject("${{ toJSON(needs) }}", "success")
        self.reject("${{ toJSON(steps) }}", "success")

    def test_event_filters_permissions_and_checkout_are_reviewed(self) -> None:
        for old, new in [
            ("branches: [main]", "branches: ['**']"),
            ("  pull_request:\n", "  pull_request:\n    paths: ['README.md']\n"),
            ("contents: read", "contents: write"),
            ("persist-credentials: false", "persist-credentials: true"),
            ("@3d3c42e5aac5ba805825da76410c181273ba90b1", "@main"),
        ]:
            with self.subTest(old=old):
                self.reject(old, new)

    def test_matrix_cannot_silently_lose_windows_or_fail_fast(self) -> None:
        self.reject("ubuntu-24.04, windows-latest, macos-latest", "ubuntu-24.04, macos-latest")
        self.reject("fail-fast: false", "fail-fast: true")

    def test_desktop_ui_timeout_covers_a_cold_browser_install(self) -> None:
        self.reject("  desktop-ui:\n    name: Desktop UI behavior\n    runs-on: ubuntu-24.04\n    timeout-minutes: 15\n",
                    "  desktop-ui:\n    name: Desktop UI behavior\n    runs-on: ubuntu-24.04\n    timeout-minutes: 10\n")

    def test_native_evidence_cannot_be_omitted_or_changed_to_literal_success(self) -> None:
        self.reject("run: python tools/check_windows_process.py", "run: echo success")
        self.reject(" && steps.supervisor_tests.outcome == 'success'", "")
        self.reject("run: python tools/run_native_tests.py", "run: echo success")
        self.reject(" && matrix.os == 'windows-latest'", " && matrix.os == 'ubuntu-24.04'")
        self.reject("          CI_MATRIX_OS: ${{ matrix.os }}", "          CI_MATRIX_OS: ubuntu-24.04")

    def test_isolated_check_cannot_fallback_to_connected_host(self) -> None:
        self.reject("run: python3 tools/ci_isolated.py", "run: cargo test --offline")
        self.reject("rust-models, isolated-build, security", "rust-models, security")

    def test_artifact_is_exact_source_only_and_pinned(self) -> None:
        for old, new in [("/ca-source-evidence/", "/"), ("retention-days: 7", "retention-days: 90"),
                         ("include-hidden-files: false", "include-hidden-files: true"),
                         ("if-no-files-found: error", "if-no-files-found: ignore"),
                         ("@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a", "@main")]:
            self.reject(old, new)

    def test_unreviewed_extra_steps_refuse(self) -> None:
        self.reject("    steps:\n", "    steps:\n      - run: echo bypass\n")

    def test_persistent_temporary_workflow_and_toolchain_drift_refuse(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            workflows = root / ".github/workflows"
            workflows.mkdir(parents=True)
            (workflows / "ci.yml").write_text(self.text, encoding="utf-8")
            for name in ("artifact-retention.yml", "release.yml", "security-scheduled.yml"):
                (workflows / name).write_text((WORKFLOW.parent / name).read_text(encoding="utf-8"), encoding="utf-8")
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

    def test_release_workflow_is_manual_exact_main_and_least_privilege(self) -> None:
        release = RELEASE.read_text(encoding="utf-8")
        self.assertIn("workflow_dispatch:\n", release)
        self.assertNotIn("push:\n", release)
        for old, new in [
            ("contents: write", "contents: read"),
            ("id-token: write", "id-token: none"),
            ("ref: ${{ github.sha }}", "ref: main"),
            ("persist-credentials: false", "persist-credentials: true"),
            ("python3 tools/publish_release.py", "echo published"),
        ]:
            with self.subTest(old=old):
                changed = release.replace(old, new)
                self.assertNotEqual(changed, release)
                with tempfile.TemporaryDirectory() as folder:
                    root = Path(folder)
                    workflows = root / ".github/workflows"
                    workflows.mkdir(parents=True)
                    for name in ("artifact-retention.yml", "ci.yml", "release.yml", "security-scheduled.yml"):
                        source = (WORKFLOW.parent / name).read_text(encoding="utf-8")
                        (workflows / name).write_text(changed if name == "release.yml" else source, encoding="utf-8")
                    (root / "rust-toolchain.toml").write_text((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"), encoding="utf-8")
                    with self.assertRaises(ValueError):
                        check(root)


if __name__ == "__main__":
    unittest.main()
