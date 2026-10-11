# Current packet: CA-05D on PR #14

Resume PR #14, `feat/ca-05c-session-correlation`. CA-05C parent:
`db7fcfced20c3fab1f602991fd6e56712ec016b6`. Main remains owner-merged PR #13,
`85aff5e32e6619d7250bf7d4dbfcffefbc016043`. Preserve the matching branch, inspect
actual head/reviews/checks and use non-force publication. Do not open a duplicate.

## Implemented increment

CA-05D extends the existing sealed sequencer with bounded login-ID matching, early
completion buffering and all completion/cancel orderings. The start reply uses the
existing request deadline; interactive login has one 600-second deadline. Cancel
has a separate fixed five-second protocol-cleanup bound and distinct acknowledgement,
error and timeout evidence. Neither a success report nor cancel acknowledgement
means helper exit, credential capture or committed onboarding. Constructors stay
test-only; no actual login, official schema, URL or credential IO is introduced.

Read live AGENTS.md, SPEC-PROTOCOL, F-017/F-019, S-011/S-012, T-24/T-25/T-26/T-31,
`ca-05d-login-sequencing.md`, `runtime/src/session.rs`, `session_login.rs` and tests.
Reuse CA-05C allocation/ownership and the existing coordinator. No dependency,
native, shared storage, workflow or qualification-catalog changes belong here.

## Finish this review

Verify pinned formatting, runtime/workspace debug and release, doctests, optimized
build, Clippy, source/Python/dependency/projection checks and all inherited hosted
jobs, including 54 exact-name Windows native executions and isolated Linux. Review
cancel races, ID bounds, original-error retention, no deadline resets and no fake
helper/credential receipts. Only the final head/tested tree and completed logs can
establish current execution. All owner/native qualification omissions remain open.

## Next eligible slice

CA-05E can implement a private synthetic transport-driver boundary: independently
bounded input/output and discarded stderr, real scheduling/cancellation behavior,
private reader ownership and refusal/EOF tests using owned synthetic endpoints.
Keep production construction disabled; do not fabricate an official schema or use
real auth. Exact version decoding, native helper ownership, stop/reap/capture and
onboarding recovery integration require their own reviewed dependencies and Q0/Q2
evidence. The native evidence collection steps remain in `maintainers/open-asks.md`.
No automatic phase advance, merge, release, permission change or live account action.
