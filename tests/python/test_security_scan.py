"""An empty or incomplete source scan must not count as a clean security run."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
import security_scan as scan


class ScanHealthTests(unittest.TestCase):
    def test_health_requires_every_source_and_error_free_results(self):
        expected = {'crates/main.rs', 'crates/app.js', 'tools/build.py'}
        healthy = {'results': [], 'errors': [], 'paths': {'scanned': sorted(expected)}}
        self.assertEqual(scan.opengrep_health(healthy, expected)['scanned_files'], 3)
        self.assertEqual(scan.opengrep_health({**healthy, 'results': [{'check_id': 'unsafe'}]}, expected)['findings'], 1)
        for broken in [None, {}, {**healthy, 'errors': [{'type': 'Timeout'}]},
                       {**healthy, 'errors': None}, {**healthy, 'results': None},
                       {**healthy, 'paths': {'scanned': ['crates/main.rs']}},
                       {**healthy, 'paths': {'scanned': [None]}}]:
            with self.subTest(report=broken), self.assertRaises(ValueError):
                scan.opengrep_health(broken, expected)
        with self.assertRaises(ValueError):
            scan.opengrep_health(healthy, set())

    def test_failed_install_cannot_reuse_previous_clean_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            reports = Path(directory)
            for name in ('opengrep.sarif', 'opengrep.json', 'opengrep-meta.json'):
                (reports / name).write_text('{"status":"complete"}')
            with patch.object(scan, 'REPORTS', reports), \
                 patch.object(scan, 'install', side_effect=ValueError('checksum mismatch')):
                with self.assertRaises(ValueError):
                    scan.run('opengrep')
            self.assertFalse((reports / 'opengrep.sarif').exists())
            self.assertFalse((reports / 'opengrep.json').exists())
            self.assertEqual(json.loads((reports / 'opengrep-meta.json').read_text())['status'], 'incomplete')

    def test_empty_source_scan_is_not_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'crates').mkdir()
            (root / 'crates/main.rs').write_text('fn main() {}')
            (root / 'tools').mkdir()
            (root / 'security/opengrep').mkdir(parents=True)
            binary = root / 'opengrep'
            binary.write_bytes(b'synthetic scanner')
            reports = root / 'reports'

            def execute(command, **kwargs):
                if '--sarif-output' in command:
                    Path(command[command.index('--sarif-output') + 1]).write_text(json.dumps({
                        'version': '2.1.0', 'runs': [{'tool': {}, 'results': []}]}))
                    if '--json-output' in command:
                        Path(command[command.index('--json-output') + 1]).write_text(json.dumps({
                            'version': '1.31.0', 'results': [], 'errors': [], 'paths': {'scanned': []}}))
                return subprocess.CompletedProcess(command, 0)

            with patch.object(scan, 'ROOT', root), patch.object(scan, 'REPORTS', reports), \
                 patch.object(scan, 'install', return_value=binary), \
                 patch.object(scan.subprocess, 'run', side_effect=execute):
                with self.assertRaises(ValueError):
                    scan.run('opengrep')


if __name__ == '__main__':
    unittest.main()
