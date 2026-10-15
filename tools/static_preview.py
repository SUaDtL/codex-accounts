"""Check or regenerate four fixed preview assets and a development icon; no launch."""
from __future__ import annotations
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import stat
import struct
import sys

ROOT = Path(__file__).resolve().parents[1]
NAMES = ('accounts.html', 'status.html', 'recovery.html', 'preview.css')
SHELL = Path('crates/shell-preparation')
CAPABILITY = {'identifier': 'static-preview', 'windows': ['main'],
              'platforms': ['windows'], 'permissions': []}
CSP = ("default-src 'none'; script-src 'none'; style-src 'self'; img-src 'none'; "
       "font-src 'none'; connect-src 'none'; object-src 'none'; frame-src 'none'; "
       "base-uri 'none'; form-action 'none'")


class PreviewError(ValueError):
    """Fixed developer-tool categories, never file content or private paths."""


def require(ok, category):
    if not ok:
        raise PreviewError(category)


def ordinary(path, directory=False):
    for ancestor in [path, *path.parents]:
        info = ancestor.lstat()
        require(not stat.S_ISLNK(info.st_mode)
                and not getattr(info, 'st_file_attributes', 0) & 0x400,
                'preview_link_refused')
    info = path.lstat()
    require(stat.S_ISDIR(info.st_mode) if directory else
            stat.S_ISREG(info.st_mode) and info.st_nlink == 1, 'preview_shape_refused')
    return info


def read(path):
    before = ordinary(path)
    require(before.st_size <= 1024 * 1024, 'preview_size_refused')
    with path.open('rb') as stream:
        data = stream.read(1024 * 1024 + 1)
    after = ordinary(path)
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
            and len(data) == before.st_size, 'preview_changed')
    return data


class StaticHtml(HTMLParser):
    def handle_starttag(self, tag, pairs):
        attrs = dict(pairs)
        require(len(attrs) == len(pairs), 'preview_duplicate_attribute')
        require(tag not in ('script', 'style', 'iframe', 'frame', 'frameset',
                            'object', 'embed', 'form', 'base', 'svg', 'math'),
                'preview_active_html')
        url_attributes = {'src', 'srcset', 'imagesrcset', 'srcdoc', 'ping', 'action',
                          'formaction', 'poster', 'background', 'data', 'codebase',
                          'archive', 'xlink:href', 'manifest'}
        require(not any(k.startswith('on') for k in attrs)
                and not url_attributes.intersection(attrs) and 'style' not in attrs,
                'preview_active_attribute')
        if tag == 'button':
            require('disabled' in attrs, 'preview_enabled_control')
        if 'href' in attrs:
            require((tag == 'a' and attrs['href'] in ('#main', *NAMES[:3]))
                    or (tag == 'link' and attrs['href'] == 'preview.css'
                        and attrs.get('rel') == 'stylesheet'), 'preview_external_link')
        if 'http-equiv' in attrs:
            require(tag == 'meta' and isinstance(attrs['http-equiv'], str)
                    and attrs['http-equiv'].lower()
                    == 'content-security-policy', 'preview_meta_navigation')

    handle_startendtag = handle_starttag


def validate(name, data):
    text = data.decode('utf8')
    if name.endswith('.html'):
        parser = StaticHtml(convert_charrefs=True)
        parser.feed(text)
        parser.close()
    else:
        require('\\' not in text and not any(token in text.lower() for token in
                ('url(', '@import', 'image-set(', 'image(')),
                'preview_external_style')


def icon():
    """Original 16x16 BGRA development mark; no downloaded branding."""
    pixels = bytearray()
    for y in reversed(range(16)):
        for x in range(16):
            border = x in (0, 15) or y in (0, 15)
            letter = ((x in (5, 6) and 4 <= y <= 12)
                      or (5 <= x <= 10 and y in (4, 5, 8, 9))
                      or (x in (9, 10) and 5 <= y <= 8))
            r, g, b = (69, 203, 218) if border else (
                (240, 244, 250) if letter else (22, 30, 43))
            pixels.extend((b, g, r, 255))
    bitmap = struct.pack('<IiiHHIIiiII', 40, 16, 32, 1, 32, 0, len(pixels), 0, 0, 0, 0)
    bitmap += pixels + bytes(64)
    return struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', 16, 16, 0, 0, 1, 32,
                                                len(bitmap), 22) + bitmap


def sources(root):
    result = {}
    for name in NAMES:
        result[name] = read(root / 'app' / name)
        validate(name, result[name])
    return result


def existing_leaf(path):
    try:
        path.lstat()
    except FileNotFoundError:
        return
    ordinary(path)


def stage(root=ROOT):
    """Regenerate only fixed public development assets, refusing extra entries."""
    values = sources(root)
    folder = root / SHELL / 'staged-assets'
    ordinary(folder.parent, True)
    folder.mkdir(exist_ok=True)
    ordinary(folder, True)
    require(set(p.name for p in folder.iterdir()) <= set(NAMES), 'preview_extra_asset')
    for name, data in values.items():
        leaf = folder / name
        existing_leaf(leaf)
        leaf.write_bytes(data)
        require(read(leaf) == data, 'preview_copy_changed')
    directory = root / SHELL / 'icons'
    directory.mkdir(exist_ok=True)
    ordinary(directory, True)
    leaf = directory / 'dev-preview.ico'
    existing_leaf(leaf)
    leaf.write_bytes(icon())
    return check(root)


def check(root=ROOT):
    values = sources(root)
    folder = root / SHELL / 'staged-assets'
    ordinary(folder, True)
    require(set(p.name for p in folder.iterdir()) == set(NAMES), 'preview_extra_asset')
    for name, data in values.items():
        require(read(folder / name) == data, 'preview_asset_drift')
    require(read(root / SHELL / 'icons/dev-preview.ico') == icon(), 'preview_icon_drift')
    def pairs(rows):
        out = {}
        for k, v in rows:
            require(k not in out, 'preview_duplicate_config_key')
            out[k] = v
        return out
    config = json.loads(read(root / SHELL / 'tauri.conf.json'), object_pairs_hook=pairs)
    require(config == {
        'productName': 'Codex Accounts Static Preview - Unqualified', 'version': '0.1.0',
        'identifier': 'invalid.codexaccounts.staticpreview',
        'build': {'frontendDist': 'staged-assets'},
        'app': {'windows': [], 'withGlobalTauri': False,
                'security': {'capabilities': [CAPABILITY], 'csp': CSP}},
        'bundle': {'active': False, 'icon': ['icons/dev-preview.ico']},
    }, 'preview_config_authority')
    for name in ('capabilities', 'permissions'):
        require(not (root / SHELL / name).exists(), 'preview_capability_directory')
    main = read(root / SHELL / 'src/main.rs').decode('utf8')
    build = read(root / SHELL / 'build.rs').decode('utf8')
    require(main.startswith('#![forbid(unsafe_code)]')
            and build.startswith('#![forbid(unsafe_code)]'), 'preview_unsafe_boundary')
    for required in ('.disable_javascript()', '.on_navigation(allowed)',
                     '.on_new_window(|_, _| NewWindowResponse::Deny)',
                     '.data_directory(profile.clone())', 'tauri::webview_version()'):
        require(required in main, 'preview_runtime_guard')
    require(not any(s in main for s in ('invoke_handler', '.plugin(', '#[tauri::command]')),
            'preview_application_command')
    return {'result': 'passed', 'frontend_assets': len(values), 'app_commands': 0,
            'credential_mutation_enabled': False, 'qualified': False}


def main(root=ROOT, argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', action='store_true')
    args = parser.parse_args(argv)
    try:
        print(json.dumps(stage(root) if args.stage else check(root), sort_keys=True))
        return 0
    except Exception:
        print('static_preview_refused', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
