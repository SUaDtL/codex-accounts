"""Execution-evidence and isolation regressions; fixtures never authorize the product."""
import errno
from contextlib import redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from tools import ci_isolated, ci_source, run_native_tests

ROOT = Path(__file__).resolve().parents[2]


def native_output() -> bytes:
    rows = [f'test {run_native_tests.PREFIX}{name} ... ok' for name in run_native_tests.CASES]
    rows.append(f'test result: ok. {len(rows)} passed; 0 failed; 0 ignored; 0 measured; 72 filtered out; finished in 8.01s')
    return ('\n'.join(rows) + '\n').encode()


class NativeProofTests(unittest.TestCase):
    def test_exact_names_and_successful_execution_are_required(self):
        run_native_tests.verify(native_output(), 0)
        run_native_tests.verify(native_output().replace(b'\n', b'\r\n'), 0)
        for code in (1, -1, 124):
            with self.assertRaises(ValueError):
                run_native_tests.verify(native_output(), code)

    def test_missing_duplicate_ignored_or_wrong_native_case_refuses(self):
        good = native_output()
        for bad in (good.split(b'\n', 1)[1], good + good.split(b'\n')[0] + b'\n',
                    good.replace(b' ... ok', b' ... ignored', 1),
                    good.replace(b' ... ok', b' ... FAILED', 1),
                    good.replace(b'ca04b_home_mutex_unicode_alias_and_recursive_contention', b'not_the_native_case'),
                    b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 83 filtered out; finished in 0.01s\n'):
            with self.assertRaises(ValueError):
                run_native_tests.verify(bad, 0)

    def test_truncated_oversized_or_multiple_summaries_refuse(self):
        for bad in (native_output()[:-20], native_output() + native_output().splitlines()[-1],
                    b'x' * (run_native_tests.MAX_OUTPUT + 1), b'\xff'):
            with self.assertRaises(ValueError):
                run_native_tests.verify(bad, 0)

    def test_case_inventory_matches_native_behavior_tests_not_child_harness(self):
        import re
        source = (ROOT / 'crates/vault-storage/src/native/lifecycle_tests.rs').read_text()
        names = re.findall(r'fn (ca04b_[a-z_]+)\(', source)
        self.assertEqual(set(names), set(run_native_tests.CASES))
        self.assertEqual(len(names), len(run_native_tests.CASES))
        self.assertNotIn('#[ignore', source)

    def test_portable_host_cannot_claim_windows_proof(self):
        with patch.object(run_native_tests.platform, 'system', return_value='Linux'):
            with self.assertRaises(ValueError):
                run_native_tests.run()


class IsolatedBuildTests(unittest.TestCase):
    def status(self):
        return 'Uid:\t1001\t1001\t1001\t1001\nNoNewPrivs:\t1\n' + ''.join(
            f'{name}:\t0000000000000000\n' for name in ci_isolated.CAPABILITIES)

    def test_actual_namespace_uid_and_all_capability_fields_are_required(self):
        good = self.status()
        ci_isolated.validate_status(good, 1001, 'net:[100]', 'net:[200]')
        changes = [good.replace('NoNewPrivs:\t1', 'NoNewPrivs:\t0'),
                   good.replace('Uid:\t1001', 'Uid:\t0')]
        changes += [good.replace(f'{key}:\t0000000000000000', f'{key}:\t0000000000000001')
                    for key in ci_isolated.CAPABILITIES]
        changes += [good.replace(f'{key}:\t0000000000000000\n', '')
                    for key in ci_isolated.CAPABILITIES]
        for changed in changes:
            with self.assertRaises(ValueError):
                ci_isolated.validate_status(changed, 1001, 'net:[100]', 'net:[200]')
        for uid, parent, child in [(0, 'net:[100]', 'net:[200]'), (1001, 'net:[100]', 'net:[100]'),
                                   (1001, '', 'net:[200]')]:
            with self.assertRaises(ValueError):
                ci_isolated.validate_status(good, uid, parent, child)

    def test_successful_route_or_inconclusive_error_cannot_pass(self):
        for error in (None, OSError(errno.EACCES, 'SYNTHETIC_NOT_NETWORK_PROOF')):
            with patch.object(ci_isolated.socket, 'socket') as socket:
                socket.return_value.__enter__.return_value.connect.side_effect = error
                with self.assertRaises(ValueError):
                    ci_isolated.no_ip_route()
        with patch.object(ci_isolated.socket, 'socket') as socket:
            socket.return_value.__enter__.return_value.connect.side_effect = OSError(errno.ENETUNREACH, 'synthetic')
            ci_isolated.no_ip_route()
            self.assertEqual(socket.call_count, 2)

    def test_failed_check_remains_failed_and_other_independent_checks_execute(self):
        with tempfile.TemporaryDirectory() as directory:
            config = {'uid': 1001, 'namespace': 'net:[100]', 'env': {}, 'target': directory, 'cargo': 'cargo'}
            with patch.object(ci_isolated, 'validate_status'), patch.object(ci_isolated, 'no_ip_route'), \
                 patch.object(ci_isolated.subprocess, 'run') as run, redirect_stdout(io.StringIO()):
                run.side_effect = [subprocess.CompletedProcess([], 1)] + [subprocess.CompletedProcess([], 0)] * 3
                self.assertEqual(ci_isolated.inner(config), 1)
                self.assertEqual(run.call_count, 4)
                for call in run.call_args_list:
                    self.assertIn('--locked', call.args[0])
                    self.assertIn('--offline', call.args[0])
                    self.assertEqual(call.kwargs['env']['CARGO_NET_OFFLINE'], 'true')
            (Path(directory) / 'old-build').write_text('SYNTHETIC')
            with patch.object(ci_isolated, 'validate_status'), patch.object(ci_isolated, 'no_ip_route'):
                with self.assertRaises(ValueError):
                    ci_isolated.inner(config)

    def test_no_privileged_python_or_connected_fallback_in_launch(self):
        source = (ROOT / 'tools/ci_isolated.py').read_text()
        for flag in ['--net', '--clear-groups', '--bounding-set=-all', '--inh-caps=-all',
                     '--ambient-caps=-all', '--no-new-privs']:
            self.assertIn(flag, source)
        self.assertNotIn('iptables', source)
        self.assertNotIn('cargo fetch', source)


class SourceEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'repository'
        self.root.mkdir()
        for command in [('init',), ('config', 'user.name', 'Synthetic Test'),
                        ('config', 'user.email', 'synthetic@example.invalid')]:
            ci_source.git(self.root, *command)
        (self.root / 'source.txt').write_text('SYNTHETIC_SOURCE\n')
        ci_source.git(self.root, 'add', 'source.txt')
        ci_source.git(self.root, 'commit', '-m', 'Synthetic fixture')

    def test_archive_is_exact_committed_source_not_untracked_runtime_data(self):
        canary = b'SYNTHETIC_UNTRACKED_SECRET'
        (self.root / 'untracked.txt').write_bytes(canary)
        destination = Path(self.temp.name) / 'evidence'
        record = ci_source.create(self.root, destination)
        data = (destination / 'source.tar.gz').read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), record['archive_sha256'])
        with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
            self.assertEqual(archive.getnames(), ['source.txt'])
            self.assertNotIn(canary, archive.extractfile('source.txt').read())
        self.assertEqual(record['qualification'], 'not_established')
        self.assertEqual(record['source_tree'], ci_source.git(self.root, 'rev-parse', 'HEAD^{tree}').decode().strip())

    def test_dirty_source_existing_output_and_link_objects_are_rejected(self):
        (self.root / 'source.txt').write_text('SYNTHETIC_CHANGED')
        destination = Path(self.temp.name) / 'evidence'
        with self.assertRaises(subprocess.CalledProcessError):
            ci_source.create(self.root, destination)
        self.assertFalse(destination.exists())
        ci_source.git(self.root, 'checkout', '--', 'source.txt')
        destination.mkdir()
        with self.assertRaises(FileExistsError):
            ci_source.create(self.root, destination)
        if hasattr(__import__('os'), 'symlink'):
            (self.root / 'alias').symlink_to('source.txt')
            ci_source.git(self.root, 'add', 'alias')
            ci_source.git(self.root, 'commit', '-m', 'Synthetic link fixture')
            with self.assertRaises(ValueError):
                ci_source.create(self.root, Path(self.temp.name) / 'other')


class NativeBoundaryTests(unittest.TestCase):
    def test_private_native_ownership_and_safe_model_stay_separate(self):
        native = ROOT / 'crates/vault-storage/src/native/lifecycle'
        home = (native / 'home.rs').read_text()
        process = (native / 'process.rs').read_text()
        helper = (native / 'helper.rs').read_text()
        self.assertIn('PhantomData<Rc<()>>', home)
        self.assertIn('FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL', home)
        self.assertIn('FILE_ID_INFO', home)
        self.assertIn('check_mutex(', home)
        self.assertNotIn('TerminateProcess', process)
        self.assertNotIn('PROCESS_TERMINATE', process)
        self.assertIn('#[cfg(test)]\n    pub(super) fn for_suspended_test_child', helper)
        self.assertIn('self.exit_observed()', helper)
        self.assertIn('#![forbid(unsafe_code)]', (ROOT / 'crates/vault-storage/src/lifecycle_model.rs').read_text())
        for text in [home, process, helper]:
            self.assertNotIn('pub fn ', text)
            self.assertNotIn('impl Effects for', text)


if __name__ == '__main__':
    unittest.main()
