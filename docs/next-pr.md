# Next packet: CA-03A

Parent work ID: CA-03 / Q1, encrypted vault, identities and generations.
This is a bounded first implementation slice, not completion of the whole vault.

## Scope

Implement a library-only, in-memory boundary for exact credential resources and
immutable generation selection. Enforce the specification's 1 MiB/resource,
4 MiB/set and 64-level JSON bounds; reject duplicate decoded object keys, malformed
or non-object JSON, invalid UTF-8, duplicate/missing/extra resources and unsupported
shapes. Preserve original bytes and explicit absence. Add fixed redacted errors,
composite identity comparisons and retention rules that preserve every unresolved
transaction reference. No email-only identity, mutable generation overwrite or
caller-provided qualification flag.

Use the existing reviewed Serde graph in a new safe library crate if useful.
Review any newly required dependency before adding an exact pin or lock entry.
Do not implement cryptography, random identifiers or protocol parsing from scratch
merely to avoid dependency review. Keep safe-core restrictions.

## Evidence and authority

Test with conspicuously synthetic resources only. A structural identity model is
not an extractor qualified for actual Codex credentials, and schema-valid JSON is
not a supported OAuth session. No filesystem input/output, native store access,
live capture, login, refresh, credential CLI, subprocess, UI or network operation
is in this slice. No public entry point can qualify an installation or enable the
existing product gates. Native resource/identity/policy facts remain pending Q0.

The full CA-03 still needs reviewed AEAD/KDF, random root/nonce generation,
current-user DPAPI / independent macOS Keychain protection, protected durable
registry/generation storage, and native permission/key-access tests. This first
slice is not an encrypted vault or consumer-ready release.

## Review and validation

Use a separate review branch. While CA-02 is unmerged, stack this PR on its exact
reviewed head and target that branch, clearly identifying the dependency. Re-read
both refs before publication; no merge, reset or force push. After owner merge,
retarget/reconcile through ordinary reviewed Git operations.

Run source/projection/provenance checks, Python tests, pinned Rust formatting,
compilation, unit tests and Clippy. Keep all current blocking checks. Include exact
boundaries, duplicate escaped-key/deep-container tests, unknown-field round trips,
absence combinations, same-email/different-principal cases, stale-generation
selection refusal, unresolved-reference retention and debug/error canaries.

Record actual final-head checks in the PR and concise validation state in the
working roadmap. Advance only the safe generic slice; no missing native evidence
may be replaced with a fixture, editable receipt or `qualified: true`.
