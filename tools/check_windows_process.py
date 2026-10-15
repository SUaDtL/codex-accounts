"""Execute the Windows supervisor fixtures; missing or skipped proof refuses."""
from __future__ import annotations

import io
import json
from pathlib import Path
import platform
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_NATIVE = frozenset('test_windows_process.NativeProcessTests.' + name for name in (
    'test_actual_assignment_failure_never_resumes_and_reaps_suspended_root',
    'test_actual_suspended_assignment_normal_output_and_closed_handles',
    'test_timeout_and_lingering_descendant_are_killed_with_empty_job',
    'test_unrelated_process_and_ambient_inheritable_handle_are_preserved',
))
KNOWN_TESTS = REQUIRED_NATIVE | frozenset('test_windows_process.SupervisionTests.' + name for name in (
    'test_success_assigns_before_resume_and_proves_empty_before_closing',
    'test_assignment_and_resume_refusals_cleanup_before_return',
    'test_timeout_is_failure_after_owned_cleanup_and_output_is_readable',
    'test_startup_counts_against_deadline_and_expiry_never_resumes',
    'test_cleanup_failure_blocks_every_later_launch',
    'test_nonempty_job_exhausts_cleanup_bound_and_blocks_launch',
    'test_check_failure_is_raised_only_after_cleanup',
    'test_invalid_bounds_and_stdio_never_create_process',
))


def exception_category(err) -> str:
    # Class identity/inheritance only: never stringify exception values, names,
    # tracebacks, process argv, output, Windows messages or assertion payloads.
    kind = err[0]
    for accepted, category in (
            (AssertionError, 'assertion_failed'),
            (subprocess.TimeoutExpired, 'timeout'),
            (subprocess.CalledProcessError, 'subprocess_failed'),
            (OSError, 'os_error'), (ValueError, 'value_error'),
            (TypeError, 'type_error'), (RuntimeError, 'runtime_error')):
        if isinstance(kind, type) and issubclass(kind, accepted):
            return category
    return 'other_exception'


class RedactedResult(unittest.TextTestResult):
    """Bounded public diagnostics independent of unittest's raw traceback text."""
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.failure_diagnostics = []
        self.diagnostics_truncated = False

    def _exc_info_to_string(self, err, test):
        return 'fixture_detail_redacted'

    def _record(self, test, category, err=None):
        if len(self.failure_diagnostics) >= len(KNOWN_TESTS):
            self.diagnostics_truncated = True
            return
        # Subtest IDs contain parameter reprs. Attribute to the enclosing known
        # test without reading/formatting parameters or exposing an unknown ID.
        if isinstance(test, unittest.case._SubTest):
            test = test.test_case
        try:
            name = test.id()
        except Exception:
            name = None
        safe_name = name if isinstance(name, str) and name in KNOWN_TESTS else 'unrecognized_fixture'
        self.failure_diagnostics.append({
            'test_id': safe_name, 'category': category,
            'exception': exception_category(err) if err is not None else 'not_applicable',
        })

    def addFailure(self, test, err):
        self._record(test, 'failure', err)
        super().addFailure(test, err)

    def addError(self, test, err):
        self._record(test, 'error', err)
        super().addError(test, err)

    def addSubTest(self, test, subtest, err):
        if err is not None:
            self._record(test, 'subtest_failure', err)
        super().addSubTest(test, subtest, err)

    def addSkip(self, test, reason):
        self._record(test, 'skipped')
        super().addSkip(test, reason)

    def addExpectedFailure(self, test, err):
        self._record(test, 'expected_failure', err)
        super().addExpectedFailure(test, err)

    def addUnexpectedSuccess(self, test):
        self._record(test, 'unexpected_success')
        super().addUnexpectedSuccess(test)


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
    # Runner text stays private; public failures use only allowlisted identities
    # and fixed categories. No raw fixture tracebacks or paths are forwarded.
    result = unittest.TextTestRunner(stream=io.StringIO(), resultclass=RedactedResult).run(suite)
    passed = assess(names, result)
    summary = {'windows_supervisor_tests': result.testsRun,
               'required_native_fixtures': len(REQUIRED_NATIVE),
               'result': 'passed' if passed else 'failed',
               'desktop_qualification': 'not_established'}
    if not passed:
        summary['failure_diagnostics'] = result.failure_diagnostics
        summary['diagnostics_truncated'] = result.diagnostics_truncated
    print(json.dumps(summary))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(run())
