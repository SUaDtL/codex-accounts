# Next safe slice: CA-03B

Parent packet: CA-03 / Q1. CA-03A is implemented on PR #3, stacked on CA-02
PR #2; neither merge nor full Q1 closure is implied. Read live refs, both PRs,
AGENTS.md and the current roadmap before selecting the next base.

## Pending CA-02 closure

The native discovery code and completion checks pass at
`6ee778e9592d46132f253e2bee836b2650a44bd0` (run 35808957582). P02-A08's
in-place T-07/provenance adoption is still pending: the tested renderer produces
a candidate, while the normative source is unchanged. Do not mark the entire
original CA-02 checklist complete. Review/apply the candidate and its provenance
migration together before claiming that item closed. The code's restrictive
backend behavior is unchanged. Real Q0 resource/identity/policy/lifecycle evidence
remains independently pending.

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

This sub-packet does not yet expose persistent account storage, capture/login,
credential replacement, a credential CLI, Tauri intents or production dispatch.
Persistent registry/generations and native path/ACL/durable-write protection belong
together in CA-03C. Do not write keys or secrets into an unqualified directory to
make a cryptography demonstration look like a working vault.

## Acceptance and evidence

Test envelope version/algorithm rejection, wrong key/AAD/context, truncation,
corruption, nonce/randomness failure, bounds, and debug/error/serialization canaries.
Native key tests must use synthetic values on the selected OS. Another ordinary
user's inability to decrypt needs actual two-user evidence; a same-user round trip
does not prove it. Record unavailable native evidence without opening authority.

Existing CA-03A inputs remain structural data only, not a qualified Codex auth
schema or verified identity. Production integration waits for exact Q0 contract
evidence and the protected persistent vault. Do not advance into CA-04 implicitly.

Run the current pinned formatting/tests/Clippy, Python/source/projection/provenance
checks and scoped native tests. Keep independent checks blocking. Publish one
normal review branch with an explicit dependency base, re-read before force:false
updates, and verify completed CI for the actual final head. No merge, force push,
release, permission changes or live account mutation.
