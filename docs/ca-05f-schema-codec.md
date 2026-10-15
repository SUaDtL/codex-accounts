# CA-05F: exact-schema handshake and account-read codec

The [partial Q0 observation](runtime-provenance-observation.md) collected the
schema from the original registered runtime, `codex-cli 0.162.0-alpha.17.2`.
The public subset keeps fourteen generated files byte-for-byte and pins all
317 generated names/sizes/hashes in its manifest. `tools/check_runtime_schema.py`
checks those bytes and the codec's version/image/manifest constants; it never
launches a runtime or resolves schema references. Its invocation and hashes grant
no runtime capability or installation qualification.

## Implemented subset

`crates/runtime/src/codec.rs` is a child of the existing private transport driver.
Its only constructor is compiled for tests. It encodes fixed initialize client
information, initialized, and account/read with `refreshToken:false`. Existing
session reservations allocate numeric IDs 1..256; actual driver write receipts
still gate replies and handshake advancement. It introduces no second sequencer.

The codec consumes a frame leased by the original driver reader and session.
Strict duplicate-key/depth/UTF-8/framing validation precedes two-pass typed serde
visitors. No generic JSON Value, raw parser error or payload-bearing Debug is
retained. Retained dynamic strings have zeroizing owners, a 4096-byte limit and
NUL rejection; parser internal scratch and allocator copies are not guaranteed
wiped. Unknown fields conservatively refuse this supported subset even where
the generated JSON schema permits additional properties.

The actual initialize result requires codexHome, platformFamily, platformOs and
userAgent. The home declaration is checked syntactically for an absolute path;
it is never read, canonicalized or treated as an effective Desktop home. The
account result requires requiresOpenaiAuth, preserving false. Missing account,
explicit null and managed ChatGPT metadata remain distinct. Managed email is a
required nullable field; the exact eighteen plan values are accepted. Neither
email nor plan is authenticated principal/workspace or credential-acceptance proof.

Unknown/stale/duplicate/unsent IDs, malformed results, remote errors and unsupported
account modes fail closed, preserving the first terminal reason and closing the
driver. Absolute session deadlines still win. A no-ID method notification returns
MethodUnavailable; an ID-bearing server request returns UnexpectedServerRequest.
Login, rate-limit decoding and notifications remain unsupported in this packet.
The existing method classifier is not permission to invoke those methods.

## Verification and remaining work

Seventeen new tests use the actual synthetic driver for fixed bytes, handshake,
ownership, correlation, out-of-order replies, missing/null/type rules, redaction,
deadline/pending bounds, syntax/framing limits and an unsent blocked request.
Two isolated safeguard-removal controls fail for their intended assertions:
bypassing ReaderFrame ownership and collapsing missing account into null. The
notification/request diagnostic correction also has a failing-before/passing-after
test. Both local runtime profiles pass 135 ordinary cases and five compile-fail
doctests. The combined Python suite runs 232 tests with one existing POSIX-only
skip on Windows. Formatting and strict workspace Clippy pass. Independent security and
coverage/schema reviews found no actionable issue in the private subset.

Seven schema-checker tests cover byte/manifest/codec pin mutations, extra/missing
files, fixed CLI diagnostics and an actual owned Windows junction while the newer
Path.is_junction API is unavailable. Python 3.11 support uses native reparse
attributes, with no silent junction-check fallback. Regressions exposed by the
combined suite retained explicit unsafe prohibitions and exact dependency maps.
The additive [dependency review](ca-05f-dependencies.json) preserves all 58 external
packages, features and six predecessor review files.

Final combined-head hosted checks must pass before merge. No actual server/RPC,
helper, native pipe, login, credentials, browser action, compatibility record or
production constructor is introduced. Next: extend the same codec for the actual
managed-login/cancel/notification schemas, then separately reviewed native helper
ownership and coordinator integration at their Q0/Q2 gates. Complete T-25 and
all full Q0-Q5 acceptance remain open.
