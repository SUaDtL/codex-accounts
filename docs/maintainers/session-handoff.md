# Session handoff: PR #13 integration and CA-05B

## Live-state checkpoint

PR #12 is owner-merged on main at `d64d3e31712798b1b790c70c3830c6853a67c1a2`.
PR #13 retains CA-04D from `345533eacd4ed4ed288fa7a4580dd4e408106ede`, integrates
that main, and adds CA-05B. Reuse `feat/ca-04d-staging-recovery-ci`; inspect its
actual head/reviews/checks. `../next-pr.md` owns routing and
`../implementation-plan.html` is the only working roadmap.

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

## Joined preparation and new runtime slice

PR #12's CA-05A framing, CA-06A synthetic HTML/tests and macOS documentary packet
are retained. `preparation-handoff.md` is historical evidence for that review.
CA-05B adds `runtime/src/json_object.rs` and adversarial tests, reusing the unchanged
strict parser in the in-memory vault crate. Raw framing remains available and
untrusted; the new object wrapper proves syntax only. See `../ca-05b-json-objects.md`.
The scoped dependency review preserves all external versions and prior review bytes.

The macOS v2 baseline adds the private staging-repair callback and retains its
predecessor plus `../macos/ca-04d-interface-review.md`. It is source consistency,
not permission for native coding or proof of macOS exclusion/Keychain behavior.

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

CA-05A is merged; CA-05B syntax validation is in this review. Request/handshake
correlation can be selected next; operational runtime/login still needs Q0/Q2.
Use `parallel-work.md` only for explicit separate assignments with one integrator,
disjoint write ownership, separate roots and final combined-head validation.

GitHub: rediscover the named write/read actions, preserve base trees, re-read the
review branch before a non-force advance and verify the full changed-file set.
Do not merge, reset/force-push, publish a release, change permissions or touch accounts.
Export only sanitized outcomes. Never request or commit real authentication material.
