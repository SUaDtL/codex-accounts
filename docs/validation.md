# Validation and evidence boundary

## CA-03A data increment

PR #3 is a separate, dependent review of `crates/vault`: in-memory resource
validation and generation metadata only. Encryption, native key protection,
persistent storage and full CA-03/Q1 acceptance remain unimplemented.

Local checks: 50 Python tests passed (48 inherited plus two static library-boundary
cases); the specification/projection/provenance checks passed. Local Rust tools
remain unavailable. Hosted run 35809705379 for
`6a6e7a329c5cb73a3d160b068c0096a76223f2a5` compiled and passed all tests and
Clippy on Ubuntu/Windows/macOS, including 18 new Rust data cases, but failed
formatting in the test file. Commit `543118c5c06e9b794c8bb1fbd3e85410109dfd5f` applies the exact pinned-formatter
diff and passed replacement run 35810179693. The final-head run must be checked
after the last documentation commit; see PR #3's current
validation section for the actual SHA/run result. An older partial/green result
is not transferred to a later SHA.

No external package/version/checksum was added; the existing 32-entry dependency
review is unchanged. The internal vault lock entry was accepted by `--locked`
hosted Cargo. Safe-core lints, always-refuse operation gates and empty catalog
remain unchanged. No credential bytes were collected from an installed app.

## CA-02 discovery increment

Source inspected: `39a74ab8f4f31e6b7d3c1fde4fe21d74850449d4`, PR #2.
The completion commit adds deterministic specification/provenance checks, roadmap
adoption, native boundary/procedure documentation and terminal-only local display.
These changes do not enable credentials or qualify an installation.

| Evidence | Observed result and scope |
| --- | --- |
| Prior CA-02 CI | Run [35794749871](https://github.com/SUaDtL/codex-accounts/actions/runs/35794749871), head `39a74ab...`: all four jobs succeeded. |
| Windows job log | Job 106971327239: 42 Rust tests passed, no ignored/filtered tests; 13 core, 16 discovery, one CLI, three platform, nine runtime. Nine discovery tests exercised actual Windows API/native-reader behavior. |
| Actual hosted OS | Windows Server 2025 x64 build 26100, not a Windows 11 Desktop qualification. |
| Dependency review | Existing 32-package locked review remains unchanged; no new dependency in CA-02 closeout. See dependency-policy.md and ca-02-dependencies.json. |
| Local completion checks | 48 Python tests passed, including 13 projection/provenance/plan tests. Spec check passed 43 requirements, 34 acceptance IDs and zero qualified records. |
| Local Rust toolchain | NOT AVAILABLE in this completion environment; do not claim local compilation or native execution. The changed CLI test needs the final-head hosted run. |
| CA-02 completed-head CI | Head `6ee778e9592d46132f253e2bee836b2650a44bd0`, run [35808957582](https://github.com/SUaDtL/codex-accounts/actions/runs/35808957582): all four jobs succeeded. Inspected Windows job 107015928251 records 43 Rust passes, including nine native API cases and the redirected-output regression. Python suite: 48 passes. |
| Real installation / Q0 contract | NOT RUN; official publisher/runtime role, effective home/policy, auth-resource/identity and lifecycle contracts remain unresolved. |
| Full product/release acceptance | NOT COMPLETE. No OAuth, account handoff, encrypted vault, Tauri UI, release package, signing or reproducibility qualification. |

## CA-02 checklist disposition

| Packet item | Code/evidence disposition |
| --- | --- |
| P02-A01 | Concrete current-user registered-package reader; pure no/single/multiple selection cases; OS inventory or explicit policy-refusal path. No installed Codex fixture is required. |
| P02-A02 | Identity disagreement and detached-runtime tests; package-relative safe reading and re-observation. No positive official binding exists for any publisher or architecture. Cached signature acceptance is never official identity. |
| P02-A03 / A04 | Distinct declared/configuration observations and always-conservative effective context. Missing layer/default/policy evidence remains unknown, never inferred from the diagnostic shell. |
| P02-A05 | Native synthetic ACL, junction, hardlink, sharing and replacement tests ran on hosted Windows. Complete later vault/write-path protection remains out of scope. |
| P02-A06 | Native open-path instrumentation and exclusive synthetic auth fixture prove the selected read scope; source review confirms no production subprocess/store/control API. Real installation actor-attributed network/filesystem tracing remains a named native gate, not falsely closed by these tests. |
| P02-A07 | Default/debug/error canaries; completion adds redirected-local-display refusal and a subprocess regression test. |
| P02-A08 | T-07 review candidate changes canonical and visible rows together, and its proposed provenance migration is tested. Adoption into the normative file remains pending; original full source can be reconstructed byte-for-byte; every modeled section and source note is projection-checked. Product version and 43/34 topology retained. |
| P02-A09 | Product mutation gates remain disabled; all-positive synthetic inputs and catalog files cannot enable them. Real catalog is empty. |
| P02-A10 | Prior hosted logs inspected; final-head checks must be completed before handoff. Safe owner collection and exact pending observations are in q0-qualification.md. |
| P02-A11 / A12 | Declared read-only code/test/doc scope only; no main write, force push, merge or release. One working roadmap and a separately bounded CA-03A next packet. |

The review milestone is discovery code with ordinary/native API tests, not Q0
contract qualification. Full release scenarios remain open where later integration
or actual installed-app evidence is required. No artifact or boolean grants authority.

## Preserved foundation evidence

PR #1 merged at `54f01f3a77936523d8202ad37d8a46c5fe32a4c6`. Its tested head
`9b9328f474322c9de48dc6a8db332ee8105899dd` passed PR run 35761967137 and push run
35761962743: 35 Python tests and 19 Rust model tests, formatting and Clippy across
Ubuntu/Windows/macOS. Those remain historical bootstrap results, not later-head
validation or product acceptance. Earlier unavailable local Rust checks were not
retroactively relabeled as locally executed.
