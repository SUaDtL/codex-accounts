"""CA-05E source boundaries supplement real channel/scheduler execution in Rust."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
RUNTIME = ROOT / "crates/runtime/src"


class DriverBoundaryTests(unittest.TestCase):
    def test_private_wiring_and_test_only_endpoint_construction(self):
        driver = (RUNTIME / "driver.rs").read_text(encoding="utf-8")
        io = (RUNTIME / "driver_io.rs").read_text(encoding="utf-8")
        parent = (RUNTIME / "session_login.rs").read_text(encoding="utf-8")
        self.assertIn('#[path = "driver.rs"]\nmod driver;', parent)
        self.assertIn('struct TransportDriver {', driver)
        self.assertIn('compile_fail', driver)
        self.assertRegex(io, r'#\[cfg\(test\)\]\s*pub\(super\) fn pair\(')
        self.assertIn('pub(super) fn cancellation(', io)
        for source in (driver, io):
            for forbidden in ('pub struct', 'pub fn', 'pub mod', 'pub use',
                              'Deserialize', 'Serialize', 'unsafe {',
                              'std::fs', 'std::net', 'std::process', 'env::var'):
                self.assertNotIn(forbidden, source)
            self.assertIn('#![forbid(unsafe_code)]', source)
        self.assertNotIn('allow(dead_code)', parent)

    def test_existing_session_owns_tickets_deadlines_and_outcomes(self):
        source = (RUNTIME / "driver.rs").read_text(encoding="utf-8")
        for required in ('session.poll(', '.poll_cancellation(', 'session.sent(',
                         'session.initialized_sent(', '.finish(Instant::now())',
                         'session.cancellation_error()', 'session.cancel(now)'):
            self.assertIn(required, source)
        for forbidden in ('LOGIN_BUDGET:', 'TOTAL_BUDGET:', 'fn issue(',
                          'CredentialAcceptance', 'DesktopIdentity', 'qualified: bool'):
            self.assertNotIn(forbidden, source)

    def test_data_and_wakeup_channels_are_bounded_without_blocking_writes(self):
        source = (RUNTIME / "driver_io.rs").read_text(encoding="utf-8")
        driver = (RUNTIME / "driver.rs").read_text(encoding="utf-8")
        self.assertIn('CHUNK_BYTES: usize = 4096', source)
        self.assertIn('QUEUE_CHUNKS: usize = 16', source)
        self.assertIn('sync_channel(QUEUE_CHUNKS)', source)
        self.assertIn('sender.try_send(chunk)', source)
        self.assertIn('requested.store(true, Ordering::Release)', source)
        self.assertIn('self.cancel.load(Ordering::Acquire)', driver)
        self.assertIn('self.wake.recv_timeout(remaining)', driver)
        self.assertRegex(driver, r'deadline\s*\.saturating_duration_since\(Instant::now\(\)\)')
        for forbidden in ('.send(', '.recv()', 'read_to_end', 'write_all', 'thread::spawn'):
            self.assertNotIn(forbidden, source + driver)

    def test_secret_buffers_have_no_debug_payload_and_single_frame_lease(self):
        source = (RUNTIME / "driver.rs").read_text(encoding="utf-8")
        io = (RUNTIME / "driver_io.rs").read_text(encoding="utf-8")
        self.assertIn('bytes: Zeroizing<Vec<u8>>', source)
        self.assertIn('bytes: Zeroizing<Vec<u8>>', io)
        self.assertIn('.zeroize()', source)
        self.assertIn('self.lease.upgrade().is_some()', source)
        self.assertIn('DriverFault::OutstandingFrame', source)
        self.assertIn('STDERR_TOTAL_LIMIT: usize = MAX_PENDING_BYTES', source)
        self.assertIn('Arc::ptr_eq(&self.reader, &driver.reader)', source)
        self.assertIn('Stream::Output', source)
        self.assertIn('Stream::Error', source)
        for text in ('ReaderFrame([REDACTED])', 'TransportDriver([REDACTED])'):
            self.assertIn(text, source)

    def test_scheduler_cases_execute_with_owned_joined_synthetic_peers(self):
        source = (RUNTIME / "driver_tests.rs").read_text(encoding="utf-8")
        for case in ('silent_endpoints_wake', 'continuous_stderr_cannot_extend',
                     'blocked_input_times_out', 'released_input_credit_wakes',
                     'foreign_session_reader', 'cancellation_drives_matching',
                     'cancellation_wait_has_its_own', 'one_stream_eof',
                     'invalid_stdout_discards', 'a_closed_driver_cannot_clear'):
            self.assertIn(case, source)
        self.assertIn('thread::scope(', source)
        self.assertIn('.join().unwrap()', source)
        self.assertIn('Duration::from_secs(10)', source)
        for forbidden in ('#[ignore', 'std::process', 'std::fs', 'std::net'):
            self.assertNotIn(forbidden, source)
        self.assertNotRegex(source, re.compile(r'pub\s+fn\s+harness'))


if __name__ == "__main__":
    unittest.main()
