# Current review: CA-04D staging recovery and CI efficiency

Parent: CA-04 / Q2. Owner requested the next slice and CI improvements in one PR.
CA-04C / PR #11 is already merged at `455baa2eeee614586b26d419ee961475fe1ad23b`;
continue on `feat/ca-04d-staging-recovery-ci`, not the merged branch. Refresh live
refs/open PRs and reuse the matching review. No automatic phase advancement.

## Implemented for this review

Explicit private archive-before-delete repair of registered staging through the
existing encrypted journal, with typed CAREG003 evidence provenance and v1/v2 read
compatibility. Unknown stages remain blocked during ordinary cleanup. Repair verifies
vault, guards, recorded live state and pinned stage objects; it leaves the operation
pending until a separate restoration choice. No stage bytes become credentials.

CI verifies a complete/disjoint compiled workspace/native partition, retains every
inherited case and independent native process, bounds concurrency at two cases,
records timings, and tests omission/failure modes. All standard hosted platforms,
network-isolated Linux and blocking gates remain. No dependency or native API added.

Read `ca-04d-staging-recovery.md`, `ci-partition-review.md`, live AGENTS.md and the
affected coordinator/codec/native/CI definitions. Specification: SPEC-RECOVERY,
SPEC-TRANSACTION, F-014/F-020/F-022/F-024/F-025/F-026, S-003/S-005/S-008/S-009/S-015
and T-13/T-14/T-17/T-21/T-22/T-23/T-29/T-30/T-31. Use tools/spec_index.py show/check.

## Finish this packet

Inspect the complete diff and exact-head completed checks. Validate old record reads,
new kind rejection, encrypted fault/retry/capacity behavior, native exclusive handles,
all inherited forward/restoration interruptions and the repair interruption cases.
Validate both CI partitions and their independent failure gates. Record actual final
head/test merge/run and any remaining omissions. Owner review/merge is separate.

## Next eligible decision

After this review, address exact Q0 and remaining namespace/directory-metadata and
power-loss qualification before production effects. These require the narrowly scoped
owner evidence in `maintainers/open-asks.md`, not caller flags or fixture receipts.

A separately selected CA-05A **protocol preparation** packet can extend existing
`crates/runtime` bounded framing/hostile-input validation without production spawning
or an invented official schema. This is preparatory work, not permission to start
managed login/refresh or skip Q0/Q2. Keep one working roadmap and select a concrete
bounded deliverable before that work begins.

No live account operation, user-app force-close, automatic restoration, merge, force
push, release, permission change, new platform, runtime patch or renderer authority.
