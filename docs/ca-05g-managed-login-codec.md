# CA-05G: private managed-login wire codec

Merged in [PR #20](https://github.com/SUaDtL/codex-accounts/pull/20); the
[validation record](validation.md) binds its reviewed source, tested tree and CI.

This extends the [CA-05F codec](ca-05f-schema-codec.md) over the same private
synthetic driver. It enables no production constructor, runtime launch, browser,
credential IO or qualified account operation.

## Wire and sequencing boundary

Fixed managed ChatGPT start requests use the existing specialized login sequencer.
Replies require the original reader/session and a fully written request receipt.
The exact opaque login ID is nonempty and bounded to 256 UTF-8 bytes. Retained
dynamic strings use the existing zeroizing owners, 4096-byte bound and NUL refusal;
parser scratch and allocator copies are not guaranteed wiped. URLs remain private
data: a late start reply after cancellation discards its URL.

Early completion is explicitly buffered without exposing a success outcome until
its ID matches the authoritative start reply. Missing, null, empty or mismatched
completion IDs refuse correlation. Known optional/null error and onboarding fields
are decoded without inferring identity or credential acceptance.

Cancel requests can use only the sequencer's owned cancellation target. The exact
`canceled` and `notFound` wire statuses remain distinct secondary acknowledgements
with the original primary failure. Neither acknowledgement proves helper exit.
Schema-valid account-change signals preserve missing/null/present fields. Managed
auth mode is restricted to ChatGPT and plan values to the pinned enum; neither
field establishes a principal or workspace.

While cancelling, an account-change notification checks the fixed cancellation
deadline and consumes the existing shared 32-event credit with early completions.
It changes neither phase nor primary error and grants no drain bypass. Existing
absolute deadlines remain: 15-second start reply, 600-second interactive login,
30-second ordinary request and five-second cancellation cleanup. Informational
notification `emittedAtMs` never supplies clock authority. Strict JSON, depth,
duplicate-key, envelope, size, request ownership and fail-closed rules remain.

## Exact schema selection

Five generated login/cancel/completion files are added from the same already
admitted original-image collection. No runtime command was repeated. The selected
subset is now 19 files / 443,473 bytes; all 317 original names, sizes and digests
are unchanged. Manifest SHA-256 is
`8a371a4eef9b1f58797e0704e798fe8c9899c35d4264285f4554d18b4540f7b2`;
the prior selection hash is retained in the manifest. The schema checker binds
these bytes to the codec, without resolving references or executing a runtime.

## Validation and remaining gates

Sixteen new actual-driver test groups and one session regression cover all six
start/completion/cancel orders, early correlation, late URL discard, exact cancel
statuses, written receipts, account notification bounds, expiry and sticky errors.
Isolated invalid-status and late-URL safeguard-removal controls fail their intended
assertions, then are restored. A cancellation-without-intent category regression
was observed failing before correction and passing afterwards.

Local runtime debug/release each pass 152 ordinary tests (127 library, 14 framing,
11 JSON) and five compile-fail doctests. The combined Python suite runs 232 tests
with one existing POSIX-only skip on Windows. Formatting, strict workspace/all-target
Clippy and schema/spec/dependency/workflow checks pass. Independent security and
coverage/schema reviews found no actionable issue. Final combined-head hosted
CI passed for PR #20; these local
results do not transfer to a later source head.

No dependencies, native test deadlines, compatibility records or qualification
rules change. Rate-limit decoding, native helper ownership, effective home/policy,
stop/reap/capture, coordinator integration and full Q0-Q5 acceptance remain open.
