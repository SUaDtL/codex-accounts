# CA-04C native API and dependency review

This is an implementation review record, not independent certification or Desktop
qualification. No dependency, Cargo feature, lockfile, envelope or journal format
changes. The existing reviewed 58-package dependency closure is retained.

## Windows contracts used

| Boundary | Contract and decision |
| --- | --- |
| Creation and reading | Reuse the existing Win32 wrappers, protected Security descriptor, OPEN_REPARSE_POINT/OPEN_NO_RECALL/WRITE_THROUGH flags and object/ACL checks. Target plaintext reads are bounded and zeroizing, unlike ciphertext storage buffers. |
| Present-target publication | SetFileInformationByHandle with FileRenameInfoEx and REPLACE_IF_EXISTS (0x1) plus POSIX_SEMANTICS (0x2). An aligned, bounded SDK buffer names only the same synthetic directory. No unsupported-API fallback. |
| Absent-target publication | Flags zero: a newly appearing destination is not overwritten. An AlreadyExists result is classified as external change. |
| Deletion | FileDispositionInfo on a retained compared DELETE handle. ACL and object checks precede disposition; file flush and observed absence are separate checks. |
| Helper ownership | Existing CA-04B OwnedFamily; new suspended synthetic test process, private noninherited job, assignment before resume, actual exit/accounting before generation capture. No discovered-process termination. |

Microsoft primary references:

- [CreateFileW sharing and access](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
- [FILE_RENAME_INFO layout and information classes](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info)
- [FILE_RENAME_INFORMATION flags and retained-handle semantics](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
- [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
- [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)

## Two different sharing constraints

A retained old file handle without FILE_SHARE_DELETE prevents POSIX replacement.
With delete sharing, replacement is possible but the handle does not exclude a
foreign namespace replacement. The implementation keeps in-place write denial,
rechecks expected bytes and the live-name object before publication, and verifies
the resulting source object. It explicitly does not claim an atomic namespace CAS.
Real writer quiescence/association remains a prerequisite, not a boolean supplied by
tests. F-014's uncooperative-writer limitation is not removed.

Separately, Windows opens a rename destination's directory for write. The existing
CA-04B read-only directory-sharing mode correctly denies that operation. Its public-
to-the-private-module acquire path is unchanged. A new cfg(test)-only constructor
permits write sharing on the synthetic leaf directory, still denies leaf deletion,
and retains write/delete denial on ancestors. It uses the same canonical object
identity, current-owner checks, thread-bound lifetime and kernel mutex. There is no
production entry to this mode. A directory-relative Win32 rename was not adopted as
an unqualified workaround. This fixture mode is not a protection policy for real
Desktop homes; metadata/namespace races remain an integration review obligation.

## Review focus

All unsafe calls stay under the existing private native boundary; the coordinator
and codecs still forbid unsafe code. No generic path/process dispatcher is added.
Only test files contain child creation, exit injection or synthetic ACL changes.
No global process kill, existing permission repair, plaintext backup, arbitrary
companion discovery or relaxed validation is used.

Staging cleanup authority now comes from authenticated generation references and
both forward/restoration registration masks. Incomplete stages can remain blocked;
that is not an implemented encrypted staging-repair workflow. File sync does not
certify directory-metadata persistence or physical-power-loss ordering. Full Q0/Q1/Q2
and all complete acceptance scenarios remain open.

The Windows matrix stays on the standard hosted runner. Its bounded job budget is
30 minutes (other Rust lanes remain 15) to retain debug/release workspace execution
and both-profile exact native proofs including real process restarts. No check,
case, failure gate or runner-isolation boundary is removed.
