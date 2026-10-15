# CA-05H: private rate-limit observations

The private synthetic codec now encodes one fixed `account/rateLimits/read`
request and decodes typed read observations and sparse update events. It uses
the existing session/reader leases, numeric tickets and actual full-write
receipts. Production constructors and native/runtime operations stay disabled.

## Behavior and limits

The fixed request contains `excludeResetCreditDetails:true` and omits
`supportsLunaReserve`, whose schema describes experiment-exposure recording.
Neither that omission nor a read method name proves absence of side effects.
F-019 still requires exact native startup/read side-effect qualification and
stop/reap/capture before real integration. There is no polling or automatic refetch.

Required read `rateLimits` is an object; an empty snapshot preserves unavailable
fields. All optional nullable fields retain Missing/Null/Present, including false,
zero, empty maps and empty lists. Window percentages use the schema's signed i32;
durations, reset timestamps and counts use i64. Negative or over-100 observations
remain unchanged, without clamping, clock authority or permission inference.
Spend-control decimal strings remain unparsed bounded text. Account identifiers,
plan and bucket names are untrusted metadata, never identity or active-account
association. Single-bucket and map observations remain independent even if they
disagree. Exact existing plan and captured reached/reset/status enums are enforced.

Dynamic bucket maps and reset-credit detail lists each admit at most 64 entries;
decoded keys/opaque IDs have a 256-byte limit and other text the existing
4096-byte/NUL bounds. Retained text has zeroizing owners and redacted Debug;
parser scratch and allocator copies are not guaranteed wiped. Passive reset-credit
details do not create reset tickets or actions. Count is never an allocation size,
and omitted detail rows are not invented from the summary count.

Only schema-unconstrained `rateLimitUpsell` is skipped with `IgnoredAny`, after
whole-frame strict UTF-8/JSON/duplicate/depth/size validation. Its content is never
retained, displayed, interpreted as a URL or converted into an action. Other
unknown fields and malformed known typed fields refuse conservatively.

Sparse rate updates return a private delta without applying a cache merge.
Missing/null observations never imply recovery or readiness. Updates reuse the
existing shared 32 notification credits with account signals and early login
completion. During cancellation they retain phase, primary failure and the fixed
deadline; they cannot acknowledge cancellation, permit normal drain or reopen a
closed driver. No caller-held event-retention queue is introduced, and the driver
budget is not a global process-memory quota.

## Source and validation

Three exact files from the already admitted original schema collection bring the
selection to 22 files / 457,451 bytes. All 317 original inventory entries and the
runtime image binding are unchanged. No original runtime command was repeated.
Current manifest SHA-256 is
`a87f649f36c3a2787182bedd125002cf1a2eff54c46383b04e7d1431a7dd758e`;
CA-05G's selection hash is retained as its predecessor.

Fifteen new actual-driver tests cover fixed requests, typed/presence/signed bounds,
64/65 collection limits, exact enums, independent map values, hostile ignored
payloads, text limits, sparse/shared-credit/cancellation rules and existing
ownership/write/correlation/envelope guards. The request test failed with
MethodUnavailable before implementation and passed after it. Raising the collection
limit and bypassing strict validation each produce intended test failures in
isolated copies; restored source passes. The first integrated run exposed the
old explicit manifest assertion, which was updated to the actual new pin.

Fresh local debug/release each pass 167 ordinary runtime tests (142 library,
14 framing, 11 JSON) and five compile-fail doctests. Python runs 232 tests with
one existing POSIX-only skip on Windows. Formatting, strict workspace/all-target
Clippy and schema/spec/dependency/workflow checks pass. Independent security,
coverage and original-schema comparison review found no actionable issue.
Both Windows workspace complements pass with complete/disjoint 397=343+54
compiled inventories per profile; workspace doctests and the optimized all-target
build pass. Final combined-head CI remains required before merge. No dependency, compatibility
entry, native test deadline, credential API or qualification rule changes.
