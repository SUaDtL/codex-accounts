# Next safe slice: CA-03B

Parent packet: CA-03 / Q1. CA-02's read-only code was merged by the owner in
PR #2 as `7773d3472d13b7087179c0819a37fd5030a8f1c9`. CA-03A is implemented
in PR #3, now targeting main, and remains subject to its own review. Read live
refs, PR state, AGENTS.md and the working roadmap before choosing a new base.
Do not recreate the old stack, reset a branch or merge automatically.

## Pending CA-02 qualification and amendment

The merged discovery head `6ee778e9592d46132f253e2bee836b2650a44bd0` passed
run 35808957582. The reviewed correction to dependency acquisition is carried
forward in PR #3; it was not part of that CA-02 merge.

P02-A08's in-place T-07/provenance adoption remains open. The deterministic
renderer produces a candidate, but the normative source remains unchanged.
A review of the proposal-delivery PR is not silent adoption of a future source
edit. Apply the candidate and provenance migration in a clearly reviewed change
before claiming this checklist item closed. Current restrictive backend behavior
and all product operation gates remain unchanged. Actual Q0 resource, identity,
policy and lifecycle evidence is independently pending.

## CA-03B implementation scope

Review and select vetted AEAD, KDF, OS randomness and secret-buffer zeroization
implementations. Verify exact versions, licenses, build scripts, advisories and
transitive lock entries before acquisition. No homemade cryptography or guessed
pins. Implement context-separated encryption/fingerprint keys and versioned
envelopes with authenticated schema/profile/generation/resource metadata. Generate
nonces at the AEAD-required size using OS randomness, not resettable counters.

Add a narrow current-user Windows root-key protection adapter with synthetic
round-trip/failure tests. Never use machine-wide DPAPI or plaintext fallback.
Keep unsafe native bindings small, documented and tested; core stays safe.
macOS Keychain is separately scoped and qualified, not implied by Windows tests.

This sub-packet does not expose persistent account storage, capture/login,
credential replacement, a credential CLI, Tauri intents or production dispatch.
Persistent registry/generations and native path/ACL/durable-write protection belong
together in CA-03C. Do not write keys or secrets into an unqualified directory to
make a cryptography demonstration look like a working vault.

## Acceptance and evidence

Test envelope version/algorithm rejection, wrong key/AAD/context, truncation,
corruption, nonce/randomness failure, bounds, and debug/error/serialization canaries.
Native key tests use synthetic values on the selected OS. Another ordinary user's
inability to decrypt needs actual two-user evidence; a same-user round trip does
not prove it. Record unavailable native evidence without opening authority.

CA-03A inputs remain structural data, not a qualified Codex auth schema or verified
identity. Real integration waits for exact Q0 contract evidence and protected
persistent storage. Do not advance into CA-04 implicitly.

Run pinned formatting/tests/Clippy, Python/source/projection/provenance checks and
scoped native tests. Preserve the separate blocking dependency-review step before
network acquisition. Publish one normal review branch with an observed dependency
base, re-read before force:false updates, and inspect completed final-head CI.
No merge, force push, release, permission changes or live account mutation.
