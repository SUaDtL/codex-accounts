"""Native supervisor proof cannot pass with absent, skipped or expected failures."""
from contextlib import redirect_stdout
import io
import json
import unittest
from unittest.mock import patch
from tools import check_windows_process as gate
from tools import windows_process as owned
from tests.python import test_windows_process as fixtures


class ProcessGateTests(unittest.TestCase):
    def test_native_fixture_refuses_existing_cleanup_poison_without_launch(self):
        case = fixtures.NativeProcessTests(
            'test_actual_suspended_assignment_normal_output_and_closed_handles')
        with patch.object(owned, '_launch_blocked', True), \
                patch.object(owned, '_Process') as process, \
                patch.object(fixtures.subprocess, 'Popen') as popen:
            with self.assertRaisesRegex(AssertionError, 'owned_process_cleanup_poisoned$'):
                case.setUp()
            self.assertTrue(owned._launch_blocked)
            process.assert_not_called()
            popen.assert_not_called()

    def test_model_fixture_restores_existing_poison_and_invalid_calls_never_construct(self):
        case = fixtures.SupervisionTests('test_invalid_bounds_and_stdio_never_create_process')
        with patch.object(owned, '_launch_blocked', True), \
                patch.object(owned, '_Process') as process, \
                patch.object(fixtures.subprocess, 'Popen') as popen:
            try:
                case.setUp()
                self.assertFalse(owned._launch_blocked)
                case.test_invalid_bounds_and_stdio_never_create_process()
            finally:
                case.doCleanups()
            self.assertTrue(owned._launch_blocked)
            process.assert_not_called()
            popen.assert_not_called()

    def result(self, count):
        result = unittest.TestResult()
        result.testsRun = count
        return result

    def test_every_required_native_identity_must_execute_once(self):
        names = sorted(gate.REQUIRED_NATIVE)
        self.assertTrue(gate.assess(names, self.result(len(names))))
        for changed in ([], names[:-1], names + [names[0]]):
            self.assertFalse(gate.assess(changed, self.result(len(changed))))
        self.assertFalse(gate.assess(names, self.result(0)))

    def test_failure_skip_and_expected_failure_never_count_as_native_proof(self):
        names = sorted(gate.REQUIRED_NATIVE)
        for field in ('failures', 'errors', 'skipped', 'expectedFailures', 'unexpectedSuccesses'):
            result = self.result(len(names))
            getattr(result, field).append('SYNTHETIC_ONLY')
            with self.subTest(field=field):
                self.assertFalse(gate.assess(names, result))

    def test_failed_known_fixture_reports_only_id_and_fixed_exception_category(self):
        known = sorted(gate.REQUIRED_NATIVE)[0]
        class SyntheticFailure(unittest.TestCase):
            def id(self):
                return known
            def runTest(self):
                raise AssertionError('SYNTHETIC_SECRET C:\\private\\auth.json --token=SYNTHETIC')
        suite = unittest.TestSuite([SyntheticFailure()])
        output = io.StringIO()
        with patch.object(gate.sys, 'platform', 'win32'), \
                patch.object(gate.platform, 'machine', return_value='AMD64'), \
                patch.object(gate.unittest.defaultTestLoader, 'discover', return_value=suite), \
                patch.object(gate, 'identifiers', return_value=iter(sorted(gate.REQUIRED_NATIVE))), \
                redirect_stdout(output):
            self.assertEqual(gate.run(), 1)
        result = json.loads(output.getvalue())
        self.assertEqual(result['failure_diagnostics'], [
            {'test_id': known, 'category': 'failure', 'exception': 'assertion_failed'}])
        self.assertNotIn('SYNTHETIC', output.getvalue())
        self.assertNotIn('private', output.getvalue())

    def test_diagnostics_normalize_unknown_and_subtest_ids_without_stringifying_errors(self):
        known = sorted(gate.REQUIRED_NATIVE)[0]
        class PrivateError(Exception):
            def __str__(self):
                raise AssertionError('must not format private exception')
        class Synthetic(unittest.TestCase):
            def id(self):
                return 'SYNTHETIC_PRIVATE_PATH C:\\private\\auth.json'
            def runTest(self):
                raise PrivateError()
        result = unittest.TextTestRunner(stream=io.StringIO(), resultclass=gate.RedactedResult).run(
            unittest.TestSuite([Synthetic()]))
        self.assertEqual(result.failure_diagnostics, [
            {'test_id': 'unrecognized_fixture', 'category': 'error', 'exception': 'other_exception'}])
        self.assertEqual(result.errors[0][1], 'fixture_detail_redacted')
        class Subtests(unittest.TestCase):
            def id(self):
                return known
            def runTest(self):
                with self.subTest(path='SYNTHETIC_PRIVATE_PATH'):
                    self.fail('SYNTHETIC_SECRET')
        result = unittest.TextTestRunner(stream=io.StringIO(), resultclass=gate.RedactedResult).run(
            unittest.TestSuite([Subtests()]))
        self.assertEqual(result.failure_diagnostics, [
            {'test_id': known, 'category': 'subtest_failure', 'exception': 'assertion_failed'}])
        self.assertNotIn('SYNTHETIC', json.dumps(result.failure_diagnostics))

    def test_diagnostic_inventory_is_exact_and_failure_rows_are_bounded(self):
        suite = unittest.defaultTestLoader.discover(str(gate.ROOT / 'tests/python'),
                                                  pattern='test_windows_process.py')
        self.assertEqual(set(gate.identifiers(suite)), gate.KNOWN_TESTS)
        self.assertEqual(len(gate.KNOWN_TESTS), 12)
        result = gate.RedactedResult(io.StringIO(), False, 0)
        test = unittest.FunctionTestCase(lambda: None)
        for _ in range(100):
            result.addSkip(test, 'SYNTHETIC_PRIVATE_REASON')
        self.assertEqual(len(result.failure_diagnostics), 12)
        self.assertTrue(result.diagnostics_truncated)
        self.assertNotIn('SYNTHETIC', json.dumps(result.failure_diagnostics))

    def test_passing_summary_keeps_exact_previous_shape_and_counts(self):
        class SyntheticPass(unittest.TestCase):
            def __init__(self, name):
                super().__init__()
                self.name = name
            def id(self):
                return self.name
            def runTest(self):
                pass
        suite = unittest.TestSuite(SyntheticPass(name) for name in sorted(gate.KNOWN_TESTS))
        output = io.StringIO()
        with patch.object(gate.sys, 'platform', 'win32'), \
                patch.object(gate.platform, 'machine', return_value='AMD64'), \
                patch.object(gate.unittest.defaultTestLoader, 'discover', return_value=suite), \
                redirect_stdout(output):
            self.assertEqual(gate.run(), 0)
        self.assertEqual(json.loads(output.getvalue()), {
            'windows_supervisor_tests': 12, 'required_native_fixtures': 4,
            'result': 'passed', 'desktop_qualification': 'not_established'})


if __name__ == '__main__':
    unittest.main()
