# CA-05A: schema-independent protocol preparation

This packet extends only `crates/runtime`. It adds no dependencies, adapter link,
process constructor, stdio ownership, environment handling, filesystem, credential
access, network, login or refresh operation. It does not invent or collect an
official runtime schema. Operational CA-05 still depends on Q0 and Q2.

## Implemented transport contract

LF delimits a raw UTF-8 payload. CR is retained as untrusted payload, not silently
normalized. Per-frame 1 MiB, aggregate queued-plus-partial 4 MiB and 32 queued-frame
limits remain. Draining a frame returns aggregate and queue credit. Any framing
error discards undelivered frames and poisons the decoder; debug output is redacted.

Successful EOF is now terminal. Repeated `finish()` is idempotent and already queued
frames may be drained. Any subsequent `feed`, including an empty chunk, returns
`Closed`, discards undelivered frames and poisons the decoder. Previously a decoder
could accept fresh input after successful EOF. Tests cover that lifecycle defect.

`ClientMessageKind` and `Method::classify_client` classify only outbound shape:
`initialized` is a notification and the five other existing allowlisted names are
requests. This is local policy vocabulary, not a schema, parameter validator,
handshake state machine, server-request dispatcher or permission to call a method.
Exact matching refuses whitespace, case changes, control suffixes, lookalikes,
encoded separators and unqualified method extensions. Existing allowlist entries
are retained; login-related enum names do not make login executable.

## Executable test surface

All nine inherited unit tests are retained. Fourteen integration tests add every
split of multibyte streams, byte-at-a-time input, deterministic chunk schedules,
EOF lifecycle, credit reuse, combined bounds, failure invalidation, hostile UTF-8,
byte/character boundaries, all outbound-kind combinations and method-confusion cases.
JSON-shaped malformed/duplicate-key/privileged payloads are deliberately tested as
**UntrustedFrame only**. A frame boundary never asserts JSON or protocol validity.

```text
cargo test -p codex-accounts-runtime --locked --offline
cargo test -p codex-accounts-runtime --release --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

The existing debug/release workspace and isolated Linux lanes discover the new
integration test executable. No workflow filter or skipped-test exception is added.
These tests partially support S-011/S-012 and T-25/T-26; they do not complete them.

## Deferred validation is explicit

Duplicate JSON-key rejection, malformed JSON validation, schema/depth/value bounds,
handshake sequencing, response/login-ID correlation, notification reordering,
unexpected privileged server requests, deadlines, independent stderr draining,
approved executable/environment selection and owned-helper exit are not implemented
by this framing packet. They require reviewed parser/qualified-schema and lifecycle
integration. Do not wire UntrustedFrame into command execution, credential decisions,
or a UI status as a substitute. A new schema must come from the reviewed exact
executable during later authorized qualification, not a hand-authored fixture.
