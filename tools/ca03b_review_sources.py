"""Temporary source-inspection transport. Downloads pinned source, never executes it."""
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tomllib
import urllib.request

OUT = Path('ca03b-review-artifacts')
LOCK_SHA = 'fb272c5d9e2147b2c3fa4dd8741f276af1dd86772c007cca29da8d369c402767'
ADVISORIES = '6477ec04375b913e13f38d966dc49eba9d178cb8'
assert hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest() == LOCK_SHA
packages = json.loads((OUT / 'registry-metadata.json').read_text())
lock = tomllib.loads(Path('Cargo.lock').read_text())
packages += [p for p in lock['package'] if p['name'] == 'windows-sys']


def download(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'codex-accounts-source-review'})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read(16 * 1024 * 1024 + 1)
    assert len(data) <= 16 * 1024 * 1024, 'Source archive size limit'
    return data


review = []
for package in packages:
    name, version = package['name'], package['version']
    source = download(f'https://static.crates.io/crates/{name}/{name}-{version}.crate')
    assert hashlib.sha256(source).hexdigest() == package['checksum'], 'Source checksum mismatch'
    prefix = f'{name}-{version}/'
    target = OUT / 'sources' / f'{name}-{version}'
    build_files = {}
    manifest = None
    total = 0
    with tarfile.open(fileobj=io.BytesIO(source), mode='r:gz') as archive:
        for member in archive:
            assert member.name.startswith(prefix), 'Unexpected archive prefix'
            rel = Path(member.name[len(prefix):])
            assert not rel.is_absolute() and '..' not in rel.parts
            if not member.isfile():
                assert member.isdir(), 'No archive links allowed'
                continue
            keep = rel.suffix == '.rs' or rel.name.startswith(('Cargo.toml', 'LICENSE', 'COPYING', '.cargo_vcs_info'))
            if name == 'windows-sys':
                keep = str(rel) in {'Cargo.toml', 'src/Windows/Win32/Foundation/mod.rs', 'src/Windows/Win32/Security/Cryptography/mod.rs'} or rel.name.startswith(('license', 'LICENSE', '.cargo_vcs_info'))
            if not keep:
                continue
            total += member.size
            assert member.size <= 8 * 1024 * 1024 and total <= 32 * 1024 * 1024
            data = archive.extractfile(member).read()
            destination = target / rel
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            if str(rel) == 'Cargo.toml':
                manifest = tomllib.loads(data.decode())
            if rel.name == 'build.rs' or rel.parts[0] == 'build':
                build_files[str(rel)] = hashlib.sha256(data).hexdigest()
    assert manifest is not None
    record = {**package, 'packaged_license': manifest['package'].get('license'), 'packaged_rust_version': manifest['package'].get('rust-version'), 'build': manifest['package'].get('build'), 'build_files': build_files}
    review.append(record)
    print('SOURCE_REVIEW', json.dumps(record, sort_keys=True))
(OUT / 'source-review.json').write_text(json.dumps(review, indent=2))

source = download(f'https://codeload.github.com/RustSec/advisory-db/tar.gz/{ADVISORIES}')
selected = {p['name'] for p in lock['package'] if 'source' in p}
matches = []
with tarfile.open(fileobj=io.BytesIO(source), mode='r:gz') as archive:
    for member in archive:
        if not member.isfile():
            continue
        parts = Path(member.name).parts
        if len(parts) == 4 and parts[1] == 'crates' and parts[2] in selected and parts[3].endswith('.md'):
            assert member.size < 1024 * 1024
            text = archive.extractfile(member).read().decode()
            matches.append({'path': '/'.join(parts[1:]), 'content': text})
(OUT / 'advisories.json').write_text(json.dumps({'commit': ADVISORIES, 'archive_sha256': hashlib.sha256(source).hexdigest(), 'matches': matches}, indent=2))
print('ADVISORY_MATCHES', json.dumps(matches, sort_keys=True))
print('REVIEW INPUTS ONLY. No dependency code or build script was executed.')
