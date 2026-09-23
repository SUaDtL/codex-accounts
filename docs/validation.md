# Validation and evidence boundary

## Current review: CA-03B / PR #5

PR #5 is stacked on unmerged PR #3 at
`595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`. It implements library-only
XChaCha20-Poly1305 envelopes, HKDF-SHA256-separated keys, OS randomness,
owned-buffer zeroization and current-user Windows x64 DPAPI. No persistent
vault, real credential read, login, process control, UI or product authority exists.
The dependency's T-07 amendment is unchanged and requires its own owner review.

| Evidence | Observed result and scope |
| --- | --- |
| Initial CA-03B CI | Head `7c963287ae7fa799c075dc6ffa5470622f54fa77`, PR run [35864627512](https://github.com/SUaDtL/codex-accounts/actions/runs/35864627512): source/Python passed; all three Rust jobs failed only formatting. Tests and Clippy passed independently. |
| Formatter correction | Exact rustfmt 1.90.0 patch from preparation run 35895393270, source `4103cd6123debcbe5f997708c0c42d4593c27ec1`, applied without changing behavior or assertions. Temporary preparation workflow removed. Preparation is not validation. |
| Local closeout | 71 Python tests passed; specification/projection/provenance and 58-package lock/manifest checks passed. The extracted baseline Git tree matched the remote tree before editing. |
| Local Rust/native | NOT RUN: no local Rust toolchain or Windows session. Hosted results are never represented as local execution. |
| Final-head CI | PR #5 records the actual final commit, completed PR/push workflow IDs and inspected native logs after publication. No older head or preparation run counts as a current pass. |
| Native evidence pending | Three ignored two-user evidence helpers are NOT RUN, not passes. Distinct-user controls, unavailable-store behavior and independent macOS Keychain remain open. |

The crypto suite exercises exact bytes and empty payloads, every expected-context
field, all single-byte mutations, truncated/extended/overflowing formats, versions,
wrong keys, nonce/root/randomness failures, presence-aware fingerprints and redaction.
The independent synthetic vector is a cross-implementation check, not native account
or complete T-15/T-16/T-31 closure. Windows tests exercise actual DPAPI with synthetic
data; ca-03b-crypto.md provides the exact ordinary-user collection procedure.

Scope/security/recovery review: expected context must be independently selected;
valid AEAD is not freshness or protection from whole-vault rollback. No unchecked
key migration, plaintext fallback, public entropy override or raw root-key
constructor is exposed. Native unsafe is limited to the documented DPAPI module;
core/platform/runtime safeguards remain. No real credentials or synthetic DPAPI
fixture are committed. Only the opt-in test helper writes its encrypted test file.

The 26 new dependencies supplement 32 unchanged checksum records. Full lock and
manifest/feature checks precede acquisition; formatting, tests and Clippy remain
blocking. The normative source, provenance, existing CI workflow and empty catalog
are unchanged by this closeout. See dependency-policy.md and ca-03b-dependencies.json.

## Preserved earlier evidence

| Increment | Exact evidence and scope |
| --- | --- |
| CA-03A / PR #3 | Head `595eebdb8d5d561ec855e6ef63a6aec3c10ad3ea`; PR run 35819942583 and push run 35819939120 passed. Source job 107049451062 recorded 62 Python passes; Windows job 107049451273 recorded 66 Rust passes, including 23 vault-data cases and nine discovery/native API cases. Formatting and Clippy passed on all three platforms. These are historical results, not CA-03B validation. |
| CA-02 / PR #2 | Owner merged head `6ee778e9592d46132f253e2bee836b2650a44bd0` as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. Run 35808957582 passed: 48 Python tests and 43 Rust tests in the inspected Windows log. Later T-07 adoption and CI acquisition repair are not attributed to this merge. |
| Foundation / PR #1 | Merge `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`; tested head `9b9328f474322c9de48dc6a8db332ee8105899dd`, runs 35761967137 and 35761962743 passed: 35 Python and 19 Rust tests, formatting and Clippy. |

PR #3 implements the remaining P02-A08 canonical/visible T-07 change and v2
provenance migration. Original bytes reconstruct to the preserved digest; all
43 requirements and 34 acceptance IDs remain. Review/merge of that amendment is
not a qualification record. CA-03A supplies strict data/generation rules, not
verified Codex identity, durable storage or complete recovery references.

CA-02's native reader, conservative configuration observations, path/refusal and
redaction tests are delivered. Actual official publisher/runtime binding,
effective home/policy, auth resources/identity, writer lifecycle and actor-attributed
traces remain Q0 evidence gaps. Earlier hosted Windows was Server 2025 x64, not an
owner's Windows 11 Desktop qualification. See q0-qualification.md and PR #2's
checklist disposition for the detailed discovery evidence.

## Remaining gates

CA-03C must implement protected local storage, independently selected context,
root-key bootstrap, encrypted registry/immutable generations, exclusive locking,
write-ahead commits and startup recovery, with native path/ACL/durability and
retention tests. No persistent account storage is exposed before those obligations.
Two-user and unavailable-store evidence remains required for full native key
protection; macOS qualification is independent. Q0 facts cannot be invented to
activate a later adapter. Coordinator/recovery, official login, Tauri UI and all
complete release scenarios remain pending. No automatic CA-04 advancement.
