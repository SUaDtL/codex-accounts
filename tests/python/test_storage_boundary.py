"""Storage source guards supplement (not replace) executable Rust/native tests."""
import hashlib
import json
from pathlib import Path
import tempfile
import shutil
import tomllib
import unittest
from tools import check_dependencies
ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / 'crates/vault-storage'

class StorageBoundary(unittest.TestCase):
    def test_native_surface_is_private_and_safely_scoped(self):
        lib = (CRATE/'src/lib.rs').read_text()
        self.assertEqual(lib.count('allow(unsafe_code)'), 1)
        self.assertIn('#![deny(unsafe_code)]', lib)
        self.assertIn('mod native;', lib)
        for name in ['codec.rs', 'engine.rs', 'records.rs', 'control_repair.rs']:
            text = (CRATE/'src'/name).read_text()
            self.assertIn('#![forbid(unsafe_code)]', text)
            self.assertNotIn('unsafe {', text)
        for name in ['build.rs', 'src/main.rs', 'src/bin', 'examples']:
            self.assertFalse((CRATE/name).exists())
        self.assertEqual(tomllib.loads((CRATE/'Cargo.toml').read_text())['lints']['rust']['unsafe_code'], 'deny')

    def test_no_live_credential_or_network_dispatch(self):
        for p in (CRATE/'src').rglob('*.rs'):
            if p.name.endswith('tests.rs'):
                continue
            text = p.read_text()
            for word in ['std::net', 'std::process', 'Command::', 'auth.json', 'cap_sid', 'println!', 'eprintln!', 'SetNamedSecurityInfoW', 'SetSecurityInfo(']:
                self.assertNotIn(word,text,(p.name,word))
        native = (CRATE/'src/native.rs').read_text()
        for required in ['FILE_FLAG_OPEN_REPARSE_POINT', 'FILE_FLAG_WRITE_THROUGH', 'SE_DACL_PROTECTED', 'CREATE_NEW', 'GetSecurityInfo(']:
            self.assertIn(required,native)
        sync=(CRATE/'src/native/sync.rs').read_text()
        self.assertIn('CF_SYNC_ROOT_INFO_BASIC',sync)
        self.assertIn('factory.vtable().GetCurrentSyncRoots',sync)
        self.assertIn('RoGetActivationFactory',sync)
        self.assertNotIn('StorageProviderSyncRootManager::GetCurrentSyncRoots',sync)
        self.assertIn('registered_sync_check(ancestors)',sync)
        self.assertNotIn('Register(',sync)
        self.assertNotIn('pub fn at(',native)
        self.assertNotIn('env::var',native)
        self.assertIn('No public', (CRATE/'src/lib.rs').read_text())

    def test_no_temporary_preparation_or_persistent_test_payload(self):
        self.assertFalse((ROOT/'.github/workflows/ca03c-prepare.yml').exists())
        for p in CRATE.rglob('*'):
            self.assertNotIn(p.suffix, ['.bin','.cakp','.stage'])

    def test_prior_dependency_reviews_cannot_be_rewritten(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t)
            review=json.loads((ROOT/'docs/ca-04a-dependencies.json').read_text())
            files=[*review['manifest_sha256'],*review['predecessor_sha256'],'docs/ca-04a-dependencies.json','Cargo.lock']
            for p in files:
                (root/p).parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/p,root/p)
            self.assertEqual(check_dependencies.check(root)['reviewed_packages'],58)
            p=root/'docs/ca-03b-dependencies.json';p.write_bytes(p.read_bytes()+b'\n')
            with self.assertRaisesRegex(ValueError,'Prior review bytes'):
                check_dependencies.check(root)

    def test_switch_coordinator_is_private_without_a_production_effect_adapter(self):
        coordinator = (CRATE/'src/coordinator.rs').read_text()
        lib = (CRATE/'src/lib.rs').read_text()
        for name in ['coordinator.rs', 'journal.rs', 'journal_codec.rs']:
            source = (CRATE/'src'/name).read_text()
            self.assertIn('#![forbid(unsafe_code)]', source)
            self.assertNotIn('unsafe {', source)
        self.assertIn('pub(crate) trait Effects', coordinator)
        self.assertIn('pub(crate) fn begin_switch(', coordinator)
        self.assertNotIn('pub fn begin_switch(', coordinator)
        self.assertNotIn('pub mod engine', lib)
        self.assertNotIn('pub use coordinator', lib)
        for p in (CRATE/'src').rglob('*.rs'):
            if p.name.endswith('tests.rs'):
                continue
            self.assertNotIn('impl Effects for', p.read_text(), p.name)

    def test_unresolved_switches_block_profile_writes_and_pruning(self):
        engine = (CRATE/'src/engine.rs').read_text()
        self.assertIn('coordinator::verify_journals(disk, root, r)', engine)
        self.assertIn('self.registry.journals.iter().any(|j| !j.terminal())', engine)
        for method in ['add', 'append', 'remove', 'prune']:
            start = engine.index('pub fn ' + method + '(')
            end = engine.find('\n    pub fn ', start + 1)
            body = engine[start:] if end == -1 else engine[start:end]
            self.assertIn('self.idle()?;', body, method)
        self.assertIn('self.switch_session != Some(id)', (CRATE/'src/coordinator.rs').read_text())

    def test_journal_codec_and_tests_are_part_of_ordinary_build_checks(self):
        source = (CRATE/'src/codec.rs').read_text()
        self.assertIn('journal_codec', source)
        self.assertIn('CAREG001', source)
        self.assertIn('CAREG002', source)
        coordinator = (CRATE/'src/coordinator.rs').read_text()
        self.assertIn('#[path = "switch_tests.rs"]', coordinator)
        tests = (CRATE/'src/switch_tests.rs').read_text()
        self.assertNotIn('#[ignore', tests)
        for case in ['every_durable_forward_and_restoration_boundary',
                     'durable_request_and_cancellation_boundaries',
                     'durable_recovery_choice_and_refresh_capture_boundaries']:
            self.assertIn(case, tests)
        self.assertFalse((ROOT/'.github/workflows/ca04a-prepare.yml').exists())
