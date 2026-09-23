# Validation and evidence boundary

## CA-02 discovery increment

Read-only discovery implementation is ready for code review. Q0 native contract
qualification is not complete. P02-A08's in-place T-07/provenance adoption is also
still pending: the deterministic review candidate is implemented and tested, but
the normative file remains unchanged. Do not mark the original checklist wholly
closed or treat a discovery report as account-operation permission.

The resumption review found a Windows CI acquisition-boundary defect: dependency
review and Cargo fetch shared one PowerShell step. A later successful native
command could mask the review exit code and acquisition could run after failure.
They now have separate blocking steps; independent tests and Clippy explicitly
require successful review and acquisition. Five regression tests reject combining
those commands, ignoring review failure, dropping the review prerequisite, or
making failures nonblocking. All five passed locally; full-suite results for the
published repair head must be taken from that head's completed CI, not an older run.

| Evidence | Observed result and scope |
| --- | --- |
| Reviewed CA-02 head | `6ee778e9592d46132f253e2bee836b2650a44bd0`, PR #2, before the acquisition-boundary repair. |
| Completed head-bound CI | Run [35808957582](https://github.com/SUaDtL/codex-accounts/actions/runs/35808957582): all four jobs succeeded on that head. |
| Windows job log | Job 107015928251: 43 Rust tests passed, none ignored/filtered; 13 core, 16 discovery, two CLI, three platform, nine runtime. Nine discovery tests exercised actual Windows API/native-reader behavior. Formatting and Clippy with `-D warnings` also passed. |
| Actual hosted OS | Windows Server 2025 x64 build 26100, not a Windows 11 Desktop qualification. |
| Dependency review | Existing 32-package locked review is unchanged; no new dependency in this repair. See dependency-policy.md and ca-02-dependencies.json. |
| Earlier local completion checks | 48 Python tests passed, including 13 projection/provenance/plan tests. Spec check passed 43 requirements, 34 acceptance IDs and zero qualified records. These are recorded earlier results, not new local execution. |
| Local resumption checks | Five new dependency-acquisition contract tests passed. Local Rust tools remain unavailable; no local compilation or native execution is claimed. |
| Final-head CI | The PR handoff records the actual repair SHA and completed run IDs after publication. Earlier green heads are not transferred to later commits. |
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
| P02-A07 | Default/debug/error canaries; redirected local-display refusal and subprocess regression tests. |
| P02-A08 | PARTIAL: deterministic T-07 canonical/projection candidate and provenance migration have regression checks. In-place adoption remains pending; the original normative source is unchanged. The proposal-only delivery variation is documented in spec-amendments/ca-02-t07.md. |
| P02-A09 | Product mutation gates remain disabled; all-positive synthetic inputs and catalog files cannot enable them. Real catalog is empty. |
| P02-A10 | Head-bound hosted logs inspected as recorded above; every later repair needs its own completed checks. Safe owner collection and exact pending observations are in q0-qualification.md. |
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
