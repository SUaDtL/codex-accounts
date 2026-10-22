"""Preparation records cannot launder Windows results into macOS evidence."""
from copy import deepcopy
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from tools import macos_preparation as m


class MacosPreparationTests(unittest.TestCase):
    def setUp(self):
        self.record = m.decode((m.ROOT / m.PACKET).read_bytes())
        self.sources = {name: (m.ROOT / name).read_bytes() for name in m.SOURCES}
        for name in ('docs/macos/interface-baseline.json',
                     'docs/macos/interface-baseline-ca04c.json',
                     'docs/macos/ca-04d-interface-review.md',
                     'docs/macos/windows-storage-interface-review.json'):
            if (m.ROOT / name).is_file():
                self.sources[name] = (m.ROOT / name).read_bytes()

    @contextmanager
    def join_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in {*self.sources, m.PACKET}:
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(m.ROOT / path, root / path)
            record = deepcopy(self.record)
            record['interfaces'] = {
                name: hashlib.sha256(self.sources[name].replace(b'\r\n', b'\n')).hexdigest()
                for name in m.SOURCES}
            (root / m.PACKET).write_text(json.dumps(record), encoding='utf-8')
            yield root, record

    def rehash_review(self, root, record, raw):
        path = ('docs/macos/windows-storage-interface-review.json'
                if record['packet_format'].endswith('/v3')
                else 'docs/macos/ca-04d-interface-review.md')
        (root / path).write_bytes(raw)
        record['review_sha256'] = hashlib.sha256(raw.replace(b'\r\n', b'\n')).hexdigest()
        (root / m.PACKET).write_text(json.dumps(record), encoding='utf-8')

    def test_current_join_retains_exact_historical_chain(self):
        historical = ['docs/macos/interface-baseline-ca04c.json',
                      'docs/macos/ca-04d-interface-review.md']
        if self.record['packet_format'].endswith('/v3'):
            historical.append('docs/macos/interface-baseline.json')
        for path in historical:
            with self.subTest(path=path), self.join_fixture() as (root, record):
                m.check(root)
                raw = (root / path).read_bytes() + b'\n '
                (root / path).write_bytes(raw)
                if record['packet_format'].endswith('/v2'):
                    field = ('predecessor_sha256' if path.endswith('ca04c.json')
                             else 'review_sha256')
                    record[field] = hashlib.sha256(raw.replace(b'\r\n', b'\n')).hexdigest()
                else:
                    previous_path = 'docs/macos/interface-baseline.json'
                    previous = m.decode((root / previous_path).read_bytes())
                    if path != previous_path:
                        field = ('predecessor_sha256' if path.endswith('ca04c.json')
                                 else 'review_sha256')
                        previous[field] = hashlib.sha256(raw.replace(b'\r\n', b'\n')).hexdigest()
                        (root / previous_path).write_text(json.dumps(previous), encoding='utf-8')
                    digest = hashlib.sha256((root / previous_path).read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                    record['predecessor_sha256'] = digest
                    review = m.decode((root / 'docs/macos/windows-storage-interface-review.json').read_bytes())
                    review['predecessor_sha256'] = digest
                    self.rehash_review(root, record, json.dumps(review).encode())
                (root / m.PACKET).write_text(json.dumps(record), encoding='utf-8')
                with self.assertRaises(m.PacketError, msg='rehashing cannot rewrite fixed historical assessments'):
                    m.check(root)
            with self.subTest(missing=path), self.join_fixture() as (root, _):
                (root / path).unlink()
                with self.assertRaises((m.PacketError, OSError)):
                    m.check(root)
        self.assertEqual(self.record['packet_format'], 'codex-accounts/macos-preparation/v3')
        self.assertEqual(self.record['source_state'], 'reviewed-working-tree')

    def test_rehashed_review_cannot_change_source_join_or_grant_native_authority(self):
        with self.join_fixture() as (root, record):
            m.check(root)
            if record['packet_format'].endswith('/v2'):
                raw = (root / 'docs/macos/ca-04d-interface-review.md').read_bytes() + b'\nnative_status=passed\n'
            else:
                review = m.decode((root / 'docs/macos/windows-storage-interface-review.json').read_bytes())
                review['native_status'] = 'passed'
                raw = json.dumps(review).encode()
            self.rehash_review(root, record, raw)
            with self.assertRaises(m.PacketError, msg='a rehashed review cannot grant native authority'):
                m.check(root)
        with self.join_fixture() as (root, record):
            original = m.decode((root / 'docs/macos/windows-storage-interface-review.json').read_bytes())
            changes = []
            for key, value in [('base_commit', '0' * 40), ('source_state', 'committed'),
                               ('platform', 'windows-x86_64'), ('native_status', True),
                               ('evidence', 'windows-native-pass'), ('qualified', True)]:
                changes.append(dict(original, **{key: value}))
            for field in ('before_sha256', 'after_sha256', 'path'):
                review = deepcopy(original)
                review['changed_sources'][0][field] = '../outside' if field == 'path' else '0' * 64
                changes.append(review)
            for field in ('changed_sources', 'assessment', 'unchanged_interfaces'):
                review = deepcopy(original)
                del review[field]
                changes.append(review)
                changes.append(dict(original, **{field: False}))
            review = deepcopy(original)
            review['unchanged_interfaces'][m.SOURCES[0]] = '0' * 64
            changes.append(review)
            review = deepcopy(original)
            review['unchanged_interfaces']['../outside'] = '0' * 64
            changes.append(review)
            for value in ('', 'x' * 2049, False):
                review = deepcopy(original)
                review['assessment']['recognition'] = value
                changes.append(review)
            for review in changes:
                with self.subTest(review=changes.index(review)):
                    self.rehash_review(root, record, json.dumps(review).encode())
                    with self.assertRaises(m.PacketError):
                        m.check(root)
            for raw in (b'{"native_status":"not-run","native_status":"passed"}',
                        b'\xff', b'{broken}', b'x' * (16 * 1024 + 1)):
                self.rehash_review(root, record, raw)
                with self.assertRaises(m.PacketError):
                    m.check(root)
            self.rehash_review(root, record, json.dumps(original).encode())
            path = m.SOURCES[3]
            changed = self.sources[path] + b'\n// synthetic source drift\n'
            (root / path).write_bytes(changed)
            record['interfaces'][path] = hashlib.sha256(changed.replace(b'\r\n', b'\n')).hexdigest()
            (root / m.PACKET).write_text(json.dumps(record), encoding='utf-8')
            with self.assertRaises(m.PacketError, msg='a new source hash cannot replace semantic review'):
                m.check(root)

    def test_current_shared_interface_snapshot_matches(self):
        m.check()

    def test_changed_interface_requires_new_review(self):
        for name in m.SOURCES:
            changed = dict(self.sources, **{name: self.sources[name] + b'\n// changed\n'})
            with self.assertRaisesRegex(m.PacketError, 'Shared interface changed'):
                m.validate(self.record, changed.__getitem__)

    def test_windows_or_claimed_native_result_is_not_accepted(self):
        for key, bad in [('platform', 'windows-x86_64'), ('native_status', 'passed'),
                         ('evidence', 'windows-native-pass'), ('base_commit', '0' * 40),
                         ('native_status', True), ('packet_format', 'official-schema')]:
            record = dict(self.record, **{key: bad})
            with self.assertRaises(m.PacketError):
                m.validate(record, self.sources.__getitem__)

    def test_arbitrary_paths_rejected_before_reading(self):
        for path in ('../outside', '/absolute', 'credentials', 'C:\\outside', 'extra.rs'):
            record = deepcopy(self.record)
            record['interfaces'][path] = '0' * 64
            calls = []
            with self.assertRaises(m.PacketError):
                m.validate(record, lambda p: calls.append(p))
            self.assertEqual(calls, [])

    def test_missing_extra_or_authority_fields_rejected(self):
        for key in self.record:
            record = deepcopy(self.record)
            del record[key]
            with self.assertRaises(m.PacketError):
                m.validate(record, self.sources.__getitem__)
        for key in ('qualified', 'consent', 'allow_mutation'):
            with self.assertRaises(m.PacketError):
                m.validate(dict(self.record, **{key: True}), self.sources.__getitem__)
        record = deepcopy(self.record)
        del record['interfaces'][m.SOURCES[0]]
        with self.assertRaises(m.PacketError):
            m.validate(record, self.sources.__getitem__)

    def test_bad_digest_and_source_bounds_fail_closed(self):
        for value in (None, 0, 'F' * 64, '0' * 63, 'x' * 64):
            record = deepcopy(self.record)
            record['interfaces'][m.SOURCES[0]] = value
            with self.assertRaises(m.PacketError):
                m.validate(record, self.sources.__getitem__)
        with self.assertRaises(m.PacketError):
            m.validate(self.record, lambda _: b'x' * (m.MAX_BYTES + 1))

    def test_duplicate_keys_malformed_and_large_records_rejected(self):
        for raw in (b'{"platform":"macos-aarch64","platform":"windows"}',
                    b'\xff', b'{broken}', b'x' * (16 * 1024 + 1)):
            with self.assertRaises(m.PacketError):
                m.decode(raw)
        for value in (None, [], 'native-pass', 42):
            with self.assertRaises(m.PacketError):
                m.validate(value, self.sources.__getitem__)

    def test_checkout_line_endings_do_not_change_contract_identity(self):
        crlf = {p: raw.replace(b'\r\n', b'\n').replace(b'\n', b'\r\n')
                for p, raw in self.sources.items()}
        m.validate(self.record, crlf.__getitem__)

    def test_record_is_not_an_official_schema_or_runtime_input(self):
        doc = (m.ROOT / 'docs/macos/native-adapter-packet.md').read_text()
        for text in ('not an official runtime schema', 'Windows evidence', 'NOT RUN',
                     'CA-04D', 'current SDK', 'No native implementation'):
            self.assertIn(text, doc)
        self.assertNotIn('macos-preparation', (m.ROOT / 'compatibility/catalog.json').read_text())


if __name__ == '__main__':
    unittest.main()
