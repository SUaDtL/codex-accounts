# Current packet: CA-05C non-login session sequencing

Base: owner-merged PR #13 at `85aff5e32e6619d7250bf7d4dbfcffefbc016043`.
Resume `feat/ca-05c-session-correlation` and its actual open PR. PR #13 is closed;
keep this bounded continuation together in one follow-on review. Refresh refs,
reviews and checks before another edit. No reset, force push or automatic merge.

## Implemented for review

`crates/runtime/src/session.rs` adds sealed normalized-event sequencing: initialize
request/send/response then initialized-send order, session-local request correlation,
bounded account signals, absolute 10/15/30-second deadlines, credit/lifetime caps
and sticky terminal errors. No production constructor or official wire schema.
Twenty-four Rust tests and four Python boundary checks join the existing CI lanes.

Read live AGENTS.md, SPEC-PROTOCOL, S-011/S-012, F-017/F-019, T-24/T-25/T-26/T-31,
`ca-05c-session-sequencing.md`, the session implementation/tests and CA-05B framing.
Existing storage, recovery, native and CI-partition code is unchanged.

## Finish this review

Validate pinned formatting, runtime/workspace debug and release, doctests, Clippy,
source/Python/dependency/projection checks and every inherited hosted job, including
all 54 exact-name Windows native executions and isolated Linux. Review the full diff
for authority leakage, deadline resets, ID reuse and false EOF/helper-exit claims.
Record completed results for the actual final head and tested tree on the PR.
Old passes do not transfer. Keep native/owner omissions explicit.

## Next eligible slice

CA-05D can add bounded login-ID correlation, early-completion ordering, cancellation
acknowledgement and ten-minute timeout as a separately selected normalized-event
slice. Reuse CA-05C sequencing, not a second transport or credential transaction.
No official wire schema, browser URL or helper may be invented to make those tests
look integrated. Version-bound decoding, independently drained private stdio,
real scheduling/suspend behavior and owned-helper stop/reap/capture still require
their own reviewed adapter and exact Q0/Q2 evidence.

The safe collection procedures and remaining platform/owner inputs are in
`maintainers/open-asks.md`. No native permission change, user-app force-close,
credential operation, release or product qualification is part of this packet.
