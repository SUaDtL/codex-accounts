# Validation and evidence boundary

## Current review increment

PR #3 targets main following the owner's CA-02 merge. It delivers the CA-03A
in-memory data slice plus the remaining CA-02 in-place T-07/provenance amendment
and dependency-acquisition guard. It is not an encrypted persistent vault, a
qualified Desktop adapter or a consumer release. No production gate is enabled.

The specification candidate and v2 manifest are applied together. The original
specification bytes reconstruct to the retained SHA-256; the current candidate,
revision, amendment reference and unchanged comparison/index inputs are pinned.
Full model/projection checks, 43 requirements and 34 acceptance IDs are retained.
See `spec-amendments/ca-02-t07.md` for exact scope and owner review status.

| Evidence | Executed result and boundary |
| --- | --- |
| Local document checks | 20 Python projection/provenance tests passed, comprising the existing 13 and seven new migration regressions. `spec_index.py check`, the amendment check and byte-identical preview reproduction passed. |
| Local CI-contract checks | Five dependency-acquisition contract tests passed. They inspect the fixed reviewed workflow layout, not arbitrary YAML semantics. |
| Local Rust/native execution | NOT RUN: the preparation host has no Rust toolchain/native Windows environment. No hosted result is described as local execution. |
| Earlier CA-03A head | `dbd79962558526798d97103dcaba61ec84a39cc9`: PR run [35817489625](https://github.com/SUaDtL/codex-accounts/actions/runs/35817489625) and push run [35817486337](https://github.com/SUaDtL/codex-accounts/actions/runs/35817486337) both completed successfully before the in-place amendment commit. |
| Inspected logs for that head | Source job 107042038473: 55 Python passes and 32-package lock review. Windows job 107042038650: 66 Rust passes, none ignored/filtered; 13 core, 16 discovery, two diagnostic, three platform, nine runtime and 23 vault cases. Formatting and Clippy with `-D warnings` passed. |
| Final-head validation | The PR description records the actual final SHA and completed workflow IDs after this source change. Older green heads are not transferred to the new document/test commit. |
| Native evidence boundary | Hosted Windows Server 2025 x64 build 26100 exercised nine native discovery/API cases with synthetic objects. It did not qualify an owner's Windows 11 Codex Desktop installation. |
| Dependencies | Existing 32 external package/version/checksum records unchanged. No new external dependency, crypto primitive, build script or native binding in CA-03A. |

The vault's 23 synthetic cases cover exact bytes/absence, malformed and duplicate
JSON, object/array depth, Unicode keys/identity bounds, composite identity,
stale-parent/reused-generation refusal, rejected-append atomicity and retention
references. They do not prove token validity, native protected storage, complete
journal references or durable multi-process atomicity.

Windows dependency review and fetch now run in separate blocking steps. The prior
combined PowerShell step could execute fetch after review failure and report the
later native exit code. Independent tests and Clippy explicitly require successful
review and acquisition. No check, assertion, platform or warning policy was relaxed.
Run 35817226733 exposed one rustfmt line wrap in the new data tests; its independent
tests and Clippy passed. The formatter-only correction passed the dbd7996 runs above.

## CA-02 checklist disposition

| Item | Implementation and evidence |
| --- | --- |
| P02-A01 | Concrete current-user registered-package reader; no/single/multiple selection cases; OS inventory or explicit policy-refusal outcome. |
| P02-A02 | Identity disagreement/detached-runtime refusal, package-relative safe reads and re-observation. No positive official binding exists; cached signature acceptance is never official identity. |
| P02-A03 / A04 | Distinct declared/configuration observations and conservative effective context. Missing layer/default/policy evidence remains unknown, never inferred from the diagnostic shell. |
| P02-A05 | Native synthetic ACL, junction, hardlink, sharing and replacement cases ran on hosted Windows. Complete later vault/write-path protection remains out of scope. |
| P02-A06 | Native open-path instrumentation and exclusive synthetic auth fixture exercise the selected read scope. No production subprocess/store/control API. Real installation actor-attributed network/filesystem tracing remains a native gate. |
| P02-A07 | Default/debug/error canaries, redirected-local-display refusal and subprocess regression tests. |
| P02-A08 | In-place canonical/visible T-07 candidate and v2 provenance migration implemented in PR #3. Full original-byte reconstruction, current/source pinning and drift regressions pass locally; final hosted evidence is recorded on the PR. Owner review/merge decides adoption. |
| P02-A09 | Product gates always refuse. All-positive synthetic inputs and catalog files cannot enable them; real catalog is empty. |
| P02-A10 | Completed merged-head hosted logs inspected. Every later head needs its own checks; `q0-qualification.md` provides exact safe owner collection and unavailable observations. |
| P02-A11 / A12 | Declared code/test/documentation surface only. One working roadmap and bounded CA-03B pointer. No agent merge, force push, release or live account operation. |

## Pending product and native gates

Official publisher/runtime role, effective home/policy, resource schema/identity,
writer/quit/launch behavior and actor-attributed traces remain unobserved on a real
installation. Follow the ordinary-user read-only Q0 procedure; never send real auth
files or treat an inventory report as a compatibility record. Encrypted envelopes,
native root-key protection, safe persistent storage, coordinator/recovery, official
login, Tauri UI and all complete release scenarios remain later work.

## Preserved source and execution history

The owner merged CA-02 PR #2 from `6ee778e9592d46132f253e2bee836b2650a44bd0`
as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. Run 35808957582 passed all
four jobs at that head: 48 Python tests and 43 Rust tests in the inspected Windows
log. The T-07 in-place change and later acquisition fix are not attributed to that merge.

CA-03A old head `5f954363adc7464651c3bdb0f71f95eb0bb150bd` and owner-rebased
head `922a93cb4d4a82fa3bebbff2a1d8aa7503c1fb53` had the identical Git tree
`5e858eee9cd6118c8ad1b1a40f827eb1272e6332`; normal follow-up commits preserved
that work. Run 35810672800 is historical evidence for the old head only.

PR #1 merged at `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`. Its tested head
`9b9328f474322c9de48dc6a8db332ee8105899dd` passed PR run 35761967137 and push
run 35761962743: 35 Python tests and 19 Rust model tests, formatting and Clippy.
Historical CI, compilation and model-test counts never close native product gates.
