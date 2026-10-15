"""Execute the Windows supervisor fixtures; missing or skipped proof refuses."""
from __future__ import annotations

import io
import json
from pathlib import Path
import platform
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_NATIVE = frozenset('test_windows_process.NativeProcessTests.' + name for name in (
    'test_actual_assignment_failure_never_resumes_and_reaps_suspended_root',
    'test_actual_suspended_assignment_normal_output_and_closed_handles',
    'test_timeout_and_lingering_descendant_are_killed_with_empty_job',
    'test_unrelated_process_and_ambient_inheritable_handle_are_preserved',
))


def assess(names: list[str], result: unittest.TestResult) -> bool:
    return (REQUIRED_NATIVE.issubset(names) and len(names) == len(set(names))
            and result.testsRun == len(names) and result.wasSuccessful()
            and not any((result.failures, result.errors, result.skipped,
                         result.expectedFailures, result.unexpectedSuccesses)))


def identifiers(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from identifiers(item)
        else:
            yield item.id()


def run() -> int:
    if sys.platform != 'win32' or platform.machine().lower() not in {'amd64', 'x86_64'}:
        print('Windows supervisor evidence requires Windows x64')
        return 1
    sys.path.insert(0, str(ROOT))
    suite = unittest.defaultTestLoader.discover(str(ROOT / 'tests/python'), pattern='test_windows_process.py')
    names = list(identifiers(suite))
    if not REQUIRED_NATIVE.issubset(names):
        print('Windows supervisor evidence missing required fixture identities')
        return 1
    # Fixed summary only; never forward raw fixture tracebacks or paths.
    result = unittest.TextTestRunner(stream=io.StringIO()).run(suite)
    passed = assess(names, result)
    print(json.dumps({'windows_supervisor_tests': result.testsRun,
                      'required_native_fixtures': len(REQUIRED_NATIVE),
                      'result': 'passed' if passed else 'failed',
                      'desktop_qualification': 'not_established'}))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(run())
