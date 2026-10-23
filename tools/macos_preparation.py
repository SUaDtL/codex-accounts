"""Source-drift check for a macOS preparation packet; never native qualification.

Only fixed repository sources are read. No platform probe, Keychain, process,
credential, network or permission operation is available through this tool.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
PACKET = 'docs/macos/interface-baseline-windows-storage.json'
REVIEW = 'docs/macos/windows-storage-interface-review.json'
PREVIOUS = 'docs/macos/interface-baseline.json'
HISTORICAL = {
    PREVIOUS: '04302afd28debed0a7f845ec1b634458dac6b4f5d522f5c8c2d205f586c65b5b',
    'docs/macos/interface-baseline-ca04c.json': 'baf3b499429d71fa7e4965e58ad5cb9316d68c35fd30eb1a2c399c45dc6efaa1',
    'docs/macos/ca-04d-interface-review.md': 'b88abdb72a4237af658954a12ff2f50e098b647a2d9de97049825c73544e595b',
}
BASE_COMMIT = '727eb6154db1d7c4ac5db10a7868b6271cd55293'
ENGINE = 'crates/vault-storage/src/engine.rs'
CHANGED = ('crates/vault-crypto/src/keys.rs', ENGINE,
           'crates/vault-storage/src/coordinator.rs',
           'crates/vault-storage/src/journal.rs')
SOURCES = (
    'crates/core/src/lib.rs',
    'crates/platform/src/lib.rs',
    'crates/vault-crypto/src/keys.rs',
    'crates/vault-storage/src/engine.rs',
    'crates/vault-storage/src/coordinator.rs',
    'crates/vault-storage/src/journal.rs',
    'crates/vault-storage/src/lifecycle_model.rs',
    'crates/vault-storage/src/stage_repair.rs',
)
MAX_BYTES = 256 * 1024
MAX_RECORD = 16 * 1024


class PacketError(ValueError):
    """Fixed messages exclude rejected paths and source contents."""


def pairs(items):
    result = {}
    for key, value in items:
        if key in result:
            raise PacketError('Duplicate preparation field')
        result[key] = value
    return result


def decode(raw: bytes) -> dict:
    if not isinstance(raw, bytes) or len(raw) > MAX_RECORD:
        raise PacketError('Preparation record exceeds bound')
    try:
        return json.loads(raw, object_pairs_hook=pairs)
    except (ValueError, UnicodeError, RecursionError):
        raise PacketError('Invalid preparation record') from None


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw.replace(b'\r\n', b'\n')).hexdigest()


def interface_set(bindings, names) -> None:
    if not isinstance(bindings, dict) or set(bindings) != set(names):
        raise PacketError('Unexpected interface set')
    if any(not isinstance(v, str) or not re.fullmatch('[0-9a-f]{64}', v)
           for v in bindings.values()):
        raise PacketError('Invalid interface digest')


def header(record, fields, version, base, evidence='source-baseline-only') -> None:
    if not isinstance(record, dict) or set(record) != fields:
        raise PacketError('Unexpected preparation fields')
    expected = {'packet_format': version, 'base_commit': base,
                'platform': 'macos-aarch64', 'evidence': evidence, 'native_status': 'not-run'}
    if any(not isinstance(record[key], str) or record[key] != value
           for key, value in expected.items()):
        raise PacketError('Preparation is not native qualification')
    for field in fields & {'predecessor_sha256', 'review_sha256'}:
        if not isinstance(record[field], str) or not re.fullmatch('[0-9a-f]{64}', record[field]):
            raise PacketError('Invalid review digest')


def bounded(read_source, name, limit=MAX_RECORD) -> bytes:
    try:
        raw = read_source(name)
    except (OSError, KeyError):
        raise PacketError('Preparation source unavailable') from None
    if not isinstance(raw, bytes) or len(raw) > limit:
        raise PacketError('Interface exceeds bound')
    return raw


def historical_chain(read_source) -> dict:
    records = {}
    for path, expected in HISTORICAL.items():
        raw = bounded(read_source, path, MAX_BYTES if path.endswith('.md') else MAX_RECORD)
        if digest(raw) != expected:
            raise PacketError('Shared interface review provenance changed')
        if path.endswith('.json'):
            records[path] = decode(raw)
    common = {'packet_format', 'base_commit', 'platform', 'evidence', 'native_status', 'interfaces'}
    previous = records[PREVIOUS]
    header(previous, common | {'predecessor_sha256', 'review_sha256'},
           'codex-accounts/macos-preparation/v2', '345533eacd4ed4ed288fa7a4580dd4e408106ede')
    interface_set(previous['interfaces'], SOURCES)
    earlier_path = 'docs/macos/interface-baseline-ca04c.json'
    header(records[earlier_path], common, 'codex-accounts/macos-preparation/v1',
           '455baa2eeee614586b26d419ee961475fe1ad23b')
    interface_set(records[earlier_path]['interfaces'], SOURCES[:-1])
    if (previous['predecessor_sha256'] != HISTORICAL[earlier_path] or
            previous['review_sha256'] != HISTORICAL['docs/macos/ca-04d-interface-review.md']):
        raise PacketError('Shared interface review provenance changed')
    return previous


def semantic_review(review, record, previous) -> None:
    fields = {'packet_format', 'base_commit', 'source_state', 'platform', 'evidence',
              'native_status', 'predecessor_sha256', 'changed_sources',
              'unchanged_interfaces', 'assessment'}
    header(review, fields, 'codex-accounts/macos-source-review/v2', BASE_COMMIT, 'source-review-only')
    changed = review['changed_sources']
    if (review['source_state'] != 'reviewed-working-tree' or
            review['predecessor_sha256'] != HISTORICAL[PREVIOUS] or
            not isinstance(changed, list) or len(changed) != len(CHANGED)):
        raise PacketError('Shared interface semantic review changed')
    for item, name in zip(changed, CHANGED):
        if (not isinstance(item, dict) or
                set(item) != {'path', 'before_sha256', 'after_sha256'} or
                item['path'] != name or
                item['before_sha256'] != previous['interfaces'][name] or
                item['after_sha256'] != record['interfaces'][name]):
            raise PacketError('Shared interface semantic review changed')
    interface_set(review['unchanged_interfaces'], set(SOURCES) - set(CHANGED))
    if any(review['unchanged_interfaces'][name] != previous['interfaces'][name] or
           record['interfaces'][name] != previous['interfaces'][name]
           for name in SOURCES if name not in CHANGED):
        raise PacketError('Shared interface semantic review changed')
    assessment = review['assessment']
    if (not isinstance(assessment, dict) or
            set(assessment) != {'reason', 'recognition', 'authentication', 'opening_recovery', 'formats', 'limits'} or
            any(not isinstance(value, str) or not value.strip() or len(value) > 2048
                for value in assessment.values())):
        raise PacketError('Invalid semantic assessment')


def validate(record: dict, read_source) -> None:
    fields = {'packet_format', 'base_commit', 'platform', 'evidence', 'native_status', 'interfaces',
              'predecessor_sha256', 'review_sha256', 'source_state'}
    header(record, fields, 'codex-accounts/macos-preparation/v3', BASE_COMMIT)
    if record['source_state'] != 'reviewed-working-tree':
        raise PacketError('Preparation is not native qualification')
    bindings = record['interfaces']
    # Validate all names/hashes before touching even a fixed repository source.
    interface_set(bindings, SOURCES)
    for name in SOURCES:
        if digest(bounded(read_source, name, MAX_BYTES)) != bindings[name]:
            raise PacketError('Shared interface changed; review macOS packet before integration')
    previous = historical_chain(read_source)
    if record['predecessor_sha256'] != HISTORICAL[PREVIOUS]:
        raise PacketError('Shared interface review provenance changed')
    raw = bounded(read_source, REVIEW)
    if digest(raw) != record['review_sha256']:
        raise PacketError('Shared interface review provenance changed')
    semantic_review(decode(raw), record, previous)


def check(root: Path = ROOT) -> None:
    # Bound reads before allocating arbitrarily large files. Paths are not input.
    def read(name: str) -> bytes:
        with (root / name).open('rb') as stream:
            limit = MAX_BYTES if name in SOURCES or name.endswith('.md') else MAX_RECORD
            return stream.read(limit + 1)
    with (root / PACKET).open('rb') as stream:
        record = decode(stream.read(MAX_RECORD + 1))
    validate(record, read)


if __name__ == '__main__':
    try:
        check()
    except (OSError, PacketError):
        raise SystemExit('macOS preparation check failed; no native qualification') from None
    print('macOS interface baseline: unchanged; native behavior: NOT RUN; qualification: not established')
