"""Developer-source containment tests, not observed WebView or account behavior."""
import copy
from contextlib import redirect_stderr, redirect_stdout
import io
import json
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock
from tools import static_preview as preview


class StaticPreviewTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        paths = ['app/' + n for n in preview.NAMES]
        paths += [(preview.SHELL / 'staged-assets' / n).as_posix() for n in preview.NAMES]
        paths += [(preview.SHELL / n).as_posix() for n in (
            'src/main.rs', 'build.rs', 'tauri.conf.json', 'icons/dev-preview.ico')]
        for relative in paths:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(preview.ROOT / relative, target)
        self.stage = self.root / preview.SHELL / 'staged-assets'
        self.config = self.root / preview.SHELL / 'tauri.conf.json'

    def test_actual_sources_and_deterministic_regeneration(self):
        self.assertFalse(preview.check()['qualified'])
        before = {n: (self.stage / n).read_bytes() for n in preview.NAMES}
        self.assertEqual(preview.stage(self.root)['frontend_assets'], 4)
        self.assertEqual(before, {n: (self.stage / n).read_bytes() for n in preview.NAMES})
        self.assertEqual(len(preview.icon()), 1150)

    def test_copied_byte_drift_and_incidental_assets_refuse(self):
        target = self.stage / 'accounts.html'
        original = target.read_bytes()
        target.write_bytes(original + b'\nSYNTHETIC_DRIFT')
        with self.assertRaisesRegex(preview.PreviewError, 'asset_drift'):
            preview.check(self.root)
        target.write_bytes(original)
        (self.stage / 'incidental.txt').write_text('SYNTHETIC', encoding='utf8')
        for action in (preview.check, preview.stage):
            with self.assertRaisesRegex(preview.PreviewError, 'extra_asset'):
                action(self.root)

    def test_active_content_refuses_even_when_both_copies_match(self):
        source = self.root / 'app/accounts.html'
        target = self.stage / 'accounts.html'
        for bad in (b'<script>void 0</script>', b'<button>enabled</button>',
                    b'<a href="https://example.invalid">remote</a>',
                    b'<img src="data:text/plain,SYNTHETIC">',
                    b'<p onclick="void 0">event</p>',
                    b'<meta http-equiv="refresh" content="0;url=https://example.invalid">',
                    b'<a href="#main" href="status.html">duplicate</a>',
                    b'<a href="accounts.html" ping="https://synthetic.invalid">ping</a>',
                    b'<img srcset="https://synthetic.invalid/x 1x">'):
            with self.subTest(bad=bad):
                source.write_bytes(bad)
                target.write_bytes(bad)
                with self.assertRaises(preview.PreviewError):
                    preview.check(self.root)

    def test_css_escape_urls_and_malformed_input_keep_fixed_cli_failure(self):
        for style in (b'@\\69mport "https://synthetic.invalid/x";',
                      b'a {background:u\\72l(https://synthetic.invalid/x)}',
                      b'a {background:image-set("x.png" 1x)}'):
            with self.subTest(style=style):
                with self.assertRaisesRegex(preview.PreviewError, 'external_style'):
                    preview.validate('preview.css', style)
        source = self.root / 'app/accounts.html'
        target = self.stage / 'accounts.html'
        original = source.read_bytes()
        source.write_bytes(b'<meta http-equiv>')
        target.write_bytes(b'<meta http-equiv>')
        for kind in ('malformed_meta', 'deep_config'):
            with self.subTest(kind=kind):
                if kind == 'deep_config':
                    source.write_bytes(original)
                    target.write_bytes(original)
                    self.config.write_text('[' * 2000 + '0' + ']' * 2000, encoding='utf8')
                output, errors = io.StringIO(), io.StringIO()
                with redirect_stdout(output), redirect_stderr(errors):
                    self.assertEqual(preview.main(self.root, []), 1)
                self.assertEqual(output.getvalue(), '')
                self.assertEqual(errors.getvalue(), 'static_preview_refused\n')

    def test_permission_navigation_and_distribution_configuration_cannot_expand(self):
        original = json.loads(self.config.read_text(encoding='utf8'))
        for change in ('permission', 'global', 'remote', 'plugin', 'bundle', 'csp'):
            with self.subTest(change=change):
                value = copy.deepcopy(original)
                if change == 'permission':
                    value['app']['security']['capabilities'][0]['permissions'] = ['core:default']
                elif change == 'global':
                    value['app']['withGlobalTauri'] = True
                elif change == 'remote':
                    value['app']['security']['capabilities'][0]['remote'] = {'urls': ['https://*']}
                elif change == 'plugin':
                    value['plugins'] = {'opener': {}}
                elif change == 'bundle':
                    value['bundle']['active'] = True
                else:
                    value['app']['security']['csp'] = "default-src *"
                self.config.write_text(json.dumps(value), encoding='utf8')
                with self.assertRaisesRegex(preview.PreviewError, 'config_authority'):
                    preview.check(self.root)

    def test_duplicate_config_keys_and_discovered_capability_files_refuse(self):
        original = self.config.read_bytes()
        version = json.loads(original)['version'].encode()
        self.config.write_bytes(original.replace(b'"version": "' + version + b'"',
                                                b'"version":"9", "version":"' + version + b'"'))
        with self.assertRaisesRegex(preview.PreviewError, 'duplicate_config_key'):
            preview.check(self.root)
        self.config.write_bytes(original)
        folder = self.root / preview.SHELL / 'capabilities'
        folder.mkdir()
        with self.assertRaisesRegex(preview.PreviewError, 'capability_directory'):
            preview.check(self.root)

    def test_owned_hardlink_is_refused_before_embedding(self):
        target = self.stage / 'accounts.html'
        target.unlink()
        os.link(self.root / 'app/accounts.html', target)
        with self.assertRaisesRegex(preview.PreviewError, 'shape_refused'):
            preview.check(self.root)

    def test_oversized_leaf_is_refused_before_open(self):
        leaf = self.root / 'oversized-public-fixture'
        leaf.write_bytes(b'x' * (1024 * 1024 + 1))
        with mock.patch.object(Path, 'open', side_effect=AssertionError('must not read')):
            with self.assertRaisesRegex(preview.PreviewError, 'size_refused'):
                preview.read(leaf)


if __name__ == '__main__':
    unittest.main()
