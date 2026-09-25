# CA-05B: strict JSON-object transport preparation

## Scope and API

PR #13 retains CA-04D and integrates owner-merged PR #12. This bounded increment
adds `JsonObjectDecoder` and `JsonObjectFrame` to `crates/runtime`, using the unchanged
`codex_accounts_vault::validate_json_object` parser. No second parser, official
schema, credential access, process construction, login or refresh is introduced.

`feed` only frames incoming bytes. Call `next_object` repeatedly to validate each
complete queued object before receiving its exact original bytes. `finish` closes
input; it does not certify queued JSON or indicate RPC success. Complete frames may
still drain after EOF. A missing frame is None; poisoned input always returns an
error, never an apparently clean EOF. On syntax failure, undelivered queued frames
and partial data are discarded. Objects returned before that failure cannot be
revoked; callers still need independent schema/ID/lifecycle validation.

Non-object roots (including batch arrays), malformed JSON, trailing content,
decoded duplicate keys at any nesting level and depth above 64 refuse. The root
object counts as a container. The existing parser's numeric range limitations
remain; valid JSON grammar outside its accepted numeric range is unsupported,
not truncated or normalized. CR, whitespace, key order, unknown fields and Unicode
bytes survive accepted frames unchanged. No methods or identity fields are inferred.

The same 1 MiB frame, 4 MiB queued-plus-partial and 32 queued-frame limits apply.
Caller-held drained objects are not part of that queue budget. Partial, queued and
drained raw/object frame owners now use exact pinned zeroize 1.8.2. This is best-
effort memory cleanup, not a guarantee about parser scratch, allocator copies,
swap, process dumps or an already compromised account.

## Dependency review

`ca-05b-dependencies.json` adds only the exact local runtime-to-vault edge and
alloc-only zeroize edge. The vault is an in-memory validation crate with no native,
filesystem, network or process implementation; it is not vault-storage. All 58
external package/version/source/checksum records and five predecessor reviews
remain byte-identical. Cargo.lock changes only the runtime's internal edge list.
No new feature, build script or executable is introduced. Prior advisory evidence
is historical; no new current-advisory-clear or full upstream-audit claim is made.

## Executable evidence and limits

Eleven adversarial tests cover every byte split of a Unicode object, exact-byte
roundtrip, malformed/scalar/batch/trailing payloads, escaped and nested duplicate
keys, depth 64/65, exact/oversize frames, queue/aggregate credit, poison cleanup,
invalid UTF-8, EOF semantics and redacted diagnostics. A compile-fail doctest
prevents caller construction of JsonObjectFrame without its private validator.
All 23 inherited raw transport/allowlist tests remain. These join ordinary debug,
release, doctest and isolated Linux CI, without any new workflow skip or exemption.

```text
python tools/check_dependencies.py
cargo test -p codex-accounts-runtime --locked --offline
cargo test -p codex-accounts-runtime --release --locked --offline
```

JSON validity is deliberately NOT valid JSON-RPC, allowed method/parameters,
handshake completion, response/login correlation, policy consent or identity proof.
A syntactically valid hostile method object may pass syntax validation but has no
execution path. No raw payload is exposed as a UI status or forwarded to the
renderer. Version-bound schemas, handshake/IDs, notifications/deadlines, approved
pipes/environment, independent stderr draining and owned-helper recovery are still
later work. All full T-25/T-26/T-31 scenarios and Desktop qualification remain open.
