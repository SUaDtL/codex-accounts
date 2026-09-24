"""Regression checks for the repository's dependency acquisition boundary.

This inspects the deliberately fixed step layout, not arbitrary YAML semantics.
A workflow restructuring must update this contract through review.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
CONDITION = ("${{ !cancelled() && steps.toolchain.outcome == 'success' && "
             "steps.dependency_review.outcome == 'success' && "
             "steps.dependencies.outcome == 'success' }}")


def check_contract(text: str) -> None:
    if text.count("\n  rust-models:\n") != 1:
        raise ValueError("Expected the reviewed Rust job")
    job = text.split("\n  rust-models:\n", 1)[1]
    steps = re.split(r"(?m)^      - ", job)[1:]

    def find_step(field: str, value: str) -> str:
        pattern = rf"(?m)^(?:        )?{field}: {re.escape(value)}$"
        found = [step for step in steps if re.search(pattern, step)]
        if len(found) != 1:
            raise ValueError("Expected exactly one reviewed step")
        return found[0]

    review = find_step("id", "dependency_review")
    fetch = find_step("id", "dependencies")
    for step, command in [(review, "python tools/check_dependencies.py"),
                          (fetch, "cargo fetch --locked")]:
        if re.findall(r"(?m)^        run: (.+)$", step) != [command]:
            raise ValueError("Review and acquisition must be separate single commands")
        if re.search(r"(?m)^        (if|continue-on-error|shell):", step):
            raise ValueError("Acquisition must keep the default success/exit-code gate")
    if steps.index(review) >= steps.index(fetch):
        raise ValueError("Review must precede acquisition")
    for name in ["Test models without network", "Lint models without network"]:
        step = find_step("name", name)
        if re.findall(r"(?m)^        if: (.+)$", step) != [CONDITION]:
            raise ValueError("Independent checks must require reviewed acquisition")
    if re.search(r"(?m)^\s*continue-on-error:", job):
        raise ValueError("All checks remain blocking")


class DependencyAcquisitionContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def test_current_workflow_keeps_review_before_acquisition(self) -> None:
        check_contract(self.text)

    def test_combined_native_commands_are_rejected(self) -> None:
        changed = self.text.replace(
            "run: python tools/check_dependencies.py",
            "run: |\n          python tools/check_dependencies.py\n          cargo fetch --locked")
        with self.assertRaises(ValueError):
            check_contract(changed)

    def test_acquisition_cannot_ignore_review_failure(self) -> None:
        changed = self.text.replace("id: dependencies\n", "id: dependencies\n        if: ${{ always() }}\n")
        with self.assertRaises(ValueError):
            check_contract(changed)

    def test_independent_checks_require_review_success(self) -> None:
        changed = self.text.replace(" && steps.dependency_review.outcome == 'success'", "")
        with self.assertRaises(ValueError):
            check_contract(changed)

    def test_failures_cannot_be_marked_nonblocking(self) -> None:
        changed = self.text.replace("id: dependency_review\n", "id: dependency_review\n        continue-on-error: true\n")
        with self.assertRaises(ValueError):
            check_contract(changed)


if __name__ == "__main__":
    unittest.main()
