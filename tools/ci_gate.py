"""Fail closed on missing, skipped or unsuccessful CI checks; never grant product authority."""
from __future__ import annotations

import json
import os
import platform
import sys
from typing import Any

MAX_INPUT_BYTES = 65536
REQUIRED = {
    "source": ("checkout", "revision", "spec", "dependency_review", "workflow_contract", "python"),
    "rust": ("checkout", "revision", "dependency_review", "toolchain", "dependencies",
             "compiler", "host", "format", "tests", "doctests", "release_build", "clippy"),
    "jobs": ("source-and-python", "rust-models"),
}


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate result field")
        result[key] = value
    return result


def assess(scope: str, payload: str) -> bool:
    """Only exact success of every required result passes. Extra results cannot replace one."""
    if scope not in REQUIRED or len(payload.encode("utf-8")) > MAX_INPUT_BYTES:
        return False
    try:
        results = json.loads(payload, object_pairs_hook=_unique_object)
    except (ValueError, RecursionError, TypeError):
        return False
    if not isinstance(results, dict):
        return False
    field = "result" if scope == "jobs" else "outcome"
    return all(
        isinstance(results.get(name), dict) and results[name].get(field) == "success"
        for name in REQUIRED[scope]
    )


def host_matches(lane: str, system: str, machine: str) -> bool:
    """Prevent Windows native tests silently compiling out on a different runner architecture.

    macOS is a portable-library lane here, not a qualified Keychain/Desktop adapter.
    """
    expected = {
        "ubuntu-24.04": ("Linux", {"x86_64", "amd64"}),
        "windows-latest": ("Windows", {"x86_64", "amd64"}),
        "macos-latest": ("Darwin", {"arm64", "aarch64", "x86_64"}),
    }
    pair = expected.get(lane)
    return pair is not None and system == pair[0] and machine.lower() in pair[1]


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in {*REQUIRED, "host"}:
        print("CI gate: invalid invocation")
        return 2
    scope = sys.argv[1]
    if scope == "host":
        passed = host_matches(os.environ.get("CI_MATRIX_OS", ""), platform.system(), platform.machine())
    else:
        passed = assess(scope, os.environ.get("CI_RESULTS", ""))
    # Never echo result JSON, exception text, environment values or arbitrary input.
    print("CI gate: passed (not product qualification)" if passed else "CI gate: missing or unsuccessful required evidence")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
