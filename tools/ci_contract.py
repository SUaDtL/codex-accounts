"""Check this repository's reviewed, fixed workflow layout, not arbitrary YAML.

GitHub validates YAML syntax. These dependency-free contract checks catch accidental
loss of mandatory commands, evidence gates and event boundaries. They do not make
an untrusted PR or its modified checker trustworthy; owner review/protection is required.
"""
from __future__ import annotations

from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CHECKOUT = "3d3c42e5aac5ba805825da76410c181273ba90b1"
UPLOAD = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
SOURCE_IF = "${{ !cancelled() && steps.checkout.outcome == 'success' }}"
RUST_IF = ("${{ !cancelled() && steps.toolchain.outcome == 'success' && "
           "steps.dependency_review.outcome == 'success' && "
           "steps.dependencies.outcome == 'success' }}")
ASSESS_IF = "${{ always() && steps.checkout.outcome == 'success' }}"
HEADER = """name: Repository checks

on:
  push:
    branches: [main]
  pull_request:
  merge_group:
    types: [checks_requested]
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: checks-${{ github.workflow }}-${{ github.event_name }}-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}"""

RELEASE_WORKFLOW = """name: Publish versioned release

on:
  workflow_dispatch:
    inputs:
      version:
        description: Exact committed version, for example 1.0.0-rc.1
        required: true
        type: string

permissions:
  contents: read

concurrency:
  group: release-${{ inputs.version }}
  cancel-in-progress: false

jobs:
  publish:
    name: Publish verified Windows release
    if: ${{ github.repository == 'SUaDtL/codex-accounts' && github.ref == 'refs/heads/main' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    permissions:
      actions: read
      contents: write
      id-token: write
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          ref: ${{ github.sha }}
          persist-credentials: false
      - name: Publish exact successful main artifact
        env:
          GH_TOKEN: ${{ github.token }}
          RELEASE_VERSION: ${{ inputs.version }}
        run: python3 tools/publish_release.py
"""


def check_contract(text: str) -> None:
    if text.count("\njobs:\n") != 1 or text.split("\njobs:\n")[0].strip() != HEADER:
        raise ValueError("Reviewed events, concurrency or permissions changed")
    if re.search(r"(?m)^\s*(continue-on-error|shell):", text):
        raise ValueError("Checks may not mask failures or replace reviewed shell behavior")
    pairs = re.split(r"(?m)^  ([a-z][a-z0-9-]*):\n", text.split("\njobs:\n", 1)[1])
    names = pairs[1::2]
    if names != ["source-and-python", "rust-models", "isolated-build", "security", "desktop-ui", "ci-gate"]:
        raise ValueError("Reviewed job set changed")
    jobs = dict(zip(names, pairs[2::2]))

    def steps(job: str) -> list[str]:
        return re.split(r"(?m)^      - ", jobs[job])[1:]

    def step(job: str, identifier: str) -> str:
        found = [s for s in steps(job) if re.search(rf"(?m)^(?:        )?id: {re.escape(identifier)}$", s)]
        if len(found) != 1:
            raise ValueError("Required step missing or duplicated")
        return found[0]

    def command(job: str, identifier: str, value: str, condition: str | None) -> None:
        value_step = step(job, identifier)
        if re.findall(r"(?m)^        run: (.+)$", value_step) != [value]:
            raise ValueError("Reviewed command or single-command exit handling changed")
        actual = re.findall(r"(?m)^        if: (.+)$", value_step)
        if actual != ([] if condition is None else [condition]):
            raise ValueError("Reviewed prerequisite/failure condition changed")

    for name in names:
        checkout = step(name, "checkout")
        pins = re.findall(r"(?m)^(?:        )?uses: actions/checkout@([0-9a-f]{40})(?: #.*)?$", checkout)
        if pins != [CHECKOUT] or "          persist-credentials: false\n" not in checkout:
            raise ValueError("Checkout identity or credential boundary changed")
        if re.search(r"(?m)^          (ref|repository|token):", checkout):
            raise ValueError("Checkout must use the triggering repository/ref")
        if re.search(r"(?m)^    (if|needs):", jobs[name]) and name != "ci-gate":
            raise ValueError("Validation jobs must not be conditionally omitted")
    source = "source-and-python"
    command(source, "revision", 'git log -1 --format="%H %T"', None)
    for identifier, value in [
        ("spec", "python3 tools/spec_index.py check"),
        ("dependency_review", "python3 tools/check_dependencies.py"),
        ("workflow_contract", "python3 tools/ci_contract.py"),
        ("source_evidence", "python3 tools/ci_source.py"),
    ]:
        command(source, identifier, value, SOURCE_IF)
    command(source, "proof_toolchain", "rustup toolchain install 1.90.0 --profile minimal",
            SOURCE_IF[:-3] + " && steps.dependency_review.outcome == 'success' }}")
    command(source, "python", "python3 -m unittest discover -s tests/python -v",
            SOURCE_IF[:-3] + " && steps.proof_toolchain.outcome == 'success' }}")
    sequence = steps(source)
    if not sequence.index(step(source, "dependency_review")) < sequence.index(step(source, "proof_toolchain")) < sequence.index(step(source, "python")):
        raise ValueError("Execution-proof acquisition must follow dependency review")
    upload = step(source, "source_upload")
    required_upload = f"""uses: actions/upload-artifact@{UPLOAD} # v7.0.1, Node 24; source evidence only
        id: source_upload
        if: ${{{{ !cancelled() && steps.source_evidence.outcome == 'success' }}}}
        with:
          name: source-evidence-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: ${{{{ runner.temp }}}}/ca-source-evidence/
          if-no-files-found: error
          retention-days: 7
          overwrite: false
          include-hidden-files: false
"""
    if upload != required_upload:
        raise ValueError("Source-only artifact boundary changed")
    command(source, "assess", "python3 tools/ci_gate.py source", ASSESS_IF)
    rust = "rust-models"
    if "      fail-fast: false\n" not in jobs[rust] or "        os: [ubuntu-24.04, windows-latest, macos-latest]\n" not in jobs[rust]:
        raise ValueError("All three independent Rust lanes must remain")
    if re.search(r"(?m)^        (exclude|include):", jobs[rust]):
        raise ValueError("Unreviewed matrix override")
    command(rust, "revision", 'git log -1 --format="%H %T"', None)
    command(rust, "dependency_review", "python tools/check_dependencies.py", None)
    command(rust, "toolchain", "rustup toolchain install 1.90.0 --profile minimal --component rustfmt --component clippy", None)
    command(rust, "dependencies", "cargo fetch --locked", None)
    ordered = steps(rust)
    if not ordered.index(step(rust, "dependency_review")) < ordered.index(step(rust, "toolchain")) < ordered.index(step(rust, "dependencies")):
        raise ValueError("Review must precede acquisition")
    toolchain_if = "${{ !cancelled() && steps.toolchain.outcome == 'success' }}"
    command(rust, "compiler", "rustc --version --verbose", toolchain_if)
    command(rust, "host", "python tools/ci_gate.py host", SOURCE_IF)
    command(rust, "format", "cargo fmt --all -- --check", toolchain_if[:-3] + " && matrix.profile == 'debug' }}")
    for identifier, value in [
        ("tests", "python tools/ci_workspace_tests.py ${{ matrix.profile }}"),
        ("doctests", "cargo test --workspace --doc --no-fail-fast --locked --offline"),
        ("release_build", "cargo build --workspace --release --locked --offline"),
        ("clippy", "cargo clippy --workspace --all-targets --locked --offline -- -D warnings"),
    ]:
        condition = RUST_IF
        if identifier in {"doctests", "clippy"}:
            condition = condition[:-3] + " && matrix.profile == 'debug' }}"
        elif identifier == "release_build":
            condition = condition[:-3] + " && matrix.profile == 'release' }}"
        command(rust, identifier, value, condition)
    native_if = RUST_IF[:-3] + " && matrix.os == 'windows-latest' }}"
    command(rust, "supervisor_tests", "python tools/check_windows_process.py", native_if)
    command(rust, "native_tests", "python tools/run_native_tests.py ${{ matrix.profile }}",
            native_if[:-3] + " && steps.supervisor_tests.outcome == 'success' }}")
    if "          CI_MATRIX_OS: ${{ toJSON(matrix.os) }}" in text:
        raise ValueError("Lane must retain the actual unquoted value")
    if "          CI_MATRIX_OS: ${{ matrix.os }}\n" not in step(rust, "assess"):
        raise ValueError("Rust outcome gate must include native evidence for Windows")
    command(rust, "assess", "python tools/ci_gate.py rust", ASSESS_IF)
    if "        profile: [debug, release]\n" not in jobs[rust] or "          CI_PROFILE: ${{ matrix.profile }}\n" not in step(rust, "assess"):
        raise ValueError("Both profile shards must run and report their identity")
    package_if = "${{ success() && matrix.os == 'windows-latest' && matrix.profile == 'release' }}"
    command(rust, "package", "python tools/package_windows.py", package_if)
    command(rust, "package_scan", "python tools/security_scan.py package", package_if)
    for required in ["if-no-files-found: error", "retention-days: 90", "path: dist/windows-x64/", "if: " + package_if]:
        if required not in step(rust, "package_upload"):
            raise ValueError("Build artifact publication boundary changed")
    if "        scanner: [trivy, opengrep]\n" not in jobs["security"]:
        raise ValueError("Both security scanners are mandatory")
    command("security", "scan", "python3 tools/security_scan.py ${{ matrix.scanner }}", None)
    for required in ["npm ci --ignore-scripts --no-audit --no-fund", "node node_modules/playwright/cli.js install --with-deps chromium", "npm run test:ui"]:
        if "        run: " + required + "\n" not in jobs["desktop-ui"]:
            raise ValueError("Browser behavior checks are mandatory")
    if "    timeout-minutes: 15\n" not in jobs["desktop-ui"]:
        raise ValueError("Browser behavior timeout must cover a cold Chromium install")
    if "uses: actions/cache@" in jobs["isolated-build"]:
        raise ValueError("The isolated build must use a fresh target")
    for action in re.findall(r"uses: ([^ #\n]+)", text):
        if not re.fullmatch(r"[a-zA-Z0-9_/-]+@[0-9a-f]{40}", action):
            raise ValueError("All actions must be pinned to commits")
    isolated = "isolated-build"
    command(isolated, "revision", 'git log -1 --format="%H %T"', None)
    command(isolated, "dependency_review", "python tools/check_dependencies.py", None)
    command(isolated, "toolchain", "rustup toolchain install 1.90.0 --profile minimal --component rustfmt --component clippy", None)
    command(isolated, "dependencies", "cargo fetch --locked", None)
    sequence = steps(isolated)
    if not sequence.index(step(isolated, "dependency_review")) < sequence.index(step(isolated, "toolchain")) < sequence.index(step(isolated, "dependencies")):
        raise ValueError("Isolated acquisition must remain reviewed")
    command(isolated, "isolated", "python3 tools/ci_isolated.py", RUST_IF)
    command(isolated, "assess", "python3 tools/ci_gate.py isolated", ASSESS_IF)
    if "    runs-on: ubuntu-24.04\n" not in jobs[isolated]:
        raise ValueError("Network isolation requires the reviewed standard VM lane")
    gate = jobs["ci-gate"].split("    steps:\n", 1)[0]
    if gate != "    name: CI gate\n    needs: [source-and-python, rust-models, isolated-build, security, desktop-ui]\n    if: ${{ always() }}\n    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n":
        raise ValueError("Aggregate gate must always assess every prerequisite")
    command("ci-gate", "assess", "python3 tools/ci_gate.py jobs", None)
    for job in [source, rust, isolated]:
        if "          CI_RESULTS: ${{ toJSON(steps) }}\n" not in step(job, "assess"):
            raise ValueError("Step outcomes must be actual context, not assertions")
    if "          CI_RESULTS: ${{ toJSON(needs) }}\n" not in step("ci-gate", "assess"):
        raise ValueError("Job results must be actual context, not assertions")
    if "          CI_MATRIX_OS: ${{ matrix.os }}\n" not in step(rust, "host"):
        raise ValueError("Host check must use the selected lane")

    expected_ids = {
        source: {"checkout", "revision", "spec", "dependency_review", "workflow_contract", "proof_toolchain", "python", "source_evidence", "source_upload", "assess"},
        rust: {"checkout", "revision", "dependency_review", "toolchain", "dependencies", "compiler", "host", "format", "tests", "doctests", "release_build", "cache", "package", "package_scan", "package_upload", "supervisor_tests", "native_tests", "clippy", "assess"},
        isolated: {"checkout", "revision", "dependency_review", "toolchain", "dependencies", "isolated", "assess"},
        "ci-gate": {"checkout", "assess"},
    }
    for job, wanted in expected_ids.items():
        ids = re.findall(r"(?m)^        id: ([a-z_]+)$", jobs[job])
        if len(ids) != len(wanted) or set(ids) != wanted or len(steps(job)) != len(wanted):
            raise ValueError("Unreviewed, omitted or duplicate step")
        actions = re.findall(r"(?m)^(?:      - |        )uses: ([^ #]+)", jobs[job])
        expected_actions = ["actions/checkout@" + CHECKOUT]
        if job == source:
            expected_actions.append("actions/upload-artifact@" + UPLOAD)
        if job == rust:
            expected_actions.extend(["actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9", "actions/upload-artifact@" + UPLOAD])
        if actions != expected_actions:
            raise ValueError("Unreviewed third-party action")


def check(root: Path = ROOT) -> None:
    workflows = root / ".github/workflows"
    if sorted(p.name for p in workflows.iterdir() if p.suffix in {".yml", ".yaml"}) != ["artifact-retention.yml", "ci.yml", "release.yml", "security-scheduled.yml"]:
        raise ValueError("Unreviewed workflow or temporary preparation workflow")
    toolchain = tomllib.loads((root / "rust-toolchain.toml").read_text(encoding="utf-8"))
    if toolchain != {"toolchain": {"channel": "1.90.0", "profile": "minimal", "components": ["rustfmt", "clippy"]}}:
        raise ValueError("Toolchain and workflow contract must change together")
    check_contract((workflows / "ci.yml").read_text(encoding="utf-8"))
    if (workflows / "release.yml").read_text(encoding="utf-8") != RELEASE_WORKFLOW:
        raise ValueError("Versioned release workflow contract changed")


if __name__ == "__main__":
    check()
    print("CI workflow contract: passed")
