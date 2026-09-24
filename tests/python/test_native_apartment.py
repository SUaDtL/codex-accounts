"""Source guard supplements actual repeated/concurrent Windows apartment tests."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ApartmentLifetimeTests(unittest.TestCase):
    def test_com_cleanup_is_stack_owned_not_a_windows_tls_destructor(self):
        text = (ROOT / 'crates/vault-storage/src/native/sync.rs').read_text()
        code = re.sub(r'//[^\n]*', '', text)
        self.assertNotIn('thread_local!', code)
        self.assertIn('let _apartment = Apartment::enter()?;', code)
        self.assertLess(code.index('let _apartment = Apartment::enter()?;'),
                        code.index('let factory:'))
        self.assertIn('impl Drop for Apartment', code)
        self.assertIn('RoUninitialize()', code)
        self.assertIn('PhantomData<std::rc::Rc<()>>', code)
        self.assertIn('native_inventory_thread_exit_balances_com_before_tls_teardown', code)
        self.assertIn('native_repeated_inventory_retains_the_owned_thread_apartment', code)
        self.assertNotIn('#[ignore', code)


if __name__ == '__main__':
    unittest.main()
