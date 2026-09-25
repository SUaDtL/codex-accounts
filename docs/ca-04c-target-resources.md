# CA-04C: synthetic Windows target resources

## Scope and authority

This increment connects real Windows target-file effects to the existing CA-04A
journal and CA-03C encrypted storage. It reuses CA-04B canonical-home mutexes,
retained directory identity and owned-job exit proof. The only target constructor
and Effects implementation are compiled into tests. There is no production switch
entry point, real home/resource discovery, login, refresh client or Desktop launch.
The compatibility catalog remains empty. All complete requirements and T-01 through
T-34 release scenarios remain open.

The fixture declares slots, not arbitrary paths: `synthetic-resource-NN.bin` in a
fresh protected synthetic directory. It distinguishes absence, empty resources and
exact nonempty bytes, including unknown JSON fields, line endings and opaque binary
bytes. Reads use zeroizing buffers, one MiB per resource and four MiB per snapshot.
The synthetic owner/SYSTEM DACL is not asserted to be the correct real Desktop ACL.
No existing user directory, permission, account or application is changed.

## Transaction and recovery

The coordinator persists staging/apply/restore intent before native effects and
completion only after verified observations. The outgoing live generation is
captured into the encrypted vault before the first target replacement. An owned
synthetic helper can update the installed target; its job and original process
handle must prove exit before the newest target generation is captured. That newer
generation remains reachable after restoration of the source. The helper does not
call a provider, accept credentials, or establish Desktop identity.

Native staging is same-directory CREATE_NEW with protected permissions from
creation, bounded exact writes, file flush and readback. Replacement retains the
expected-old handle, rechecks its name binding and content, uses FileRenameInfoEx,
and verifies the installed source object's identity, permissions and bytes.
Absent targets use no-clobber publication. Removal uses a compared DELETE handle,
not pathname deletion. No truncate-live, cross-volume, copy/delete or plaintext
backup fallback exists. No-op resources still undergo expected-current comparison.

Cleanup now receives authenticated source, original-target and newest-target
CredentialSets from the encrypted journal references. Registration includes both
staging and restoration intent. A matching filename is insufficient: the native
adapter validates every candidate's registration, shape, permissions and allowed
bytes before deletion, then rechecks each pinned candidate. Foreign, over-limit,
linked, inaccessible or changed data is preserved and blocks the operation.
Failures retain primary and restoration errors separately; a committed target is
not rolled back merely because cleanup or launch failed.

A process can die immediately after creating a stage but before writing its full
contents. Such an unverified stage is not silently deleted, adopted as a generation
or overwritten. The restart tests require a visible conflict and unchanged recorded
live resources in those cases. CA-04D adds a separately invoked encrypted staging-evidence repair path; see
`ca-04d-staging-recovery.md`. This does not relax normal cleanup or qualify production
integration. Missing ownership/path/live-state evidence still blocks repair.

## Native evidence

Twelve exact-name behaviors run through `tools/run_native_tests.py` in debug and
release, in addition to every inherited CA-04B case. Missing, duplicated, ignored,
failed or zero filtered cases are rejected. Two child entry points are harnesses,
not additional behavior proofs.

| Behavior group | Evidence exercised |
| --- | --- |
| Exact resources | All nine absent/empty/nonempty single-resource transitions; exact binary/JSON preservation; invalid slots and oversize refusal. |
| Concurrency and paths | Expected-old mismatch, changed staging, an open writer, hardlinks, home replacement and duplicate lock acquisition, foreign namespace entries and changed leaf DACL. |
| Journal integration | Newest outgoing capture, separate installed/launch outcomes, stale selected generation, external conflict, partial installation/reopen, separately retained primary/restoration failures. |
| Cleanup | Authenticated content and registered-mask enforcement; cleanup sharing failure blocks until verified retry without replaying a launch. |
| Helper | Suspended test child assigned to the existing private owned job before resume; actual exit verification; newest helper-written target preserved after source restoration. |
| Process interruption | Seventeen native forward boundaries and seventeen native restoration boundaries, each in a separately created child and real vault/home. Parent reopens the protected key and encrypted records. Three torn-stage cases initially refuse normal recovery; CA-04D additionally tests explicit repair. |

The interruption inventory includes before/after secure stage creation, content
write, file flush, rename/publication, post-publication file flush and handle-bound
deletion/close. Existing encrypted-store/coordinator fault tests remain intact.
The tests do not interrupt a storage-controller write, remove power, or establish
cache/namespace persistence after an OS crash.

## Filesystem limitations requiring review

File flush, namespace publication and physical-power-loss durability are different
claims. Only the first two are exercised here. A qualified directory-metadata
persistence primitive and physical-power-loss evidence remain unavailable;
F-012/T-21/T-22 are not closed.

POSIX replacement requires delete sharing on the retained old target. It therefore
is not an atomic namespace compare-and-swap and cannot force an uncooperative
writer to obey the home mutex. Immediate handle/name/content checks detect observed
changes, not every possible mutation in the final check-to-publication interval.
Exact shared-home writer association and lifecycle qualification remain mandatory
before live integration. See the native API review for the separate directory-share
constraint and its test-only treatment.

## Safe reproduction

Use a reviewed checkout on Windows x64 with the pinned Rust 1.90.0 toolchain.
Acquire only the reviewed dependencies, then run Cargo offline:

```powershell
python tools/check_dependencies.py
cargo fetch --locked
cargo fmt --all -- --check
cargo test --workspace --all-targets --no-fail-fast --locked --offline
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
cargo test --workspace --all-targets --release --no-fail-fast --locked --offline
python tools/run_native_tests.py
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Full source/Python and IP-network-isolation evidence runs in the inherited Linux
hosted jobs. CA-04D corrected the Unicode and mocked /proc fixtures. The POSIX-only FIFO test
still reports an explicit Windows skip; hosted Linux runs it.

The native suite creates unique protected test parents and owns its child handles.
Do not supply real credentials or change an existing ACL to make a case pass. On
normal completion the parent removes only its own synthetic directories. A crash of
the outer test runner can leave synthetic fixtures; treat cleanup separately and
never redirect the harness at a real home. Child environments retain the minimal
Windows path variables, including SYSTEMDRIVE, but do not inherit account tokens.

Export only fixed case names, pass/fail, non-secret counts, build/commit identifiers
and the aggregate outcome. Raw helper output, plaintext stages, vault contents,
profile identifiers, owner paths and local logs are not PR/release evidence. CI
retains committed source, not a working-directory or user-home archive.

## Remaining gates

Review this implementation and exact-head hosted evidence before merge. Production
constructor/consent, exact Q0 resource/home/policy/process binding, uncooperative
namespace-writer handling, production staging-repair integration, ordinary-user/two-user and
unavailable-store protection, representative sync providers, namespace durability,
physical power loss and independent macOS evidence remain separate requirements.
CA-05 official-runtime construction and stdio must not reuse a fixture as authority.
