# Next safe slice: CA-03B

Parent packet: CA-03 / Q1. The owner merged CA-02 read-only discovery in PR #2 as
`7773d3472d13b7087179c0819a37fd5030a8f1c9`. PR #3 targets main and delivers
CA-03A data/generation rules, the CI acquisition fix and the remaining in-place
T-07/provenance closeout. Read its actual head, review and merge state before
choosing a new base. Do not recreate an old stack, reset or merge automatically.

## CA-02 closeout and pending native evidence

T-07 and its v2 source manifest are now changed together in PR #3, with canonical/
visible agreement and exact original-source reconstruction. See
`spec-amendments/ca-02-t07.md`. The in-place amendment is submitted for owner review;
its metadata is not evidence of approval, Desktop compatibility or a valid account.
No future source-edit task is needed merely to apply this already checked-in
candidate. Do not waive the review or silently change the original provenance.

Q0's real publisher/runtime binding, effective home/policy, auth-resource/identity
and lifecycle contracts remain pending. All account-operation gates remain disabled.
Safe generic CA-03 development can proceed; real installation integration cannot.

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
