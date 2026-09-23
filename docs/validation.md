# Validation and evidence boundary

## CA-03A data increment

PR #3 targets main following the owner's merge of CA-02 and rebase of this
branch. Reviewed old head `5f954363adc7464651c3bdb0f71f95eb0bb150bd` and rebased
head `922a93cb4d4a82fa3bebbff2a1d8aa7503c1fb53` have the identical Git tree
`5e858eee9cd6118c8ad1b1a40f827eb1272e6332`. Existing work was preserved, not reset.
Main was observed at `7773d3472d13b7087179c0819a37fd5030a8f1c9`.

This is in-memory resource validation and generation metadata only. Encryption,
native root-key protection, persistent storage and full CA-03/Q1 acceptance remain
unimplemented. No public credential-operation interface or live integration exists.

The resumption review adds five Rust edge cases alongside the existing 18 data
cases: decoded surrogate-pair duplicate keys and malformed surrogates, object
nesting boundaries, rejected append atomicity, shuffled resources with explicit
optional absence, and UTF-8 byte-based identity bounds. All use synthetic data.
These tests were authored and source-reviewed locally; Rust execution is performed
by the final-head hosted workflow, not claimed in the preparation environment.

The Windows dependency review and Cargo fetch are now separate blocking steps.
Previously a later successful native command could mask the review exit code and
fetch could run after review failure. Tests/Clippy explicitly require successful
review and acquisition. Five Python regression tests cover this boundary and its
negative mutations; all five passed locally. No dependency, assertion, lint or
platform gate was removed or relaxed.

| Evidence | Result and scope |
| --- | --- |
| Earlier CA-03A hosted run | [35810672800](https://github.com/SUaDtL/codex-accounts/actions/runs/35810672800), head `5f954363adc7464651c3bdb0f71f95eb0bb150bd`: all four jobs completed successfully. This is historical evidence, not a green result for the rebased or repaired SHA. |
| Earlier preparation tests | 50 Python tests were recorded passed before this resumption: 48 inherited and two library-boundary cases. Earlier source/projection/provenance checks passed. |
| Local resumption | Five new CI-contract Python tests passed. Rust tools and native execution are unavailable locally. No claim of a local full-suite rerun or Rust pass. |
| Final-head validation | PR #3 records the actual published SHA and completed hosted run IDs after this correction. Inspect source/projection/provenance, Python, formatting, all workspace tests and Clippy for that SHA. Never transfer a superseded head's result. |
| Dependency surface | Existing 32 external package/version/checksum entries remain unchanged. The internal vault lock entry and exact direct Serde pins are retained. |
| Native and product authority | No native vault, protected storage, OAuth, real account capture/switching or Desktop qualification is implemented or tested by this slice. All complete release scenarios remain open. |

Initial CA-03A run 35809705379 compiled and passed data tests and Clippy but failed
formatting. The formatter-only correction passed run 35810179693 at
`543118c5c06e9b794c8bb1fbd3e85410109dfd5f`. Those failures and repairs remain
historical evidence, not justification to weaken current checks.

## CA-02 discovery increment

The owner merged PR #2 from head `6ee778e9592d46132f253e2bee836b2650a44bd0` as
`7773d3472d13b7087179c0819a37fd5030a8f1c9`. Discovery code is implemented and
reviewed; Q0 native contract qualification is not complete. T-07 in-place adoption
also remains open. The new CI gate correction is carried in PR #3, not retroactively
attributed to the CA-02 merge.

| Evidence | Observed result and scope |
| --- | --- |
| Completed CA-02 head-bound CI | [35808957582](https://github.com/SUaDtL/codex-accounts/actions/runs/35808957582), head `6ee778e9592d46132f253e2bee836b2650a44bd0`: all four jobs succeeded. |
| Inspected Windows log | Job 107015928251: 43 Rust tests passed, none ignored/filtered; 13 core, 16 discovery, two CLI, three platform, nine runtime. Nine discovery cases exercised native API behavior with synthetic objects. Formatting and Clippy with `-D warnings` passed. |
| Actual hosted OS | Windows Server 2025 x64 build 26100, not a Windows 11 Desktop qualification. |
| Recorded Python completion | 48 Python tests passed, including 13 projection/provenance/plan cases. Source check passed 43 requirements, 34 acceptance IDs and zero qualified records. |
| Real installation / Q0 contract | NOT RUN; official publisher/runtime role, effective home/policy, auth-resource/identity and lifecycle contracts remain unresolved. |
| Full product/release acceptance | NOT COMPLETE. No OAuth, account handoff, encrypted vault, Tauri UI, release package, signing or reproducibility qualification. |

## CA-02 checklist disposition

| Packet item | Code/evidence disposition |
| --- | --- |
| P02-A01 | Concrete current-user registered-package reader; pure no/single/multiple selection cases; OS inventory or explicit policy-refusal path. No installed Codex fixture is required. |
| P02-A02 | Identity disagreement and detached-runtime tests; package-relative safe reading and re-observation. No positive official binding exists for any publisher or architecture. Cached signature acceptance is never official identity. |
| P02-A03 / A04 | Distinct declared/configuration observations and conservative effective context. Missing layer/default/policy evidence remains unknown, never inferred from the diagnostic shell. |
| P02-A05 | Native synthetic ACL, junction, hardlink, sharing and replacement cases ran on hosted Windows. Complete later vault/write-path protection remains out of scope. |
| P02-A06 | Native open-path instrumentation and exclusive synthetic auth fixture exercise the selected read scope; source review confirms no production subprocess/store/control API. Real installation actor-attributed network/filesystem tracing remains a named native gate, not closed by these tests. |
| P02-A07 | Default/debug/error canaries, redirected-local-display refusal and subprocess regression tests. |
| P02-A08 | PARTIAL: T-07 canonical/visible candidate and provenance migration are reproducibly generated and regression-tested. In-place adoption remains pending; original normative source and manifest are unchanged. The proposal-delivery variation is documented in spec-amendments/ca-02-t07.md. |
| P02-A09 | Product gates remain disabled; all-positive synthetic inputs and catalog files cannot enable them. Real catalog is empty. |
| P02-A10 | Completed merged-head hosted logs inspected. The safe owner procedure lists exact unavailable observations in q0-qualification.md. |
| P02-A11 / A12 | Declared read-only code/test/documentation scope only. Owner merge is observed; no agent merge, force push, release or account mutation. One working roadmap and a separately bounded CA-03A increment. |

The milestone is discovery code with ordinary/native API tests, not full Q0
contract qualification. No test artifact, filled record or boolean grants authority.

## Preserved foundation evidence

PR #1 merged at `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`. Its tested head
`9b9328f474322c9de48dc6a8db332ee8105899dd` passed PR run 35761967137 and push run
35761962743: 35 Python tests and 19 Rust model tests, formatting and Clippy across
Ubuntu/Windows/macOS. These remain historical bootstrap results, not later-head
validation or product acceptance. Unavailable local Rust checks were not relabeled
as locally executed.
