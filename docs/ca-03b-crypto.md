# CA-03B: encrypted envelopes and current-user root-key protection

This library-only slice implements S-006/S-007 primitives and portions of
T-15/T-16/T-31. It is not a persistent vault or a qualified Desktop adapter.
No account file, credential store belonging to Codex, process, login, network,
UI or production operation gate is accessed. macOS Keychain is not implemented.

## Cryptographic contract

`RootKey::generate` obtains a 256-bit root and an independent opaque 128-bit key ID
from `getrandom` OS randomness. There is no production raw-key constructor,
custom entropy provider or resettable nonce counter. Failure returns no key or
envelope and clears the partially filled destination. Defensive all-zero root/ID
rejection is not a statistical test of the OS random generator.

HKDF-SHA256 derives separate 256-bit encryption and HMAC fingerprint keys, using
the random key ID as salt and distinct fixed application/version/purpose labels.
DPAPI inner-record integrity uses a third distinct KDF label. The implementation
uses reviewed RustCrypto primitives, not a local cipher, KDF or MAC algorithm.
XChaCha20-Poly1305 uses a fresh OS-random 24-byte nonce for every seal operation.
The extended nonce variant is the implementation choice, not a new spec mandate.

The version-1 `CAEN` wire format is fixed: four-byte magic, one-byte version,
one-byte algorithm (1 = XChaCha20-Poly1305), 16-byte key ID, 24-byte nonce,
four-byte little-endian plaintext length, ciphertext, 16-byte tag. Overhead is
66 bytes. Bounds are 1 MiB for one credential-resource payload and 4 MiB for
other envelope payloads; overflow, truncation, trailing bytes, unknown algorithms
and versions are rejected. No migration, fallback or algorithm negotiation exists.
The generic envelope bound does not authorize expanding the credential-set bound.

Authenticated data is a fixed application/version domain, the entire 50-byte
header and the expected context: purpose, schema version, opaque profile/namespace
ID, generation ID and resource slot, encoded at fixed widths. Context is not
stored in the plaintext header. Identity metadata and credential bytes belong in
the encrypted payload, not in these identifiers or diagnostics. Registry/journal
callers must use an independently selected vault namespace and record generation;
this slice does not define their durable registry or bootstrap selection protocol.
Resource slots retain the structural 16-slot bound from CA-03A; non-resource
purposes require slot zero. Unknown fields in payload bytes are not reconstructed.

The caller supplies context from the independently selected record. Never decode
untrusted context and accept it as the expected value. Wrong purpose, schema,
profile, generation, resource, key ID, key, nonce, ciphertext or tag fails closed.
An authentic envelope is not proof of freshness: deliberately selecting an old
context still opens its matching old ciphertext. CA-03C/coordinator storage must
preserve current generation selection and handle rollback/recovery separately.

HMAC-SHA256 fingerprints bind the same context, explicit presence, length and
bytes. Absence and present-empty differ. Verification uses the MAC implementation's
constant-time comparison. Fingerprints are opaque in-memory evidence, have fixed
redacted Debug output and no serialization or diagnostic export interface.

## Native boundary and secret lifetime

Only `src/dpapi.rs` permits application-authored unsafe code. Neighboring crypto
modules forbid it; the crate otherwise denies it and the existing safe core,
platform and runtime restrictions remain unchanged. The native API surface is
CryptProtectData, CryptUnprotectData and LocalFree from the existing exact
windows-sys pin, available only on Windows x64.

DPAPI always uses current-user scope with UI forbidden and null deprecated prompt
parameters. Machine-wide protection and plaintext fallback are absent. The public
`CAKP` wrapper declares version and algorithm and caps input at 4096 bytes. A
successful native call is followed by fixed inner shape and HMAC integrity checks
before constructing a root key. This additional check handles damaged recovered
bytes; it is not a replacement for DPAPI's user protection. Parsing a wrapper does
not establish who created it or qualify its eventual storage location.

Native input lengths are bounded before conversion. Borrowed input buffers remain
alive throughout synchronous documented read-only calls. Output begins empty;
an RAII owner immediately takes the OS allocation and releases it on every exit.
The initialized output bytes are zeroized before LocalFree; no OS pointer escapes.
The OS allocator contract is trusted. No unbounded native output is copied into
Rust; null, failed and oversized results return fixed error categories.

Root/derived keys, decrypted buffers, native returned allocations and temporary
root packets use zeroization. CA-03A resource buffers, identity strings and
application-owned decoded JSON keys now use the reviewed zeroize implementation,
including rejection paths. The AEAD implementation wipes its stored key state.
Borrowed caller input, Serde's private scratch, HKDF/HMAC internal state, copies
created by moves/allocators, registers, swap and crash dumps remain outside a
secure-erasure claim. There is no whole-memory, SSD or backup erasure guarantee.

## Executable checks

Use the repository's Rust 1.90.0 and Python 3.11+ prerequisites. Run each acquisition
command only after the preceding command succeeds; PowerShell does not universally
stop a pasted group on a native command's failure.

```powershell
python tools/check_dependencies.py
if ($LASTEXITCODE -ne 0) { throw 'Dependency review failed' }
cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw 'Dependency acquisition failed' }
cargo test -p codex-accounts-vault-crypto --locked --offline
```

Ordinary tests cover exact bytes, every context field, all single-byte mutations,
truncation/extension, bounds, version/algorithm rejection, wrong key, partial
randomness failure, absent/empty fingerprints and redacted errors. A fixed
synthetic envelope vector was independently generated with Python HKDF-SHA256 and
libsodium XChaCha20-Poly1305, not by the Rust implementation under test. Windows
adds real DPAPI roundtrip, corruption, inner-shape/integrity and refusal tests.
No selected Desktop installation is required. Hosted native tests are OS API
evidence only; refer to the PR's actual final-head result and validation.md.

## T-16 two-user evidence: explicitly not closed by ordinary CI

Three ignored test cases in `tests/native_users.rs` are owner-run evidence helpers,
not a shipped credential CLI. They accept no key or account input. They create
only a fresh random test key, protect it with DPAPI, and encrypt a fixed synthetic
message. The sole file is `ca03b-synthetic.dpapi`, created without overwrite in the
crypto crate's test working directory (`crates/vault-crypto`). Never substitute a
real wrapped vault key or authentication file. No elevated session is required.

Prerequisites: two already-existing ordinary Windows x64 user sessions on the
same machine, the same reviewed source/build and a permitted way to copy this
synthetic fixture byte-for-byte. Do not create OS accounts, alter permissions,
bypass policy, or run these tests as SYSTEM/admin to manufacture evidence. When
these prerequisites are unavailable, record NOT RUN and keep T-16 open.

Under the first user, from the repository root:

```powershell
cargo test -p codex-accounts-vault-crypto --test native_users native_two_user_1_create_fixture --locked --offline -- --ignored --exact
cargo test -p codex-accounts-vault-crypto --test native_users native_two_user_2_creator_control --locked --offline -- --ignored --exact
(Get-FileHash .\crates\vault-crypto\ca03b-synthetic.dpapi -Algorithm SHA256).Hash
```

Both tests must pass. Retain the fixture hash privately. Copy only that synthetic
fixture to the second user's same test working directory using the approved local
transfer mechanism. Confirm its SHA-256 is identical, then under the second user:

```powershell
cargo test -p codex-accounts-vault-crypto --test native_users native_two_user_3_other_user_refuses --locked --offline -- --ignored --exact
```

The test must pass because DPAPI key recovery is refused after successful fixture
reading and parsing, not because the file is missing or inaccessible. Back under
the creator, verify the unchanged hash and repeat `native_two_user_2_creator_control`.
A corrupt fixture or a caller assertion about the user is not native isolation
proof. Record the actual distinct-user observation privately without publishing
OS usernames, paths, credentials, fixture contents or raw native errors.

A shareable result states source commit, OS/build, first-user control before/after,
byte-identical fixture confirmation and the other-user refusal test outcome.
Delete only the identified synthetic fixture copies after evidence collection.
Creation refuses an existing filename rather than deleting/replacing it.
This proves only the tested ordinary-user case. Locked/unavailable user protection,
enterprise recovery and roaming behavior require separate target-environment
qualification; administrators and same-user malware are not excluded by DPAPI.

## Dependencies and remaining work

See `ca-03b-dependencies.json` and `dependency-policy.md` for exact pins, hashes,
build-script observations and the advisory snapshot. Preparation downloads were
source inspection only. Tests and Clippy build offline after reviewed acquisition.
The review is scoped, not a full independent cryptographic or supply-chain audit.

CA-03C must implement protected local storage, encrypted registry and immutable
credential generations with a recoverable commit protocol, before any persistent
account storage is exposed. Random profile UUID generation, independent expected
context bootstrap, native ACL/path/durability checks and retention remain there.
Key loss requires official reauthentication, never an emergency plaintext key,
backdoor or unchecked migration. No active account mutation or CA-04 advancement
is licensed by this slice. macOS Keychain and real Q0 evidence remain separate.

Primary implementation references:
- https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/
- https://docs.rs/hkdf/0.12.4/hkdf/
- https://docs.rs/hmac/0.12.1/hmac/
- https://docs.rs/getrandom/0.3.4/getrandom/
- https://docs.rs/zeroize/1.8.2/zeroize/
- https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
- https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata
