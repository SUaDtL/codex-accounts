"""Verify inherited scoped dependency reviews before build acquisition."""
from __future__ import annotations

import hashlib
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REVIEW = 'docs/ca-08-dependencies.json'
REVIEW_SHA256 = '52c5f461748e79d40b650c8aa7429433eefcfcb335f6335a567afcae32b266b4'
HISTORICAL_REVIEW_SHA256 = {
    'docs/ca-02-dependencies.json': 'ceaffa36b058d90fef01df21634610938aa47755edd8b6a15f7adc7257f6cc5f',
    'docs/ca-03b-dependencies.json': '0071f27fcdd6dd9aba04aee06e9cb75b2177be7fae7999980f05a19c31563c7d',
    'docs/ca-03c-dependencies.json': '81d4e34248cfc53265d0e087d287c5a5a00f3b793202bedf967b4796146bdf15',
    'docs/ca-04a-dependencies.json': '3c613e03a967aff766c1bd2e34d1a00d31cfcb02210bb360289d30ffe7efdd56',
    'docs/ca-04b-dependencies.json': 'ffdec4f5fbaecb40d4992964702e212fd5c5c112cfa3791566168f16463dfae9',
    'docs/ca-05b-dependencies.json': 'af5b9762e7d36ce15bd824edb8354defb95d21570e5818ddf7ea86813fa88f0d',
    'docs/ca-05f-dependencies.json': '118e242fe79f17eaf98ec7ad86e02c2818aaebf2bf7881653e1756074b4c478c',
    'docs/ca-06b-dependencies.json': '2fb9f49d7b9e9ddb2d27f562ce6f45f506c5313fc3f8b6836f76b33201a456b1',
    'docs/ca-02b-dependencies.json': 'c87f0dec8db21a6a736a406b0b0b91f7df00cc94263689440521f949cff41299',
    'docs/ca-07-dependencies.json': '8fe0269c4afc09c552968734fc5abcdc42c80c3f983d71867bba58d0322d0448',
}


def review_json(raw: bytes) -> dict:
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('Duplicate review key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def direct_boundaries(root: Path) -> None:
    for name in ['core', 'platform']:
        value = tomllib.loads((root / f'crates/{name}/Cargo.toml').read_text(encoding='utf-8'))
        if any(key in value for key in ('dependencies', 'build-dependencies', 'dev-dependencies', 'target')):
            raise ValueError('Safe bootstrap crate gained dependencies')
    runtime = tomllib.loads((root / 'crates/runtime/Cargo.toml').read_text(encoding='utf-8'))
    if (any(key in runtime for key in ('build-dependencies', 'dev-dependencies', 'target', 'features'))
            or runtime.get('dependencies') != {
                'codex-accounts-vault': {'path': '../vault', 'version': '=0.1.0'},
                'zeroize': {'version': '=1.8.2', 'default-features': False, 'features': ['alloc']},
                'serde': '=1.0.229', 'serde_json': '=1.0.145',
            }):
        raise ValueError('Runtime dependency boundary changed')
    shell = tomllib.loads((root / 'crates/shell-preparation/Cargo.toml').read_text(encoding='utf-8'))
    if (set(shell) != {'package', 'lints', 'target'} or shell['target'] != {
            'cfg(windows)': {
                'dependencies': {'tauri': {'version': '=2.12.4', 'default-features': False,
                                          'features': ['wry', 'custom-protocol']}},
                'build-dependencies': {'tauri-build': {'version': '=2.7.3', 'default-features': False,
                                                      'features': ['config-json']}},
            }}):
        raise ValueError('Shell dependency boundary changed')
    discovery = tomllib.loads((root / 'crates/discovery/Cargo.toml').read_text(encoding='utf-8'))
    sdk = discovery.get('target', {}).get('cfg(windows)', {}).get('dependencies', {}).get('windows-sys')
    if sdk != {'version': '=0.61.2', 'features': [
            'Win32_Foundation', 'Win32_Security', 'Win32_Security_Authorization',
            'Win32_Security_Cryptography', 'Win32_Security_Cryptography_Catalog',
            'Win32_Security_Cryptography_Sip', 'Win32_Security_WinTrust',
            'Win32_Storage_FileSystem', 'Win32_System_Com', 'Win32_System_Threading',
            'Win32_System_WindowsProgramming', 'Win32_System_IO', 'Win32_System_Ioctl',
            'Win32_System_SystemServices']}:
        raise ValueError('Discovery SDK dependency boundary changed')


def package_map(items: list) -> dict:
    result = {}
    for package in items:
        key = (package['name'], package['version'])
        checksum = package['checksum']
        if (key in result or package['source'] != 'registry+https://github.com/rust-lang/crates.io-index'
                or len(checksum) != 64 or any(c not in '0123456789abcdef' for c in checksum)):
            raise ValueError('Dependency graph changed; update the scoped review')
        result[key] = (package['source'], checksum)
    return result


def check(root: Path = ROOT) -> dict[str, int | str]:
    direct_boundaries(root)
    lock_bytes = (root / 'Cargo.lock').read_bytes()
    lock = tomllib.loads(lock_bytes.decode('utf-8'))
    prior_packages = []
    original_packages = []
    for path, digest in HISTORICAL_REVIEW_SHA256.items():
        raw = (root / path).read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError('Prior review bytes changed')
        previous = review_json(raw)
        if path == 'docs/ca-06b-dependencies.json':
            if (len(prior_packages) != 58 or len(previous['packages']) != 349
                    or package_map(previous['retained_package_identity']) != package_map(prior_packages)):
                raise ValueError('Dependency graph changed; update the scoped review')
            original_packages = prior_packages.copy()
        prior_packages += previous['packages']
    raw = (root / REVIEW).read_bytes()
    review = review_json(raw)
    if hashlib.sha256(raw).hexdigest() != REVIEW_SHA256:
        raise ValueError('Current review bytes changed')
    if review['predecessor_sha256'] != HISTORICAL_REVIEW_SHA256:
        raise ValueError('Prior review set changed')
    if (review['schema'] != 'codex-accounts/dependency-review/v1'
            or review['packet'] != 'CA-08' or review['status'] != 'source_reviewed'):
        raise ValueError('Unknown or incomplete dependency review')
    actual_packages = [p for p in lock['package'] if 'source' in p]
    expected_packages = prior_packages + review['packages']
    actual, expected = package_map(actual_packages), package_map(expected_packages)
    # Historical packages are additive contributions. This review records only
    # the ten packages introduced by optional Argon2id vault hardening.
    if (len(original_packages) != 58 or len(prior_packages) != 411
            or len(review['packages']) != 10 or len(expected) != 421
            or actual != expected):
        raise ValueError('Dependency graph changed; update the scoped review')
    if hashlib.sha256(lock_bytes).hexdigest() != review['lock_sha256']:
        raise ValueError('Reviewed lock bytes changed')
    workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    manifests = {'Cargo.toml'} | {f'{p}/Cargo.toml' for p in workspace['workspace']['members']}
    if set(review['manifest_sha256']) != manifests:
        raise ValueError('Workspace manifest review set changed')
    for path, digest in review['manifest_sha256'].items():
        raw_manifest = (root / path).read_bytes()
        manifest = tomllib.loads(raw_manifest.decode('utf-8'))
        # Owner-selected 2026-10-22 metadata amendment. Removing exactly this one
        # license declaration must reproduce the reviewed manifest byte-for-byte.
        # Dependency pins, features, workspace membership and all other bytes stay bound.
        if path == 'Cargo.toml':
            license_line = b'license = "AGPL-3.0-only"\n'
            declared_license = manifest['workspace']['package'].get('license')
            expected_license = 'AGPL-3.0-only'
        else:
            license_line = b'license.workspace = true\n'
            declared_license = manifest['package'].get('license')
            expected_license = {'workspace': True}
        if (declared_license != expected_license or raw_manifest.count(license_line) != 1
                or hashlib.sha256(raw_manifest.replace(license_line, b'', 1)).hexdigest() != digest):
            raise ValueError('Reviewed dependency manifest changed')
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
