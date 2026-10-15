"""Exact collected schema preservation, not installation qualification."""
from pathlib import Path
from contextlib import redirect_stdout, redirect_stderr
import io
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

from tools import check_runtime_schema as schema

ROOT = Path(__file__).resolve().parents[2]


class RuntimeSchemaTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / schema.SCHEMA_DIR, self.root / schema.SCHEMA_DIR)
        source = self.root / 'crates/runtime/src'
        source.mkdir(parents=True)
        shutil.copyfile(ROOT / 'crates/runtime/src/codec.rs', source / 'codec.rs')

    def test_collected_subset_and_codec_bind_to_the_exact_original_image(self):
        result = schema.check(self.root)
        self.assertEqual(result['included_schema_files'], 14)
        self.assertEqual(result['generated_schema_files'], 317)
        self.assertIs(result['qualified'], False)

    def test_changed_generated_bytes_are_not_adopted(self):
        path = self.root / schema.SCHEMA_DIR / 'v1/InitializeResponse.json'
        path.write_bytes(path.read_bytes().replace(b'codexHome', b'otherHome'))
        with self.assertRaisesRegex(schema.SchemaError, '^schema_digest_refused$'):
            schema.check(self.root)

    def test_observation_cannot_be_relabelled_qualified(self):
        path = self.root / schema.SCHEMA_DIR / 'manifest.json'
        data = path.read_bytes()
        self.assertIn(b'"qualified": false', data)
        path.write_bytes(data.replace(b'"qualified": false', b'"qualified": true'))
        with self.assertRaisesRegex(schema.SchemaError, '^schema_manifest_digest_refused$'):
            schema.check(self.root)

    def test_extra_or_missing_schema_files_refuse(self):
        directory = self.root / schema.SCHEMA_DIR
        extra = directory / 'unreviewed.json'
        extra.write_text('{}', encoding='utf8')
        with self.assertRaisesRegex(schema.SchemaError, '^schema_subset_refused$'):
            schema.check(self.root)
        extra.unlink()
        (directory / 'v1/InitializeResponse.json').unlink()
        with self.assertRaisesRegex(schema.SchemaError, '^schema_subset_refused$'):
            schema.check(self.root)

    def test_codec_cannot_claim_a_different_runtime_or_schema(self):
        path = self.root / 'crates/runtime/src/codec.rs'
        original = path.read_text(encoding='utf8')
        for value in [schema.RUNTIME_VERSION, schema.RUNTIME_IMAGE_SHA256, schema.MANIFEST_SHA256]:
            with self.subTest(value=value):
                self.assertIn(value, original)
                path.write_text(original.replace(value, 'SYNTHETIC_WRONG_BINDING'), encoding='utf8')
                with self.assertRaisesRegex(schema.SchemaError, '^codec_schema_binding_refused$'):
                    schema.check(self.root)

    def test_owned_junction_or_symlink_refuses_without_path_is_junction(self):
        target = self.root / 'SYNTHETIC_OUTSIDE'
        target.mkdir()
        link = self.root / schema.SCHEMA_DIR / 'SYNTHETIC_LINK'
        if os.name == 'nt':
            command = Path(os.environ['SystemRoot']) / 'System32/cmd.exe'
            result = subprocess.run([str(command), '/d', '/c', 'mklink', '/J', str(link), str(target)],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5, check=False)
            self.assertEqual(result.returncode, 0, 'Owned junction setup failed')
        else:
            link.symlink_to(target, target_is_directory=True)
        with mock.patch.object(Path, 'is_junction', None, create=True):
            with self.assertRaisesRegex(schema.SchemaError, '^schema_link_refused$'):
                schema.check(self.root)

    def test_cli_refusals_do_not_print_tracebacks_or_private_exception_text(self):
        for error, category in [(schema.SchemaError('schema_digest_refused'), 'schema_digest_refused'),
                                (OSError('SYNTHETIC_PRIVATE_PATH'), 'schema_check_failed')]:
            with self.subTest(category=category):
                output, errors = io.StringIO(), io.StringIO()
                with mock.patch.object(schema, 'check', side_effect=error), redirect_stdout(output), redirect_stderr(errors):
                    self.assertEqual(schema.main(), 2)
                self.assertEqual(json.loads(output.getvalue())['category'], category)
                self.assertNotIn('SYNTHETIC_PRIVATE_PATH', output.getvalue())
                self.assertEqual(errors.getvalue(), '')


if __name__ == '__main__':
    unittest.main()
