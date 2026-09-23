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
            review=json.loads((ROOT/'docs/ca-03c-dependencies.json').read_text())
            files=[*review['manifest_sha256'],*review['predecessor_sha256'],'docs/ca-03c-dependencies.json','Cargo.lock']
            for p in files:
                (root/p).parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/p,root/p)
            self.assertEqual(check_dependencies.check(root)['reviewed_packages'],58)
            p=root/'docs/ca-03b-dependencies.json';p.write_bytes(p.read_bytes()+b'\n')
            with self.assertRaisesRegex(ValueError,'Prior review bytes'):
                check_dependencies.check(root)
