# CA-04B native and CI input review

## Scoped native change

Retain windows-sys 0.61.2 and all 58 existing external package/version/checksum
records. Add only ToolHelp, JobObjects, WindowsAndMessaging, Gdi and LibraryLoader
features. The first three enable bounded process snapshot, owned job and normal
window-close primitives. Gdi and LibraryLoader support the SDK window structure and
module handle used by synthetic native window tests. No package version, Cargo.lock
entry, build script or external native library is added.

Publisher source at microsoft/windows-rs commit
`d468916ac27a36fb8a12bafc1bf5c0ec2fe92238` declares windows-sys 0.61.2.
The inspected generated ToolHelp and JobObjects declarations match the selected
structure layout, handle ownership and feature boundaries. Prior checksum-bound
archive review remains in ca-03c-dependencies.json. This scoped review is not a new
full upstream audit or a current advisory scan.

Authoritative API references:
- https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexexw
- https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes
- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- https://learn.microsoft.com/en-us/windows/win32/toolhelp/taking-a-snapshot-and-viewing-processes

Mutex ownership is thread-bound and recursive. The private Rust guard must be
non-Send/non-Sync and reject a second same-process acquisition. Existing named
mutex security must be checked, not assumed from the requested descriptor.
Process exit time is undefined while running; it is read only after the retained
process handle is signalled. Snapshot PPIDs do not establish a complete birth
history, process ownership or shared Codex home. An owned non-breakaway job covers
ordinary inherited process creation, not arbitrary broker/WMI-created children.
Exact Desktop/runtime qualification must establish any such exclusions before a
production adapter can rely on those observations.

## Source evidence artifact

The GitHub-owned upload action is pinned to v7.0.1 commit
`043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`. Its action.yml uses Node 24.
Reviewed action inputs and src/upload/upload-artifact.ts: the supplied directory
is uploaded; missing files fail; overwrite is false; retention is seven days.
Only a git archive of the committed public source and a bounded digest record are
included. Untracked test directories, runtime vaults, credential buffers, environment
values and dependency caches are not packaged. The action's artifact service and
bundled dependencies remain trusted; this is a scoped action review, not a full
JavaScript dependency audit. The immutable action/source pin is not security proof.

An artifact from a failed run remains source evidence only. It does not override
failed checks, approve dependencies, qualify an installation or become a release.
Source capture does not modify the checked-out source. The inherited checkout pin
is unchanged at this preparation stage; its runtime warning is not claimed fixed.
