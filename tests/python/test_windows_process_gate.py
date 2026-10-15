"""Native supervisor proof cannot pass with absent, skipped or expected failures."""
import unittest
from tools import check_windows_process as gate


class ProcessGateTests(unittest.TestCase):
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


if __name__ == '__main__':
    unittest.main()
