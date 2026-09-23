# CA-03A: bounded in-memory vault data

This is the first CA-03/Q1 sub-packet. It is a library-only implementation, not
an encrypted vault, persistent credential store, or authorized account operation.
CA-02's read-only discovery code is merged. PR #3 targets main; its actual source
and final-head CI remain independently reviewable. No merge is implied here.

## Implemented

`crates/vault` keeps exact owned resource bytes and explicit absence. Structural
set rules reject duplicate/missing/extra slots and required absence. JSON-object
resources are parsed through Serde visitors, not a last-key-wins map: decoded
object keys must be unique at every depth, including escaped equivalents.
UTF-8, JSON syntax, trailing content and the root object shape are validated.
The original bytes are never normalized or re-serialized.

Limits are 1 MiB per resource, 4 MiB per set and 64 object/array containers of
nesting, including the root. The generic slice additionally caps 16 resource
slots and 128 in-memory generation-index entries; exhaustion refuses growth,
never evicts recovery state. These are conservative implementation bounds, not
claims that the normative spec names these additional counts.

Composite identity compares issuer, subject and workspace. Email and labels are
not identity keys. Identity input is bounded and rejects empty/control/ambiguous
surrounding whitespace; it is not cryptographic verification or a Codex extractor.
Distinct profile/generation IDs are opaque nonzero inputs, not self-generated
UUIDs or filesystem names. A reviewed random UUID provider remains future work.

The generation index accepts a new immutable ID only for its current parent and
matching profile/composite identity. Stale parent, reused ID and mismatches leave
the latest pointer unchanged. Retention computes unreferenced candidates while
protecting the latest and every supplied unresolved reference; an unknown reference
rejects the whole decision. It does not prune files or prove that the caller has
supplied every durable journal reference.

## Deliberate limits

This data crate has no filesystem, native key store, subprocess, network, UI, CLI, live capture,
login, refresh, encryption, storage migration or credential replacement. Structural
rules are not qualified auth-mode/schema rules. All production gates and the empty
compatibility catalog are untouched. No selected Desktop build is qualified.

Secret-containing types have fixed redacted Debug output and no Clone/Serialize
implementation. Errors are fixed categories without input or parser messages.
CA-03B replaces ordinary buffer filling with `zeroize` for owned resource buffers,
identity strings and decoded object-key owners, including rejection paths. Borrowed
caller input, Serde private scratch, copies from moves/allocators, registers, swap
and crash dumps remain outside an erasure guarantee. See ca-03b-crypto.md for the
scoped secret-lifetime review; zeroization is not whole-memory protection.

In-memory parent checks are not durable atomicity or cross-process locking. A
future storage/journal transaction must make encrypted generation persistence and
registry advancement recoverable together. Do not wire this slice to live accounts.

## Dependencies and validation

CA-03A reused `serde = 1.0.229` and `serde_json = 1.0.145` without adding an
external package. CA-03B adds `zeroize = 1.8.2` to this data crate; its separate
crypto crate and transitive graph are recorded in ca-03b-dependencies.json.
The original 32 entries remain unchanged within the now 58-package review.
Review and acquisition are separate blocking CI steps, including on Windows;
formatting, tests and Clippy remain blocking. Rust is executed on hosted runners,
not claimed as locally executed when the preparation host lacks the toolchain.

Primary API basis: https://serde.rs/impl-deserializer.html and
https://docs.rs/serde/latest/serde/de/trait.MapAccess.html. These document the
visitor/seed interface, not application security. The wrapper implements the
application's depth and duplicate-key policy and keeps Serde's own limit enabled.

`crates/vault/tests/data.rs` covers bounds, malformed input, decoded duplicate
keys, byte/absence preservation, identity, stale-generation rejection, retention
and debug/error canaries. `data_edges.rs` adds decoded surrogate-pair duplicates,
malformed surrogates, object depth, rejected-append non-consumption, shuffled slots,
explicit optional absence and UTF-8 byte-based identity limits. All fixtures are
synthetic. Inspect PR #3 and validation.md for actual head-bound execution results;
a test definition or an older green head is not a current pass.

Compilation/model tests do not close full T-08/T-11/T-12/T-13/T-18/T-31 scenarios.

## Remaining CA-03

CA-03B is implemented for review in PR #5: context-bound envelopes, separated keys,
OS randomness, zeroization and Windows x64 DPAPI. Native two-user/unavailable-store
qualification remains pending; macOS Keychain is not implemented.
CA-03C: protected local storage, encrypted registry/generations, durable commits,
retention/deletion and native permission/key-access tests. Real resource-specific
integration waits for Q0 identity/resource/policy evidence. No CA-04 advancement
is implied by passing this generic first slice.
