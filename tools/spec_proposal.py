"""Render the CA-02 T-07 proposal; never grants product operation authority.

Run from the reviewed repository: python tools/spec_proposal.py --check
--preview emits the review candidate; --manifest-preview emits its proposed
provenance migration. Neither command changes repository or application state.
"""
from __future__ import annotations

import argparse
import hashlib
from html import escape
import json
from pathlib import Path
import re

if __package__:
    from . import spec_index, spec_projection
else:
    import spec_index
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
    # This is a proposed contract on a review branch, not a compatibility receipt.
    data['amendment_status'] = 'proposed_for_owner_review'
    result = MODEL.sub(lambda m: m[1] + json.dumps(data, ensure_ascii=False,
                                               separators=(',', ':')) + m[3], html)
    result = ROW.sub(lambda m: m[1] + escape(NEW) + m[3], result)
    spec_projection.check(result, data)
    return result


CANDIDATE_SHA256 = 'af8171ed2e69cdca214afc5a569f060e70582033c72829594c1c81f02fd82d43'


def proposed_manifest(root: Path, candidate: str) -> dict:
    manifest = json.loads((root / 'docs/source-inputs.json').read_text(encoding='utf-8'))
    manifest['schema'] = 'codex-accounts/source-inputs/v2'
    entry = next(e for e in manifest['files'] if e['path'] == 'docs/local-codex-switcher-spec.html')
    entry.update(original_sha256=ORIGINAL_SHA256, revision=REVISION,
                 amendment='docs/spec-amendments/ca-02-t07.md',
                 sha256=hashlib.sha256(candidate.encode('utf-8')).hexdigest())
    return manifest


def check_candidate(root: Path) -> None:
    path = root / 'docs/local-codex-switcher-spec.html'
    if hashlib.sha256(path.read_bytes()).hexdigest() != ORIGINAL_SHA256:
        raise ValueError('Original specification changed; reconcile the pending proposal')
    candidate = render(path.read_text(encoding='utf-8'))
    if hashlib.sha256(candidate.encode('utf-8')).hexdigest() != CANDIDATE_SHA256:
        raise ValueError('T-07 proposal changed outside its scoped review')
    entry = proposed_manifest(root, candidate)['files'][0]
    if entry['original_sha256'] != ORIGINAL_SHA256 or entry['sha256'] != CANDIDATE_SHA256:
        raise ValueError('Invalid proposed source-provenance migration')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--check', action='store_true')
    modes.add_argument('--preview', action='store_true')
    modes.add_argument('--manifest-preview', action='store_true')
    args = parser.parse_args()
    root = spec_index.ROOT
    check_candidate(root)
    candidate = render(spec_index.SPEC.read_text(encoding='utf-8'))
    if args.preview:
        print(candidate, end='')
    elif args.manifest_preview:
        print(json.dumps(proposed_manifest(root, candidate), indent=2))
    else:
        print('T-07 candidate and provenance migration verified; normative source unchanged')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
