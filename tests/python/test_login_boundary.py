"""CA-05D source/authority contracts supplement actual Rust transition tests."""
from pathlib import Path
import unittest
from tools import spec_index
ROOT = Path(__file__).resolve().parents[2]
RUNTIME = ROOT / 'crates/runtime/src'

class LoginBoundaryTests(unittest.TestCase):
    def test_construction_ids_and_lane_are_sealed(self):
        s = (RUNTIME / 'session_login.rs').read_text(encoding='utf-8')
        self.assertIn('#[cfg(test)]\n    fn for_login_test', s)
        self.assertIn('#[cfg(test)]\n    fn login_id_for_test', s)
        self.assertIn('bytes: Zeroizing<Vec<u8>>', s)
        self.assertIn('Arc::ptr_eq', s)
        self.assertIn('id.bytes.len() > 256', s)
        self.assertGreaterEqual(s.count('compile_fail'), 2)
        for bad in ('pub fn new', 'pub fn for_login_test', 'pub fn login_id_for_test',
                    'pub fn as_bytes', 'pub owner:', 'pub bytes:', 'Deserialize', 'Serialize',
                    'unsafe {', 'allow(dead_code)', 'std::fs', 'std::net', 'std::process', 'env::var'):
            self.assertNotIn(bad, s)
        parent = (RUNTIME / 'session.rs').read_text(encoding='utf-8')
        self.assertIn('Method::LoginStart | Method::LoginCancel | Method::Initialized', parent)
        self.assertRegex(s, r'self\s*\.issue\(\s*Method::LoginStart')
        self.assertIn('self.issue(Method::LoginCancel', s)
        self.assertIn('self.matching_sent(request)', s)

    def test_budget_matches_spec_without_new_native_evidence(self):
        s = (RUNTIME / 'session_login.rs').read_text(encoding='utf-8')
        self.assertIn('LOGIN_BUDGET: Duration = Duration::from_secs(600)', s)
        self.assertIn('CANCELLATION_BUDGET: Duration = Duration::from_secs(5)', s)
        self.assertIn('now >= deadline', s)
        model, _ = spec_index.model()
        table = next(b for b in spec_index.symbols(model)['SPEC-PROTOCOL']['blocks']
                     if b.get('headers', [])[:1] == ['Operation'])
        self.assertIn(['Browser login', '10 minutes overall; cancel available',
                       'Cancel matching login, stop/reap, cleanup staging.'], table['rows'])
        for bad in ('CredentialAcceptance', 'DesktopIdentity', 'RootKey', 'qualified: bool'):
            self.assertNotIn(bad, s)

    def test_tests_execute_real_transitions_without_native_or_sleep_substitutes(self):
        s = (RUNTIME / 'session_login_tests.rs').read_text(encoding='utf-8')
        for behavior in ('all_six_start_completion_cancel_orders', 'wrong_login_id_refuses',
                         'same_id_bytes_from_another_session', 'combined_account_and_early_completion',
                         'exact_login_deadline_wins', 'malformed_transport_and_eof_during_cancel',
                         'cancellation_cannot_bypass_the_lifetime', 'cancellation_wait_is_capped'):
            self.assertIn(behavior, s)
        for bad in ('#[ignore', 'sleep(', 'std::fs', 'std::net', 'std::process'):
            self.assertNotIn(bad, s)
        self.assertIn('#[path = "session_login_tests.rs"]', (RUNTIME / 'session_login.rs').read_text())

    def test_first_error_and_cancellation_evidence_remain_separate(self):
        s = (RUNTIME / 'session_login.rs').read_text(encoding='utf-8')
        self.assertIn('l.cancel_error.get_or_insert(error)', s)
        self.assertIn('pub fn cancellation_state', s)
        self.assertIn('pub fn cancellation_error', s)
        self.assertIn('Never reverse a cancellation intent', s)
        self.assertIn('self.notifications + l.early.len() >= MAX_NOTIFICATIONS', s)
        self.assertIn('self.phase = SessionPhase::Stopped', s)
        self.assertIn('Err(self.failure.unwrap_or(SessionError::Sequence))', s)

if __name__ == '__main__':
    unittest.main()
