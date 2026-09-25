# Current review: PR #13 integration and CA-05B

Continue `feat/ca-04d-staging-recovery-ci` in PR #13. The owner requested the next
slice and resolution of its conflicts in the same PR. Observed parents:
CA-04D `345533eacd4ed4ed288fa7a4580dd4e408106ede` and merged main
`d64d3e31712798b1b790c70c3830c6853a67c1a2` (PR #12). Preserve both histories;
never reset main or force-push this branch. Refresh live refs before another edit.

## Selected increment

CA-05B adds an opt-in strict JSON-object decoder over CA-05A framing. Reuse the
existing vault parser and exact zeroize dependency; reject malformed/non-object
JSON, decoded duplicate keys, depth overflow and transport bounds. Retain bytes,
poison failed streams and keep diagnostics redacted. Object syntax is not an RPC
schema, method authorization, response/login ID or identity observation.

The join preserves CA-04D encrypted staging repair and complete/disjoint CI,
CA-05A framing tests, CA-06A static views/tests and macOS documentary preparation.
Resolve the five conflicting routing/plan/validation files rather than choosing
one side wholesale. Reissue the macOS source baseline with its predecessor and
scoped interface review; native construction remains unavailable.

Read live AGENTS.md, SPEC-PROTOCOL, S-003/S-011/S-012/S-015, T-25/T-26/T-31,
`ca-05b-json-objects.md`, the unchanged strict parser, runtime APIs/tests and the
CA-04D recovery/CI reviews. One integrator owns shared records and publication.

## Validation and finish

Run source/projection/provenance/dependency and Python checks, preview parity,
macOS source consistency, pinned formatting, workspace debug/release partitions,
doctests, optimized build, all 27 named Windows behaviors in both profiles and
Clippy with warnings denied. Hosted isolated Linux remains mandatory. Inspect
completed logs for the final source and tested merge/tree. Failed or unavailable
checks remain explicit; never inherit an earlier SHA's green status.

## Next eligible step after review

Schema-independent request/handshake correlation and deadline tests can be selected
as a later CA-05 preparation slice. A production schema must come from an approved
exact runtime, not a synthetic fixture. No official helper/login/refresh, native
consent or credential access is enabled by CA-05B.

Operational CA-05 still needs exact Q0/Q2 and owned-helper lifecycle. macOS native
coding needs owner selection, CA-04D review, exact SDK/binding review and an
authorized host. Browser/keyboard/zoom/screen-reader, two-user, directory-metadata
and physical-power-loss evidence remain separate in `maintainers/open-asks.md`.
No merge of this PR, release, permission change or live account operation.
