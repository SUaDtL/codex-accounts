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
PACKET = 'docs/macos/interface-baseline.json'
SOURCES = (
    'crates/core/src/lib.rs',
    'crates/platform/src/lib.rs',
    'crates/vault-crypto/src/keys.rs',
    'crates/vault-storage/src/engine.rs',
    'crates/vault-storage/src/coordinator.rs',
    'crates/vault-storage/src/journal.rs',
    'crates/vault-storage/src/lifecycle_model.rs',
)
MAX_BYTES = 256 * 1024


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
    if len(raw) > 16 * 1024:
        raise PacketError('Preparation record exceeds bound')
    try:
        return json.loads(raw, object_pairs_hook=pairs)
    except (ValueError, UnicodeError, RecursionError):
        raise PacketError('Invalid preparation record') from None


def validate(record: dict, read_source) -> None:
    fields = {'packet_format', 'base_commit', 'platform', 'evidence', 'native_status', 'interfaces'}
    if not isinstance(record, dict) or set(record) != fields:
        raise PacketError('Unexpected preparation fields')
    if (record['packet_format'] != 'codex-accounts/macos-preparation/v1' or
            record['base_commit'] != '455baa2eeee614586b26d419ee961475fe1ad23b' or
            record['platform'] != 'macos-aarch64' or
            record['evidence'] != 'source-baseline-only' or
            record['native_status'] != 'not-run'):
        raise PacketError('Preparation is not native qualification')
    bindings = record['interfaces']
    if not isinstance(bindings, dict) or set(bindings) != set(SOURCES):
        raise PacketError('Unexpected interface set')
    # Validate all names/hashes before touching even a fixed repository source.
    if any(not isinstance(v, str) or not re.fullmatch('[0-9a-f]{64}', v)
           for v in bindings.values()):
        raise PacketError('Invalid interface digest')
    for name in SOURCES:
        raw = read_source(name)
        if not isinstance(raw, bytes) or len(raw) > MAX_BYTES:
            raise PacketError('Interface exceeds bound')
        canonical = raw.replace(b'\r\n', b'\n')
        if hashlib.sha256(canonical).hexdigest() != bindings[name]:
            raise PacketError('Shared interface changed; review macOS packet before integration')


def check(root: Path = ROOT) -> None:
    # Bound reads before allocating arbitrarily large files. Paths are not input.
    def read(name: str) -> bytes:
        with (root / name).open('rb') as stream:
            return stream.read(MAX_BYTES + 1)
    with (root / PACKET).open('rb') as stream:
        record = decode(stream.read(16 * 1024 + 1))
    validate(record, read)


if __name__ == '__main__':
    try:
        check()
    except (OSError, PacketError):
        raise SystemExit('macOS preparation check failed; no native qualification') from None
    print('macOS interface baseline: unchanged; native behavior: NOT RUN; qualification: not established')
