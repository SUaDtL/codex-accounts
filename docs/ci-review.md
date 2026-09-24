# CI/CD assurance and coverage review

Review scope: owner-requested audit after PR #7; repair PR #8. Baseline is merged
main `103d1a409ea19544ac4a18f33a19df925f5efc58`, tree
`50ad33a56c30b3a0bb6bbfde444c39f98457cc62`. Observed September 24, 2026.
This is a CI and test-evidence review, not an independent cryptographic audit,
line-by-line review of all native code, measured line/branch coverage, or release
qualification. The product specification is unchanged.

## Verified starting state

PR #7 merged the later head `bfe30a0eee174dfe107e5b7edc07d5c637408e28`, not the
unfinished `43902b9...` draft described in its stale conversation text. Main run
[35962095930](https://github.com/SUaDtL/codex-accounts/actions/runs/35962095930)
completed successfully. The source log, job 107512653814, records 78 Python passes,
43 requirements, 34 acceptance scenarios, zero qualified records and 58 reviewed
external dependency records. Windows job 107512653966 records 153 Rust passes,
including 64 storage cases. The 64 comprise 44 generic storage/coordinator cases
and 20 Windows cases, including two child-test entry points. Three separate
owner-run two-user helpers were ignored, not passed. All four baseline jobs passed.

These are actual merged-head results. Earlier failed dependency review was repaired
before this merge; it is not an outstanding main failure. Source inspection confirms
that coordinator tests are compiled and execute against independently retained
encrypted storage and synthetic external resources. A passing main run is not proof
that GitHub required the tests before merging.

Repository reads returned `protected: false`, required-check enforcement off and
an empty ruleset collection. No protection or permission setting was changed.
The tracked workflow directory contained only `ci.yml`; the preparation workflow
was already removed. There is no implemented application packaging/deployment
pipeline in that directory. The application directory is still a boundary document.

## Findings and disposition

| ID / priority | Finding | Disposition in PR #8 |
| --- | --- | --- |
| CI-01 / high | Main has no required-check enforcement. A failed or incomplete check does not prevent a merge. | Add a stable `CI gate` that assesses real prerequisite results. **Owner action still required** to make it a required check; code cannot configure protection by itself. |
| CI-02 / important | Source integrity/dependency checks share a step; the first failure hides the independent check and Python suite. | Separate checks, explicit non-cancelled conditions, and a final assessor that requires all source steps to execute successfully. |
| CI-03 / important | Matrix fail-fast is disabled, but Cargo still stops after a failing test executable. | Add `--no-fail-fast`. Failure remains a nonzero exit; other test executables may report their independent result. |
| CI-04 / important | `--all-targets` is not documentation testing; no optimized-target build was checked. | Add explicit `--doc` and optimized target compilation on every lane. Three meaningful API doctests cover public metadata and unavailable private dispatch. Optimized tests are compiled, **not executed** by this build step. |
| CI-05 / important | All branch pushes plus PR events generate duplicate runs; cancellation can leave confusing checks on a shared commit. | Push checks cover main only; PRs remain unfiltered. Manual and merge-group triggers remain separate. Only a superseded PR run is cancelled. Pre-PR feature pushes no longer automatically test; open a draft PR for review validation. |
| CI-06 / important | Native test code could silently compile out if the Windows runner architecture changes. | Explicit host/lane assessment requires Windows x64. macOS remains a portable-library lane, not Keychain or Desktop evidence. Runner labels still follow hosted image updates. |
| CI-07 / important | A future skipped prerequisite could make an aggregate badge misleading. | Require exact `success` for every required step outcome and job result. Missing, skipped, cancelled, failed, malformed or duplicated result fields fail. Test actual process exit and redaction as well as truth tables. |
| CI-08 / important | `--offline` was described as network-free validation, but it controls Cargo acquisition only. | Correct labels and document the distinction. **OS-level network denial and observed egress checks remain open**; no firewall changes or unreviewed runner sandbox are introduced. |
| CI-09 / maintenance | Checkout v4.2.2 produces a Node 20 deprecation warning under current runners. | Preserve its exact reviewed SHA. A current action/runtime upgrade needs a separate scoped source/runtime review; warning is not represented as fixed. |
| CI-10 / evidence | PR #7 and the working handoff still use pre-publication status text. | Record merged-head evidence, update the single roadmap/validation/next pointer and retain older results under their original source commits. |

The workflow and assessor are themselves repository code reviewed in the PR. The
fixed-layout checker is intentionally not a general YAML parser, malicious-PR
sandbox, or external policy authority. GitHub parses the YAML; mutation tests check
the reviewed event/command/job contract. A malicious editor who changes both tests
and their checks is outside that assurance. Protected review and trusted maintainers
remain necessary. A gate pass is never a Desktop qualification receipt.

## Coverage inventory: what the existing suite actually establishes

| Layer / relevant scenarios | Present evidence | Remaining gap |
| --- | --- | --- |
| Discovery and refusal: T-03/04/06/07/09/17 | Typed unknown/denied/conflict distinctions, override/path refusal and selected Windows package/file API tests. | Exact official publisher/runtime, effective Desktop home/backend/policy and build qualification are unobserved. No fixture authorizes a real installation. |
| Credential structure: T-08/11/12 | Exact bytes/unknown fields, explicit absence, strict JSON bounds/duplicate-key rejection, composite identity and stale-generation rules. | Structural identities are not official OAuth/Desktop identity; qualified companion-resource semantics remain unknown. |
| Cryptography: T-15/16/31 | Context-bound AEAD, corruption/wrong-key/version/randomness failures, independent synthetic vector, application buffer redaction/zeroization; real same-user DPAPI. | Two-user and unavailable-store trials remain not run; parser/private primitive state and same-user/admin exposure remain outside erasure/isolation claims. |
| Protected persistence: T-17/18/21/22 | Actual Windows DPAPI, ACL/link/ownership/locking/replace tests; encrypted registry, control repair and storage-restart fault tests. | Physical power-loss ordering, directory-metadata durability, representative sync providers, owner Windows 11 and independent macOS remain open. |
| Coordinator: T-08/13/14/18/20/22/23/27/29/30 | 22 synthetic journal tests exercise A0-to-A1-to-B-to-A1, B1 helper refresh before restoration, optional presence, unknown bytes, complete holds, cancellation and separate primary/restoration errors. Native tests reopen encrypted journal storage. | External credentials/helpers/launch are controlled memory effects. Canonical-home lock, real writer discovery, process start identity, normal quit and descendant exit proof belong to CA-04B. |
| Runtime: T-01/04/20/24/25/26/27 | Nine tests cover bounded byte framing, method names and redacted debug behavior. | No complete schema/duplicate-key/response-ID client, actual subprocess ownership, staged login, hostile executable protocol or egress trace yet. Method names and framing do not close protocol scenarios. |
| UI and diagnostics: T-01/02/05/10/28/31/32/33 | Restrictive source/API boundaries; this PR adds compile-fail checks for private coordinator and absent public dispatch. | No Tauri shell, IPC/preview/consent tests, full canary propagation, accessible installed UI or owner real A-to-B-to-A trial. |
| Release: T-34 and S-015/S-016 | Pinned compiler, exact dependency graph/manifest checks, locked Cargo acquisition; optimized compilation added. | No full build-network isolation, regularly refreshed advisory scan, built SBOM/provenance bundle, two clean unsigned-payload comparison, installer/signing/manual-upgrade/uninstall evidence. |

All complete T-01 through T-34 remain open. The table maps partial evidence, not a
percentage of product completion. Native child entry points and ignored helpers
must not inflate qualification totals. No line/branch coverage percentage, fuzzing
campaign, mutation score or complete Rust test-executable inventory was measured.
Future measurement should identify unexecuted decision paths and missing assertions,
not introduce an arbitrary percentage as a substitute for scenario closure.

## Owner merge gate

After `CI gate` has a successful recent PR-triggered run, configure a branch rule or
ruleset targeting main: require a pull request, require `CI gate` from GitHub Actions,
and require current/up-to-date checks. Review whether the rule also applies to
administrators; avoid a blanket bypass that defeats the required checks. Choose
approval requirements compatible with the actual reviewer arrangement; this review
does not invent a second maintainer. Keep force pushes and branch deletion disabled.
Do not require the workflow title `Repository checks` instead of the job `CI gate`.

Validate the setting read-only in repository settings, then use a controlled review
PR to confirm a failing gate blocks merge. Do not deliberately break main or bypass
a protection to test it. No setting change is performed by PR #8. Manual-dispatch
success is diagnostic evidence, not a substitute for a required PR event result.
Merge-group trigger support does not enable a merge queue or assert its availability.

## CI/CD operating contract

Every PR runs source/Python and all three Rust lanes; no README path filter can hide
a required gate. Main pushes validate the merged tree. Manual execution is available
for diagnostic reruns. Successful acquisition precedes locked Cargo-offline tests;
independent checks still report after formatter/test failures. Each job records the
actual checked-out commit/tree, and Rust records its compiler. The aggregate fails
unless all required jobs actually succeed. The retained timeouts bound job duration;
a cancellation or timeout is incomplete evidence, never a passing result.

Read the exact final head and GitHub test-merge SHA when handing off. Retry only a
supported transient acquisition/runner failure; fix reproducible code/assertion
failures. Do not rerun until a flaky test becomes green and call that closure. Preserve
failed evidence and repair the test or implementation. Never upload authentication
fixtures, vaults, paths, keys or raw runtime output as CI artifacts.

No CD is added now. Packaging, signing, SBOM/provenance, reproducibility and installed
acceptance belong to CA-07 after the operational product exists. No self-updater,
release publication or deployment is implied by an optimized build. The next product
packet remains CA-04B; CI-08/09 and unclosed native evidence remain explicit backlog,
not reasons to recreate already-merged CA-04A.

## Verification and rollback

Run each prerequisite command individually; stop acquisition if review fails:

```console
python tools/check_dependencies.py
python tools/ci_contract.py
cargo fetch --locked
python tools/spec_index.py check
python -m unittest discover -s tests/python -v
cargo fmt --all -- --check
cargo test --workspace --all-targets --no-fail-fast --locked --offline
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Expected documentation tests: one public metadata example and two compile-fail
private-dispatch examples. No native test on an unsupported host is reported passed.
Final PR #8 evidence is bound to its actual final head in the PR and validation record;
baseline results above are not transferred. Local container/Python execution failed
before commands started during this review; hosted results are reported separately.

Rollback is an owner-reviewed follow-up/revert of CI-only changes, preserving product
and vault data. A required-check rename/removal also requires a deliberate matching
settings update; do not disable all checks to resolve an expected-name mismatch.

## Primary references and repository evidence

- [Baseline workflow](https://github.com/SUaDtL/codex-accounts/blob/103d1a409ea19544ac4a18f33a19df925f5efc58/.github/workflows/ci.yml), [main run](https://github.com/SUaDtL/codex-accounts/actions/runs/35962095930), [merged PR #7](https://github.com/SUaDtL/codex-accounts/pull/7).
- Repository `crates/vault-storage/src/{engine,coordinator,journal,switch_tests}.rs`, `tests/python/test_storage_boundary.py`, and `docs/ca-04a-journal.md`: selective source tracing of compiled tests, graph/holds and recovery assertions. This is not an exhaustive native-code audit.
- [Cargo test options](https://doc.rust-lang.org/cargo/commands/cargo-test.html): no-fail-fast, target selection, separate documentation tests, locked/offline acquisition semantics.
- [Required checks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks): actual head/test-merge results, skipped checks and eligible events.
- [Protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches): required checks, expected app source, current-branch policy and administrative bypass behavior.
- [Workflow events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows): separate merge-group checks. Public vendor references inform CI semantics; they do not amend the product specification.
