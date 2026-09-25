# CI quality and efficiency: CA-04D

## Change and equivalence

The previous Windows workflow ran every native case inside the debug/release
workspace suites and then reran the same cases individually. CA-04D makes the
workspace/native sets disjoint without removing either proof or any platform lane.

`ci_workspace_tests.py` asks the compiled whole workspace for its unfiltered test
inventory, then requests the proposed filtered inventory. It requires the difference
to be exactly the reviewed native case set, each present once. New, missing, duplicate,
substring-colliding or unexpectedly omitted names fail closed. It runs the verified
complement with Cargo `--no-fail-fast`, `--all-targets`, `--locked` and `--offline`.
Other operating systems still run the entire workspace; Windows ARM cannot claim
Windows x64 native proof.

`run_native_tests.py` retains an independent process for every named case. Each
profile prepares its test binary once and uses at most two case workers, each with
unique owned homes, vaults and children. Costly restart cases start first. Cases are
not retried; failed preparation cannot run a stale binary, and a failed/aborted case
does not suppress other case results. All 27 case names must execute successfully in
each profile. Existing per-case timeout and output bounds remain.

The workflow gate still requires both partitions, doctests, optimized compilation,
Clippy, source/projection/provenance/dependency checks, all three OS lanes and the
independent Linux network-isolated rebuild. The standard hosted runner types,
reviewed action/toolchain pins, read-only token and source-only artifact boundary
remain unchanged. No executable/result cache, path-based job omission, nightly-only
safety test, `continue-on-error`, or privilege/protection change is introduced.

## Quality checks

New regression tests reject missing/extra/duplicated inventories, accidental substring
skips, zero tests, stale test binaries, malformed output, failed commands and raw-output
leakage. They verify fixed commands, disjoint coverage, Windows architecture refusal,
independent native failures and the two-worker bound. The old assertions and negative
workflow/gate tests remain active.

Two existing Python tests are corrected rather than skipped: the Unicode filename
now actually contains valid Unicode instead of a control character, and the mocked
Linux isolation orchestration test no longer reads the executing host's `/proc` data.
This does not convert a mocked test into native network-isolation evidence. The
separate real Linux isolation job remains mandatory.

## Timing evidence

Both runners emit elapsed time with their sanitized result records. Compare complete
job/step durations for the actual source and tested merge, not an earlier green head
or a compilation-only run. Native case/profile records describe execution time, not
coverage percentages or product qualification.

Baseline: PR #11 source `b51e460cbd2a9d82bd24f3344d4f9d83c06c4d64`,
[run 36117748297](https://github.com/SUaDtL/codex-accounts/actions/runs/36117748297),
Windows job `108015976629`. Record this PR's final completed run and measured
comparison in its validation handoff. A single before/after hosted run is an observed
comparison, not a controlled hardware benchmark or a future runtime guarantee.

## Vendor contracts

[Cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html) documents
`--no-fail-fast`, `--no-run`, target selection and libtest argument forwarding.
[The Rust test harness](https://doc.rust-lang.org/rustc/tests/) documents listing and
filter/skip options. This implementation does not assume a skip expression is safe:
the pinned toolchain's actual compiled before/after inventories must prove it.
