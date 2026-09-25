"""Preparation records cannot launder Windows results into macOS evidence."""
from copy import deepcopy
import hashlib
import json
import unittest
from tools import macos_preparation as m


class MacosPreparationTests(unittest.TestCase):
    def setUp(self):
        self.record = m.decode((m.ROOT / m.PACKET).read_bytes())
        self.sources = {name: (m.ROOT / name).read_bytes() for name in m.SOURCES}

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
