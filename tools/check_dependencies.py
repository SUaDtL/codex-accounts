"""Verify inherited scoped dependency reviews before build acquisition."""
from __future__ import annotations

import hashlib
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def check(root: Path = ROOT) -> dict[str, int | str]:
    lock_bytes = (root / 'Cargo.lock').read_bytes()
    lock = tomllib.loads(lock_bytes.decode('utf-8'))
    inherited = json.loads((root / 'docs/ca-02-dependencies.json').read_text(encoding='utf-8'))
    crypto = json.loads((root / 'docs/ca-03b-dependencies.json').read_text(encoding='utf-8'))
    storage = json.loads((root / 'docs/ca-03c-dependencies.json').read_text(encoding='utf-8'))
    review = json.loads((root / 'docs/ca-04a-dependencies.json').read_text(encoding='utf-8'))
    required = {'docs/ca-02-dependencies.json', 'docs/ca-03b-dependencies.json', 'docs/ca-03c-dependencies.json'}
    if set(review['predecessor_sha256']) != required:
        raise ValueError('Prior review set changed')
    for path, expected_hash in review['predecessor_sha256'].items():
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != expected_hash:
            raise ValueError('Prior review bytes changed')
    if (inherited['schema'] != 'codex-accounts/dependency-review/v1'
            or review['schema'] != 'codex-accounts/dependency-review/v1'
            or review['status'] != 'source_reviewed'
            or storage['schema'] != 'codex-accounts/dependency-review/v1'
            or storage['status'] != 'source_reviewed'
            or crypto['schema'] != 'codex-accounts/dependency-review/v1'
            or crypto['status'] != 'source_reviewed'):
        raise ValueError('Unknown or incomplete dependency review')
    if hashlib.sha256(lock_bytes).hexdigest() != review['lock_sha256']:
        raise ValueError('Reviewed lock bytes changed')
    actual_packages = [p for p in lock['package'] if 'source' in p]
    expected_packages = inherited['packages'] + crypto['packages'] + storage['packages'] + review['packages']
    keyed = lambda items: {(p['name'], p['version']): (p['source'], p['checksum']) for p in items}
    actual, expected = keyed(actual_packages), keyed(expected_packages)
    if (len(actual) != len(actual_packages) or len(expected) != len(expected_packages)
            or actual != expected):
        raise ValueError('Dependency graph changed; update the scoped review')
    workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    manifests = {'Cargo.toml'} | {f'{p}/Cargo.toml' for p in workspace['workspace']['members']}
    if set(review['manifest_sha256']) != manifests:
        raise ValueError('Workspace manifest review set changed')
    for path, digest in review['manifest_sha256'].items():
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != digest:
            raise ValueError('Reviewed dependency manifest changed')
    for name in ['core', 'platform', 'runtime']:
        value = tomllib.loads((root / f'crates/{name}/Cargo.toml').read_text(encoding='utf-8'))
        if 'dependencies' in value:
            raise ValueError('Safe bootstrap crate gained dependencies')
    return {'reviewed_packages': len(expected), 'lock_review': 'passed'}


def observed_inputs(root: Path = ROOT) -> dict[str, str]:
    """Diagnostics only: hashes never authorize acquisition or update a review."""
    paths = ['Cargo.lock', 'Cargo.toml', 'docs/ca-04a-dependencies.json']
    paths += [str(p.relative_to(root)).replace('\\', '/')
              for p in sorted((root / 'crates').glob('*/Cargo.toml'))]
    return {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths}


if __name__ == '__main__':
    print(json.dumps({'observed_source_inputs_not_approval': observed_inputs()}, sort_keys=True), flush=True)
    print(json.dumps(check(), sort_keys=True))
