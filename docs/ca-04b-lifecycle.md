# CA-04B: native home locking and process lifetime evidence

Private Windows x64 primitives and controlled native tests in PR #9. This packet
implements bounded slices of F-009/F-010/F-011/F-014/F-024 and T-06/T-14/T-17/T-19/
T-20. No complete acceptance scenario or exact Desktop adapter is qualified.

## Implemented boundaries

| Primitive | Established behavior | Still not established |
| --- | --- | --- |
| HomeLock | Retained local NTFS ancestor handles, current-owner leaf, file identity and security checks; owner/file-identity-derived global mutex; contention and lifetime release; Unicode aliases share the lock. | Effective Desktop home, user consent, cooperation by unrelated processes or durability of later auth writes. |
| ObservedProcess | Read-only query/synchronize handles, PID plus native creation identity, owner/image rechecks, signalled-handle exit observations, bounded normal WM_CLOSE requests. | Publisher qualification, home association or permission to close an arbitrary user application. |
| ObservedTree | Bounded snapshots and retained descendants with same-owner birth/lifetime checks; incomplete observations remain explicit. | Every missed intermediate/broker child, or a complete shared-home writer set. |
| OwnedFamily | New noninherited, non-breakaway, bounded job assigned to a suspended test child before it runs; graceful and post-termination waits use real job/handle evidence. | Official-runtime launch, private stdio, managed policy or recoverable ownership across a switcher crash. |

The vault-root lock and target-home lock are different. Home locking uses current
owner plus native volume/file identity, not an unnormalized string or stale marker.
Its thread-bound mutex owner cannot be sent/shared across threads. Same-process
recursive acquisition is refused. Existing mutex ownership/DACLs are checked.
Retained ancestors reject replacement rather than repairing arbitrary permissions.
No existing home is created or modified by acquiring this metadata-only primitive.

Process observation does not request termination rights. Normal quit is bounded by
30 seconds and must be followed by actual process/descendant checks; a WM_CLOSE
receipt is not exit. Tests use only their own hidden windows and original creator
handles for cleanup. No test enumerates names and kills a Desktop, terminal or IDE.

Owned helpers are separate from observed user processes. Only a cfg(test)
constructor currently creates OwnedFamily. Its five-second graceful budget precedes
any owned-job termination, followed by five more seconds for exit proof. Root exit,
zero active-job count and all retained handle signals are jointly required.
Incomplete member surveys remain sticky metadata, never absence. A query error may
remain pending within that same budget, but failed job accounting/changed limits
or a nonempty family cannot become success. The kernel job, not a caller boolean,
binds containment. Broker/WMI behavior remains a qualification exclusion.

All public vault and coordinator restrictions remain. No synthetic Effects adapter
is selectable outside tests. Shared-home readiness always returns incomplete or
qualification-missing; no success value can be constructed by a developer override,
renderer flag, serialized report or a process-name match. No live auth read/write,
login, launch, RPC, UI, inference, account creation or global permission change is
introduced. The registry, envelope and recovery formats are unchanged.

## Native validation on standard hosted Windows

Prerequisites: exact reviewed checkout, Python 3.11+, Windows x64 with ordinary
local NTFS test storage, pinned Rust 1.90.0 and reviewed dependency acquisition.
No Codex installation or actual account is required. Hosted Server 2025 evidence
is not a Windows 11 owner-machine or ordinary-user security qualification.

Run each PowerShell command individually. Stop on failure; never count zero tests.

```powershell
python tools/check_dependencies.py
if ($LASTEXITCODE -ne 0) { throw 'Dependency review failed' }
cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw 'Dependency acquisition failed' }
python tools/run_native_tests.py
if ($LASTEXITCODE -ne 0) { throw 'Named native execution failed' }
```

Expected: **22 passed JSON records**, eleven fixed behaviors in debug and release.
The exact-name verifier rejects missing, duplicated, ignored, truncated and failed
results. The separate child-test entry point is not counted as a behavior. Each
case runs once per profile in a new process; this is not a retry-until-green loop.
All cases also remain in ordinary workspace debug/release execution. The verifier
reports only fixed error categories and source line numbers, never raw child output.

| Behavior | Required observation |
| --- | --- |
| Unicode/alias and recursive contention | Same native home is busy; independent home is not; guard drop releases. |
| Child contention and interrupted owner | Contention exists while owner runs; actual exit releases kernel ownership. |
| Replacement/non-directory hazards | Held root/parent cannot be renamed; file/hardlink/traversal/network candidates refuse. |
| Foreign owner and junction hazards | System-owned candidate and synthetic root/ancestor junction refuse; junction target unchanged. |
| Start identity and process exit | Stale start identity refuses; timeout is not exit; held handle later signals. |
| Normal close and unsaved-work refusal | Own window receives normal close; accepted close exits, refused close remains alive. |
| Late descendant and final write | Parent exit does not complete family; last synthetic write occurs before family exit. |
| Owned timeout and unrelated child | Full graceful budget precedes owned-job termination; unrelated child remains alive. |
| Snapshot/descendant readiness | Known child remains tracked after parent exit; neither observation nor exit qualifies the home. |

Stale/reused-PID identity is deterministically simulated through changed creation
identity; this does not claim an actual OS PID reuse was forced. Unknown owner,
invalid lifetime, incomplete enumeration and missed-parent limitations also have
pure-model tests. Positive exact Desktop/home association is deliberately absent.

The hardlink test creates its hazard before taking restrictive directory handles.
It does not weaken the lock to make fixture setup succeed. Sync-root inventory
uses stack-owned, balanced WinRT apartments and uncached factories, with the
existing 100-query regression plus concurrent fresh-thread exit coverage. COM
teardown no longer runs under the Windows TLS loader lock. See the source review
for the API contracts and observed-versus-inferred failure distinction.

## Hosted CI and its limits

The normal Ubuntu/Windows/macOS matrix executes formatting, debug tests, doctests,
optimized compilation, optimized tests and Clippy with warnings blocking. A named
Windows gate proves actual CA-04B execution rather than a portable compile-out.
The independent standard Ubuntu lane starts with an empty target directory inside
a new network namespace, drops privilege/capabilities, verifies loopback-only
interfaces and unavailable numeric IPv4/IPv6 routes, then runs debug/release tests,
doctests and Clippy. Dependencies are acquired before isolation. Isolation failure
never falls back to the connected host. This is direct-IP isolation, not a complete
hostile-build/IPC sandbox or Windows/macOS network isolation.

Only exact committed public source and its digest record are retained for seven
days. A failed run can retain source but cannot be represented as passing evidence.
No vault, fixture keys, encrypted recovery evidence or raw helper transcript is
uploaded. A source archive is not an SBOM, signed provenance or consumer release.

## Owner evidence, cleanup and rollback

Run the same checks under an existing ordinary Windows 11 user and share only the
source commit, OS/build, case names, profile, executed count and pass/fail. Keep paths,
SIDs, process command lines, account IDs, authentication files, keys and raw output
out of reports. Tests create unique marked directories and their own children;
cleanup touches only those objects. Do not remove unrelated files after a failure.

For the actual installation, run the read-only discovery steps in
`q0-qualification.md`. Expected sanitized output continues to show mutation disabled
and explicit unknown contract fields. The exact publisher/runtime/home/backend/
policy/resource/writer/quit/launch contract still needs owner-reviewed evidence;
never provide a real auth file. Ordinary two-user protection uses the existing
procedure in `ca-03b-crypto.md`, not new accounts created by this packet. Sync-provider,
unavailable-store, physical-power-loss/directory-metadata and macOS evidence remain
open. Native synthetic passes do not close those obligations.

No live-state rollback is needed: this packet never performs a live account
operation. Code correction uses a normal reviewed follow-up or owner-reviewed
revert, not force pushing. CA-04C is the proposed subsequent target-resource adapter
slice; official launch/login and production integration remain separately gated.
