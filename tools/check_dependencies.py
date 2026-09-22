"""Verify that the lockfile still matches the scoped dependency review."""
from __future__ import annotations

import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def check(root: Path = ROOT) -> dict[str, int | str]:
    lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
    review = json.loads((root / 'docs/ca-02-dependencies.json').read_text(encoding='utf-8'))
    if review['schema'] != 'codex-accounts/dependency-review/v1':
        raise ValueError('Unknown dependency review schema')
    actual = {(p['name'], p['version']): (p['source'], p['checksum'])
              for p in lock['package'] if 'source' in p}
    expected = {(p['name'], p['version']): (p['source'], p['checksum'])
                for p in review['packages']}
    if len(expected) != len(review['packages']) or actual != expected:
        raise ValueError('Dependency graph changed; update the scoped review')
    for name in ['core', 'platform', 'runtime']:
        value = tomllib.loads((root / f'crates/{name}/Cargo.toml').read_text(encoding='utf-8'))
        if 'dependencies' in value:
            raise ValueError('Safe bootstrap crate gained dependencies')
    return {'reviewed_packages': len(expected), 'lock_review': 'passed'}


if __name__ == '__main__':
    print(json.dumps(check(), sort_keys=True))
