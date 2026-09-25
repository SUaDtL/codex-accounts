# Session handoff: CA-04D review

## Live-state checkpoint

CA-04C / PR #11 is owner-merged at `455baa2eeee614586b26d419ee961475fe1ad23b`.
Current branch: `feat/ca-04d-staging-recovery-ci`. Inspect its actual open PR, current
head, reviews and completed checks before editing. Do not reuse the merged branch
or reset to a bootstrap snapshot. `../next-pr.md` owns packet routing and
`../implementation-plan.html` remains the only working roadmap.

## Existing implementation and this increment

The discovery, protected vault/immutable generations, encrypted storage, write-ahead
coordinator, CA-04B home/process ownership and CA-04C synthetic target adapter remain
in place. Reuse them rather than rebuilding foundations. CA-04D adds:

- `crates/vault-storage/src/stage_repair.rs`: private explicit repair, guard/live-state
  validation, encrypted evidence callback and pending restoration boundary.
- `journal.rs`, `journal_codec.rs`, `codec.rs`: typed Live/Staging evidence in CAREG003;
  old record read compatibility and unchanged key/envelope/generation formats.
- `native/lifecycle/target.rs` and repair/restart tests: exclusive stage handles,
  archive-before-delete, sharing/path refusal and actual child-process interruption.
- `tools/ci_workspace_tests.py`, `run_native_tests.py`: compiled inventory partition,
  independent exact-name native processes, bounded concurrency and measured duration.

All effects construction remains synthetic/test-only. No installation is qualified;
no live capture/login/switch, official helper launch, renderer authority or release
is enabled. See `../ca-04d-staging-recovery.md` and `../ci-partition-review.md`.

## Validation

Use the pinned Rust 1.90.0 toolchain and reviewed locked dependencies. Run dependency
review before acquisition, then offline formatting, both workspace partitions, both
native profiles, doctests, optimized compilation and Clippy. Run source/Python,
specification/model/projection/provenance and workflow-contract checks. Commands and
expected output are in the CA-04D procedure; do not count a workspace complement as
complete without the matching native verifier.

The native inventory is 27 distinct named behaviors per profile, including all CA-04B
and CA-04C cases and four CA-04D cases. Child entry points are harnesses. Three owner-
run two-user helpers remain NOT RUN. The Linux isolated job is actual direct-IP
isolation evidence; portable macOS compilation is not Keychain/Desktop qualification.

`../validation.md` preserves historical exact-source evidence. The current PR records
its final head/test merge/tree/run and inspected logs. No old green badge transfers
to a new SHA. Do not remove tests, relax lints, fabricate receipts or add error masking.

## Next action and remaining inputs

Complete owner review of CA-04D after current-head checks. Exact Q0, ordinary-user/
two-user/unavailable-store, representative sync/path, namespace/directory-metadata,
physical-power-loss and independent macOS evidence remain in `open-asks.md`.
Successful explicit stage repair is not complete Q2 or a consumer release.

CA-05A schema-independent protocol preparation is separately selectable; operational
runtime/login still needs Q0/Q2. Do not infer that a future adapter is approved.
Use `parallel-work.md` only for explicit separate assignments with one integrator,
disjoint write ownership, separate roots and final combined-head validation.

GitHub: rediscover the named write/read actions, preserve base trees, re-read the
review branch before a non-force advance and verify the full changed-file set.
Do not merge, reset/force-push, publish a release, change permissions or touch accounts.
Export only sanitized outcomes. Never request or commit real authentication material.
