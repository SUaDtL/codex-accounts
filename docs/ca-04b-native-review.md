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
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi/nf-jobapi-isprocessinjob
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject

Mutex ownership is thread-bound and recursive. The private Rust guard is
non-Send/non-Sync and rejects a second same-process acquisition. Existing named
mutex security is checked, not assumed from the requested descriptor.
Process exit time is read only after the retained process handle is signalled.
Snapshot PPIDs do not establish complete birth history or a shared Codex home.
An owned non-breakaway job covers ordinary inherited process creation, not arbitrary
broker/WMI-created children. Exact runtime qualification must establish those
exclusions before production integration can rely on these observations.

Member PID surveys may race with exit or return access denied. A survey failure
remains sticky incomplete metadata and may remain pending only inside the fixed
owned-job wait budget. It is never converted into absence. Only a signalled root,
all retained process handles signalled, and a separately successful zero-active-job
query establish family exit. Job identity/limit/accounting failures remain blocking.
The five-second graceful and five-second post-termination budgets are unchanged;
only this private owned job can be terminated. There is no production constructor.

## Apartment lifetime correction

The inherited sync-root reader kept its RoInitialize guard in thread-local storage.
Rust documents that Windows holds the loader lock during TLS destructors; Microsoft
prohibits COM initialization/uninitialization from DllMain and describes runtime
uninitialization as unloading DLLs and closing RPC. Moving the guard to the ordinary
query stack removes that teardown hazard. All interfaces drop before the guard;
each successful initialization, including S_FALSE, remains balanced on the same
thread. The guard cannot cross threads. The registered factory remains uncached;
no cloud-path check is removed or replaced with an assumption of absence.

Native regression tests retain the existing 100-query test under explicit batch
ownership and add concurrent fresh-thread query/exit cycles. A Python source guard
rejects reintroducing the TLS destructor. The earlier STATUS_STACK_BUFFER_OVERRUN
exit is observed failure evidence, not by itself a proven stack or root-cause trace.
Final-head native execution is required to validate the corrected path.

Primary lifetime contracts inspected September 24, 2026:
- https://doc.rust-lang.org/std/thread/struct.LocalKey.html#synchronization-in-thread-local-destructors
- https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-roinitialize
- https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-rouninitialize
- https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-couninitialize

## Reviewed Actions and source evidence

Checkout uses GitHub's v7.0.1 commit
`3d3c42e5aac5ba805825da76410c181273ba90b1`. Inspected the tag/release,
`action.yml` (Node 24), and `src/git-source-provider.ts` at that exact revision.
The triggering repository/ref and `persist-credentials: false` remain. The provider
removes authentication in its finally path. No write token or checkout override
is added. This is not an independent audit of its JavaScript dependency closure.

Upload-artifact is pinned to v7.0.1 commit
`043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`; its action.yml uses Node 24.
Reviewed inputs and src/upload/upload-artifact.ts: missing files fail, overwrite is
false, retention is seven days. Only an exact git archive of committed public source
and a bounded digest record are included. No untracked test directories, runtime
vaults, credentials, environment values or dependency caches are packaged.

Official source inputs:
- https://github.com/actions/checkout/blob/3d3c42e5aac5ba805825da76410c181273ba90b1/action.yml
- https://github.com/actions/checkout/blob/3d3c42e5aac5ba805825da76410c181273ba90b1/src/git-source-provider.ts
- https://github.com/actions/upload-artifact/blob/043fb46d1a93c77aae656e7c1c64a875d1fc6a0a/action.yml
- https://github.com/actions/upload-artifact/blob/043fb46d1a93c77aae656e7c1c64a875d1fc6a0a/src/upload/upload-artifact.ts

An artifact from a failed run is source evidence only, not test success, dependency
approval, qualification or a release. The artifact service and bundled dependencies
remain trusted. Source capture does not modify the checked-out source.

## Standard-runner assurance additions

Linux isolation uses installed util-linux tools. Elevated execution is limited to
creating a child network namespace and dropping privileges with setpriv. Every
Python/Cargo/build/test child runs under the original nonzero UID with zero
capability sets and no-new-privileges. Preflight verifies a different namespace,
actual identity/capabilities, loopback-only interface topology and both numeric
IPv4/IPv6 route refusals. UDP connect sends no packet. IPv6 EADDRNOTAVAIL is accepted
only with the independent loopback-only topology; it cannot pass the IPv4 probe.

Dependencies are reviewed/acquired first. A fresh target directory forces actual
compilation inside the disconnected namespace. No global firewall/interface/runner
configuration changes and no connected-host fallback are used. Sources:
- https://man7.org/linux/man-pages/man1/unshare.1.html
- https://man7.org/linux/man-pages/man1/setpriv.1.html
- https://docs.github.com/en/actions/reference/runners/github-hosted-runners

This blocks direct IP networking, not filesystem/Unix-domain IPC access or every
hostile-build escape. It does not establish Windows/macOS network isolation or
runtime T-02. Hosted Windows runs as an administrative identity with UAC disabled,
not an ordinary-owner isolation qualification.

Optimized workspace tests execute on all three matrix platforms. The named Windows
verifier independently requires all eleven CA-04B behavior cases in debug/release;
zero, missing, duplicate, ignored or failed cases refuse. Raw child output remains
in a temporary local file; only fixed error categories and source line numbers are
reported. The child-test entry is not a native behavior or qualification receipt.
