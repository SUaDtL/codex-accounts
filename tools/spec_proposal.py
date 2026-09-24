"""Render and verify the in-place CA-02 T-07 review amendment and provenance.

--check verifies the checked-in candidate and its v2 source manifest. --preview
and --manifest-preview reproduce them without writing files. This document
amendment grants no product-operation or native-qualification authority.
"""
from __future__ import annotations

import argparse
import hashlib
from html import escape
import json
from pathlib import Path
import re

if __package__:
    from . import spec_projection
else:
    import spec_projection

ORIGINAL_SHA256 = 'a5041d63e0b517f68019a4b527c050014616e7bd46c5742768a65199120ad3e6'
OLD = ('file, keyring, auto-with-file, auto-with-keyring, encrypted/secrets, ephemeral '
       'and inaccessible-store fixtures. Only qualified explicit file mode proceeds.')
NEW = ('Explicit file, positively established exact-build official file default, keyring, '
       'auto-with-file, auto-with-keyring, encrypted/secrets, ephemeral, inaccessible-store '
       'and unresolved-precedence fixtures. Only an otherwise-qualified installation with '
       'a positively established effective file backend is eligible. A missing setting '
       'alone never establishes that default. Auto, keyring, unsupported sources and '
       'unresolved evidence remain ineligible; no backend or keyring state is changed.')
REVISION = 'ca-02-t07-proposal-1'
MODEL = re.compile(r'(<script type="application/json" id="artifact-model">)(.*?)(</script>)', re.S)
ROW = re.compile(r'(<tr id="T-07" class="test-row"><td>T-07</td><td>)(.*?)(</td></tr>)', re.S)
CANDIDATE_SHA256 = 'af8171ed2e69cdca214afc5a569f060e70582033c72829594c1c81f02fd82d43'
SPEC_PATH = 'docs/local-codex-switcher-spec.html'
UNCHANGED = {
    'docs/codex-switchers-comparison.html': '3928997418de0c6871f7af4edbf8760ca1d7c049fd1f99850e5580c08aeebb57',
    'docs/maintainers/GITHUB_CONNECTOR_INDEX.md': '9cb896bd347db0c566e007fc846f4a2bb10f0880feb498371827e8a07ddc730c',
}


def render(html: str) -> str:
    match = MODEL.search(html)
    if not match or len(MODEL.findall(html)) != 1 or len(ROW.findall(html)) != 1:
        raise ValueError('Expected exactly one canonical model and T-07 projection')
    data = json.loads(match.group(2))
    spec_projection.check(html, data)
    rows = next(b['rows'] for s in data['sections'] if s['id'] == 'SPEC-TESTS'
                for b in s['blocks'] if b['type'] == 'table')
    selected = [r for r in rows if r[0] == 'T-07']
    if len(selected) != 1 or selected[0][1] not in {OLD, NEW}:
        raise ValueError('Unrecognized T-07 source; reconcile instead of overwriting')
    selected[0][1] = NEW
    data['document_revision'] = REVISION
    # Proposed on the review branch; owner review/merge determines adoption.
    data['amendment_status'] = 'proposed_for_owner_review'
    result = MODEL.sub(lambda m: m[1] + json.dumps(data, ensure_ascii=False,
                                               separators=(',', ':')) + m[3], html)
    result = ROW.sub(lambda m: m[1] + escape(NEW) + m[3], result)
    spec_projection.check(result, data)
    return result


def reconstruct_original(candidate: str) -> str:
    """Invert only this amendment and verify the complete original input bytes."""
    if hashlib.sha256(candidate.encode('utf-8')).hexdigest() != CANDIDATE_SHA256:
        raise ValueError('Candidate changed outside the scoped T-07 amendment')
    data = json.loads(MODEL.search(candidate).group(2))
    spec_projection.check(candidate, data)
    if data.pop('document_revision') != REVISION:
        raise ValueError('Unexpected document revision')
    if data.pop('amendment_status') != 'proposed_for_owner_review':
        raise ValueError('Unexpected amendment state')
    rows = next(b['rows'] for s in data['sections'] if s['id'] == 'SPEC-TESTS'
                for b in s['blocks'] if b['type'] == 'table')
    next(r for r in rows if r[0] == 'T-07')[1] = OLD
    original = MODEL.sub(lambda m: m[1] + json.dumps(data, ensure_ascii=False,
                                                 separators=(',', ':')) + m[3], candidate)
    original = ROW.sub(lambda m: m[1] + escape(OLD) + m[3], original)
    if hashlib.sha256(original.encode('utf-8')).hexdigest() != ORIGINAL_SHA256:
        raise ValueError('Original source provenance cannot be reconstructed')
    if render(original) != candidate:
        raise ValueError('Amendment is not the deterministic scoped transformation')
    return original


def original_manifest() -> dict:
    """Immutable reviewed source basis, not a second editable specification."""
    return {
        'schema': 'codex-accounts/source-inputs/v1',
        'date': '2026-09-22',
        'files': [{'path': SPEC_PATH, 'sha256': ORIGINAL_SHA256}]
                 + [{'path': path, 'sha256': digest} for path, digest in UNCHANGED.items()],
    }


def proposed_manifest(root: Path, candidate: str) -> dict:
    reconstruct_original(candidate)
    manifest = original_manifest()
    manifest['schema'] = 'codex-accounts/source-inputs/v2'
    manifest['files'][0].update(
        original_sha256=ORIGINAL_SHA256, revision=REVISION,
        amendment='docs/spec-amendments/ca-02-t07.md', sha256=CANDIDATE_SHA256)
    current = json.loads((root / 'docs/source-inputs.json').read_text(encoding='utf-8'))
    if current not in (original_manifest(), manifest):
        raise ValueError('Source manifest differs from the reviewed provenance migration')
    return manifest


def check_candidate(root: Path) -> None:
    """Require the in-place candidate; a v1 manifest alone no longer suffices."""
    raw = (root / SPEC_PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CANDIDATE_SHA256:
        raise ValueError('Checked-in specification is not the reviewed T-07 candidate')
    candidate = raw.decode('utf-8')
    expected = proposed_manifest(root, candidate)
    current = json.loads((root / 'docs/source-inputs.json').read_text(encoding='utf-8'))
    if current != expected:
        raise ValueError('The in-place amendment requires its v2 provenance manifest')
    for path, digest in UNCHANGED.items():
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != digest:
            raise ValueError('An unchanged supplied source has drifted')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--check', action='store_true')
    modes.add_argument('--preview', action='store_true')
    modes.add_argument('--manifest-preview', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    try:
        check_candidate(root)
        candidate = render((root / SPEC_PATH).read_text(encoding='utf-8'))
        if args.preview:
            print(candidate, end='')
        elif args.manifest_preview:
            print(json.dumps(proposed_manifest(root, candidate), indent=2))
        else:
            print('In-place T-07 candidate, projection and original-source provenance verified')
        return 0
    except (ValueError, OSError, KeyError) as exc:
        parser.exit(1, f'Specification amendment check failed: {exc}\n')


if __name__ == '__main__':
    raise SystemExit(main())
