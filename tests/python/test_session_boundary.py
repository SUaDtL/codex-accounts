"""CA-05C construction and scope regressions supplement executable state tests."""
from pathlib import Path
import re
import unittest
from tools import spec_index

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'crates/runtime/src/session.rs'


class SessionBoundaryTests(unittest.TestCase):
    def test_test_only_construction_without_reset_or_qualification(self):
        source = SOURCE.read_text(encoding='utf-8')
        self.assertIn('#![forbid(unsafe_code)]', source)
        self.assertIn('#[cfg(test)]\n    fn for_test(', source)
        for forbidden in ('pub fn new(', 'pub fn for_test(', 'pub fn reset(',
                          'impl Default for ProtocolSession', 'impl Clone for ProtocolSession',
                          'Serialize', 'Deserialize', 'unsafe {', 'allow(dead_code)',
                          'std::fs', 'std::net', 'std::process', 'thread::sleep'):
            self.assertNotIn(forbidden, source)
        self.assertIn('compile_fail', source)
        self.assertIn('Arc::ptr_eq(&self.owner, &other.owner)', source)

    def test_error_and_observation_types_do_not_claim_native_authority(self):
        source = SOURCE.read_text(encoding='utf-8')
        self.assertIn('self.failure.get_or_insert(error)', source)
        self.assertIn('self.pending.clear()', source)
        self.assertIn('self.notifications = 0', source)
        self.assertIn('now.checked_duration_since(self.last)', source)
        for forbidden in ('CredentialAcceptance', 'DesktopIdentity', 'RootKey',
                          'qualified: bool', 'RequestId(pub', 'pub owner:', 'pub sequence:'):
            self.assertNotIn(forbidden, source)
        self.assertIn('Method::LoginStart | Method::LoginCancel | Method::Initialized', source)

    def test_budgets_match_specification_and_fail_at_equality(self):
        source = SOURCE.read_text(encoding='utf-8')
        for expected in ('INITIALIZATION_BUDGET: Duration = Duration::from_secs(10)',
                         'TOTAL_BUDGET: Duration = Duration::from_secs(30)',
                         'REQUEST: Duration = Duration::from_secs(15)',
                         'MAX_NOTIFICATIONS: usize = 32', 'now >= self.total_deadline',
                         'now >= self.initialize_deadline', 'now >= p.request.deadline'):
            self.assertIn(expected, source)
        model, _ = spec_index.model()
        protocol = spec_index.symbols(model)['SPEC-PROTOCOL']
        table = next(b for b in protocol['blocks'] if b.get('headers', [])[:1] == ['Operation'])
        self.assertIn(['Runtime initialization', '10 seconds', 'Stop/reap owned helper; show typed error.'], table['rows'])
        self.assertTrue(any('15 seconds per request; 30 seconds total non-login RPC' in row for row in table['rows']))

    def test_regular_state_tests_do_not_require_real_time_or_native_data(self):
        source = SOURCE.read_text(encoding='utf-8')
        tests = (SOURCE.parent / 'session_tests.rs').read_text(encoding='utf-8')
        self.assertIn('#[path = "session_tests.rs"]', source)
        self.assertEqual(len(re.findall(r'#\[test\]', tests)), 24)
        self.assertNotIn('#[ignore', tests)
        for forbidden in ('sleep(', 'std::fs', 'std::net', 'std::process'):
            self.assertNotIn(forbidden, tests)
        for behavior in ('all_response_orders', 'completed_requests_do_not_reset',
                         'response_one_nanosecond_before', 'every_event_checks_expiry',
                         'local_ticket_owner_lifetime', 'truncated_protocol_eof'):
            self.assertIn(behavior, tests)


if __name__ == '__main__':
    unittest.main()
