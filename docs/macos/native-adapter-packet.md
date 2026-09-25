# Independent macOS preparation: bounded native adapter packet

Checkpoint: 2026-09-25. Platform scope: macOS arm64 only. **No native implementation
or native execution is delivered by this packet. All macOS native checks are NOT RUN.**
This is a preparation handoff, not an official runtime schema, compatibility receipt,
signing approval, or authorization to access credentials. Windows evidence does not
establish macOS behavior or qualification. A hosted macOS compilation also does not.

## Shared contract and join barrier

The v2 source baseline pins CA-04D commit
`345533eacd4ed4ed288fa7a4580dd4e408106ede`, including the new private staging-repair
callback. `ca-04d-interface-review.md` explains the two changed shared definitions,
the new eighth file, preservation requirements and macOS exclusion mismatch.
`interface-baseline-ca04c.json` preserves the original seven-file record verbatim.
The checker verifies source and review provenance only, never native behavior.

| Existing boundary | Reuse obligation; no invented replacement |
| --- | --- |
| `core/src/lib.rs` | Separate credential, launch, identity and recovery observations. Accepted helper output does not confirm Desktop. |
| `platform/src/lib.rs` | macOS arm64 is a development lane; native qualification remains false. |
| `vault-crypto/src/keys.rs` | Preserve root identifier, purpose-bound encryption/fingerprints and redacted ownership. Existing protect/unprotect is Windows-only. Do not reinterpret CAKP algorithm 1 as Keychain. |
| `vault-storage/src/engine.rs` / `Files` | Preserve read/absent, stage, expected-parent publish, expected-content erase and bounded inventory semantics. A Unix rename is not an expected-old compare-and-swap. |
| `vault-storage/src/coordinator.rs` / `Effects` | Preserve guard, exact-resource staging/replacement, authenticated cleanup, newest-generation capture and explicit recovery ordering. No public Effects constructor. |
| `vault-storage/src/journal.rs` | Reuse the one write-ahead operation/restoration protocol; no second recovery engine. |
| `vault-storage/src/lifecycle_model.rs` | Preserve start identity and incomplete-observation refusal. Model booleans are observations, not native evidence. |

The interfaces are pinned for preparation, not permanently stable or approved for
macOS integration. PR #13 reconciles the CA-04D source delta in review. Native coding
still requires owner selection, review of CA-04D, exact SDK/binding review and an
authorized macOS test host. `stage_repair.rs` requires archive-before-delete and a
separate restoration choice; advisory flock alone cannot implement Windows sharing
exclusion. Drift still blocks source checks. No hash or passing test grants authority.

## Collected platform facts and their limits

The following are documentary observations, consulted September 25, 2026. The Apple
manuals are explicitly archived Mac OS X documentation despite the iPhoneOS URL
path. They identify questions and candidate semantics; revalidate them against the
**current SDK and selected macOS build** before introducing a binding. They are not
measurements from the owner's Mac.

| Area | Supported documentary fact | Consequence for the proposed adapter |
| --- | --- | --- |
| Exclusive creation and links | Apple's archived `open(2)` describes exclusive create and refusal of a final symbolic-link target with the relevant flags. Creation mode is affected by umask. [A1] | Verify every ancestor and retained object, not just the last component. Test inherited ACLs and descriptor inheritance independently. No claim that O_NOFOLLOW secures an entire path. |
| Home coordination | `flock(2)` is advisory; noncooperating processes can still access the file. Nonblocking contention reports failure. Duplicated descriptors share a lock. [A2] | A cooperating-switcher lock is not Windows sharing denial, writer quiescence or namespace exclusion. Unknown writer association continues to block mutation. |
| Flush and durability | The archived `fcntl(2)` description distinguishes F_FULLFSYNC's drive-flush request from ordinary fsync and notes storage limitations. [A3] | Do not extrapolate its old filesystem list into an APFS or physical-power-loss guarantee. File-data flush, directory metadata and device persistence need separate evidence. |
| Replacement | Archived `rename(2)` requires same-filesystem operands and replaces an existing destination. [A4] | Establish expected-old and namespace protection separately; no cross-volume copy/delete fallback. Its namespace guarantees do not prove the whole journal survives device power loss. |
| Owned-process exit | `waitpid` reports child status. WNOHANG can return zero; stopped is not exited. ECHILD is not a descendant-exit receipt. [A5] | Match retained ownership/start identity and distinguish pending, stopped, reaped and unknown. Never equate a successful signal with exit. |
| Process groups | `setsid` can create a new session and process group. [A6] | A session/process group is not a non-breakaway Windows Job Object. A descendant-escape test and an independently reviewed containment design are prerequisites. |
| Keychain scope | Apple's macOS section distinguishes default user, user-created and system keychains; the article warns that platform mechanisms differ. [A7] | Review exact keychain selection, access control, lock/denial/absence/duplicate handling and signing interaction. Do not import the article's iOS-specific mechanisms as universal macOS facts. |
| Signing identity | TN2206 describes designated requirements and subsystem-specific trust decisions. [A8] | Publisher verification, key access and launch eligibility need their own exact-build tests. A single signature-valid boolean cannot qualify the application. |

Current Security/AppKit documentation that required unavailable JavaScript content
was not treated as verified API semantics. Exact SecItem query flags, access-control
construction, noninteractive denial behavior and AppKit quit/launch bindings remain
binding-review items, not undocumented implementation choices.

## Next native subpacket: synthetic filesystem and home coordination only

**Start condition:** the shared join barrier above is reviewed, exact SDK/binding
sources and locked dependencies are approved, and a separate macOS arm64 test host
with an ordinary test user is selected. Owner review, not a manifest flag, selects
this execution. No installed Desktop, real home, credential store or account is an
input to these tests.

**Allowed implementation:** private macOS modules for newly created synthetic
fixture directories and fixed synthetic filenames; handle-relative creation,
identity/ownership inspection, expected-content checks, bounded cooperating lock
acquisition and failure mapping. Keep unsafe bindings private and minimal. The
integrator alone owns cfg/module wiring and any reviewed manifest edits. No changes
to Windows implementations, shared formats, public credential constructors, product
selectors or compatibility catalog. The native module path/layout is chosen in that
review; this packet does not pretend an empty module is an adapter.

**Deliverable:** actual native code and exact-name tests against its real operations,
not duplicate model implementations. Preserve the original failure plus cleanup
outcome. Retain owned descriptors; refuse unsupported volume/path conditions. No
unverified overwrite, symlink traversal, recursive directory copy or automatic
permission repair. Do not implement live target replacement until namespace and
expected-old semantics are demonstrated; a missing primitive must remain a refusal.

| Required case | Evidence needed before claiming that case passed |
| --- | --- |
| Ordinary-user protected creation | Inspect resulting owner, mode and ACL under restrictive and permissive umask; prove no preexisting object was modified. |
| Same-home contention | Two independent fixture processes; second bounded attempt refuses, later succeeds only after lock release. |
| Alias/ancestor substitution | Symlink, renamed ancestor, case/Unicode alias and nonregular object fixtures; retain expected object identity or refuse. |
| Hardlink and mount boundary | Detect extra links and cross-volume conditions; no fallback copying or deleting unknown objects. |
| Expected-old race | Change synthetic destination between observation and publication; preserve foreign bytes and report conflict. |
| Partial write and full-volume refusal | Exercise short writes, interrupted calls, ENOSPC and denied flush; failed publication must not become committed state. |
| Crash/reopen ordering | Terminate only owned fixture processes at durable boundaries; reopen using the same journal and distinguish process-crash from power-loss evidence. |
| Cleanup refusal | Leave unverified/changed staging registered and blocking; do not delete by name alone. Reconcile CA-04D archival contract before implementing repair. |
| Descriptor ownership | Check close-on-exec and inherited descriptor behavior in synthetic children; bound waits and close only owned handles. |
| Independent user denial | Separate authorized users and unchanged synthetic fixture; no account creation or ACL alteration by the runner. |

All ten rows are **NOT RUN**. Physical-power-loss testing remains a separate owner
qualification activity. Source tests cannot supply its missing evidence.

## Separate later packets, not permission granted here

Keychain work must use only a newly created, uniquely scoped synthetic key item or
dedicated test keychain after specific approval; never enumerate, unlock, export or
read the owner's existing items. Require positive and negative ordinary-user,
locked, unavailable, missing, duplicate, wrong-identity and two-user tests. Establish
format/migration semantics before replacing the Windows protected-key encoding.

Process work requires independent containment, graceful-quit refusal, PID reuse,
late/escaped descendants, parent-exit and incomplete-survey tests. No global process
kill, force-close of Desktop, guessed socket, official helper launch or login occurs
in this packet. Exact Desktop schema/resource/policy behavior, native consent,
packaging/signing and A-to-B-to-A account qualification remain separate later work.

## Evidence return contract

Return the reviewed source commit/tree, exact macOS/architecture/SDK/compiler and
binding revisions, filesystem/volume class, test name/profile, bounded result,
primary/cleanup outcomes and omissions. Keep native OS evidence distinct from
owner exact-build qualification. Export no home paths, user names, labels, keychain
item identifiers, credentials, fingerprints, login URLs or raw helper output.
Do not convert empty/skipped tests to passes. Windows results cannot close these
macOS cases; Linux and Intel macOS do not silently expand product scope.

## Documentary sources

- [A1: Apple archived open(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html)
- [A2: Apple archived flock(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html)
- [A3: Apple archived fcntl(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html)
- [A4: Apple archived rename(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/rename.2.html)
- [A5: Apple archived wait(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/wait.2.html)
- [A6: Apple archived setsid(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setsid.2.html)
- [A7: Apple Platform Security, Keychain data protection, macOS section](https://support.apple.com/guide/security/keychain-data-protection-secb0694df1a/web)
- [A8: Apple TN2206, macOS Code Signing In Depth](https://developer.apple.com/library/archive/technotes/tn2206/_index.html)
