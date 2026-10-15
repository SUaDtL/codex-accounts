"""Check the pinned generated schema subset; never launch a runtime or resolve refs."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_DIR = Path('crates/runtime/schema/codex-0.162.0-alpha.17.2')
RUNTIME_VERSION = '0.162.0-alpha.17.2'
RUNTIME_IMAGE_SHA256 = 'e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e'
MANIFEST_SHA256 = 'a87f649f36c3a2787182bedd125002cf1a2eff54c46383b04e7d1431a7dd758e'


class SchemaError(ValueError):
    """Fixed categories only; no source content or local path in diagnostics."""


def require(ok: bool, category: str) -> None:
    if not ok:
        raise SchemaError(category)


def metadata(path: Path):
    try:
        info = path.lstat()
    except OSError:
        raise SchemaError('schema_read_refused') from None
    require(not stat.S_ISLNK(info.st_mode)
            and not getattr(info, 'st_file_attributes', 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT,
            'schema_link_refused')
    return info


def read_file(path: Path, limit: int) -> bytes:
    info = metadata(path)
    require(stat.S_ISREG(info.st_mode), 'schema_file_refused')
    require(info.st_size <= limit, 'schema_size_refused')
    try:
        with path.open('rb') as stream:
            data = stream.read(limit + 1)
    except OSError:
        raise SchemaError('schema_read_refused') from None
    require(len(data) <= limit, 'schema_size_refused')
    return data


def unlinked(root: Path, relative: Path) -> Path:
    path = root
    for component in relative.parts:
        path /= component
        metadata(path)
    return path


def check(root: Path = ROOT) -> dict[str, int | str | bool]:
    directory = unlinked(root, SCHEMA_DIR)
    pending = [directory]
    actual_files = set()
    directories = 0
    try:
        while pending:
            current = pending.pop()
            directories += 1
            require(directories <= 128 and stat.S_ISDIR(metadata(current).st_mode),
                    'schema_directory_refused')
            with os.scandir(current) as entries:
                for entry in entries:
                    info = entry.stat(follow_symlinks=False)
                    require(not stat.S_ISLNK(info.st_mode)
                            and not getattr(info, 'st_file_attributes', 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT,
                            'schema_link_refused')
                    if stat.S_ISDIR(info.st_mode):
                        pending.append(Path(entry.path))
                        require(directories + len(pending) <= 128, 'schema_directory_refused')
                    else:
                        require(stat.S_ISREG(info.st_mode), 'schema_file_refused')
                        actual_files.add(Path(entry.path).relative_to(directory).as_posix())
                        require(len(actual_files) <= 513, 'schema_count_refused')
    except OSError:
        raise SchemaError('schema_read_refused') from None
    raw = read_file(directory / 'manifest.json', 1024 * 1024)
    require(hashlib.sha256(raw).hexdigest() == MANIFEST_SHA256, 'schema_manifest_digest_refused')
    manifest = json.loads(raw)
    require(manifest['schema'] == 'codex-accounts/runtime-schema-observation/v1'
            and manifest['status'] == 'observed_unqualified'
            and manifest['runtime_version'] == RUNTIME_VERSION
            and manifest['runtime_image_sha256'] == RUNTIME_IMAGE_SHA256
            and manifest['qualified'] is False
            and manifest['credential_mutation_enabled'] is False,
            'schema_observation_boundary_refused')
    rows = manifest['files']
    names = [row['relative_name'] for row in rows]
    require(len(rows) == len(set(names)) == manifest['total_generated_files'] == 317
            and sum(row['bytes'] for row in rows) == manifest['total_generated_bytes'] == 3590402,
            'schema_inventory_refused')
    inventory = json.dumps(sorted(rows, key=lambda row: row['relative_name']),
                           sort_keys=True, separators=(',', ':')).encode()
    require(hashlib.sha256(inventory).hexdigest() == manifest['inventory_sha256'],
            'schema_inventory_digest_refused')
    included = set(manifest['included_files'])
    require(actual_files == included | {'manifest.json'} and len(included) == 22,
            'schema_subset_refused')
    indexed = {row['relative_name']: row for row in rows}
    for name in included:
        row = indexed[name]
        data = read_file(directory / name, 16 * 1024 * 1024)
        require(len(data) == row['bytes'] and hashlib.sha256(data).hexdigest() == row['sha256'],
                'schema_digest_refused')
    try:
        codec = read_file(unlinked(root, Path('crates/runtime/src/codec.rs')), 1024 * 1024).decode('utf8')
    except UnicodeError:
        raise SchemaError('codec_source_encoding_refused') from None
    for name, value in [('RUNTIME_VERSION', RUNTIME_VERSION),
                        ('RUNTIME_IMAGE_SHA256', RUNTIME_IMAGE_SHA256),
                        ('SCHEMA_MANIFEST_SHA256', MANIFEST_SHA256)]:
        matches = re.findall(r'\bconst\s+' + name + r'\s*:\s*&str\s*=\s*"([^"]*)"\s*;', codec)
        require(matches == [value], 'codec_schema_binding_refused')
    return {'included_schema_files': len(included), 'generated_schema_files': len(rows),
            'runtime_version': RUNTIME_VERSION, 'qualified': False, 'result': 'passed'}


def main() -> int:
    try:
        result = check()
    except SchemaError as error:
        print(json.dumps({'result': 'refused', 'category': str(error), 'qualified': False}))
        return 2
    except Exception:
        print(json.dumps({'result': 'refused', 'category': 'schema_check_failed', 'qualified': False}))
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
