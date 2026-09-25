"""Retain the exact committed source tree, not caches, test vaults or credentials.

An archive and digest record are reproducible source inputs, not passing CI,
qualification, a release, a cryptographic attestation or a dependency approval.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
MAX_SOURCE = 16 * 1024 * 1024


def git(root: Path, *arguments: str) -> bytes:
    result = subprocess.run(['git', *arguments], cwd=root, check=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            timeout=30)
    if len(result.stdout) > MAX_SOURCE:
        raise ValueError('Source evidence exceeds its bound')
    return result.stdout


def create(root: Path, output: Path) -> dict:
    revision = git(root, 'rev-parse', 'HEAD').decode('ascii').strip()
    tree = git(root, 'rev-parse', 'HEAD^{tree}').decode('ascii').strip()
    if not all(re.fullmatch(r'[0-9a-f]{40}', v) for v in [revision, tree]):
        raise ValueError('Invalid source identity')
    entries = git(root, 'ls-tree', '-rlz', 'HEAD').split(b'\0')
    size = 0
    for entry in entries:
        if not entry:
            continue
        metadata, _ = entry.split(b'\t', 1)
        mode, kind, _, length = metadata.split()
        if kind != b'blob' or mode not in {b'100644', b'100755'}:
            raise ValueError('Unsupported source object')
        size += int(length)
    if size > MAX_SOURCE:
        raise ValueError('Source tree exceeds its bound')
    git(root, 'diff', '--exit-code', 'HEAD', '--')
    archive = git(root, 'archive', '--format=tar.gz', 'HEAD')
    record = {
        'schema': 'codex-accounts/source-evidence/v1',
        'source_commit': revision,
        'source_tree': tree,
        'archive_sha256': hashlib.sha256(archive).hexdigest(),
        'tracked_source_bytes': size,
        'qualification': 'not_established',
    }
    output.mkdir(parents=False, exist_ok=False)
    (output / 'source.tar.gz').write_bytes(archive)
    (output / 'source.json').write_text(json.dumps(record, sort_keys=True, indent=2) + '\n',
                                      encoding='utf-8')
    return record


if __name__ == '__main__':
    try:
        record = create(ROOT, Path(os.environ['RUNNER_TEMP']) / 'ca-source-evidence')
        print(json.dumps(record, sort_keys=True))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        raise SystemExit('Source evidence collection failed') from None
