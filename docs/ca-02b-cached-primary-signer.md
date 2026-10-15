# CA-02B cached primary signer observation

This increment extends the existing read-only registered-image collector. It
reports the SHA-256 of a bounded owned copy of certificate zero from primary
signer zero in a successful cached WinTrust provider state. It does not choose a
publisher policy or establish official identity, runtime role or qualification.

The existing retained image handle/path, cached-only URL retrieval, no-UI trust
flags, chain revocation settings and final image/registration revalidation remain.
Extraction checks null pointers, SDK structure sizes, provider/signer/certificate
errors, bounded signer/chain counts and X509 encoding. DER must be nonempty and
at most 64 KiB. Borrowed provider pointers and raw DER never enter the report.
The certificate copy is made before exactly one provider CLOSE. A failed CLOSE
suppresses accepted trust and signer identity; the guard also closes on unwind.

The new `primary_signer` object has `state` and `leaf_der_sha256`:

| State | Meaning |
| --- | --- |
| `not_established_offline` | Cached trust was not established; digest is null |
| `signer_unavailable` | Cached trust accepted, bounded signer extraction unavailable; digest is null |
| `observed_not_publisher_approval` | Bounded primary leaf copied and hashed; digest is present |
| `cleanup_not_established` | Provider cleanup failed; digest is null and accepted trust is suppressed |

No subject/DN, credential identity or raw certificate is emitted. Cached trust
does not prove fresh online revocation, MSIX package-signature verification,
protected outer ancestry, effective home/backend/policy or an approved runtime.
All qualification and credential-mutation fields remain false; the compatibility
catalog stays empty. No original application executable is launched.

## Dependencies and verification

The additive [CA-02B review](ca-02b-dependencies.json) preserves all eight
predecessors, including CA-06B's 349-package addition. The lock remains exactly
407 external packages with no added package. Only the pinned discovery
`windows-sys =0.61.2` feature list gains `Win32_Security_Cryptography_Catalog` and
`Win32_Security_Cryptography_Sip`. Each enables declarations under the already
enabled Cryptography parent. No new build script, procedural macro, download or
manual ABI declaration is introduced. Historical source/license/advisory
dispositions retain their stated scope; no new advisory scan is claimed.

The unsigned synthetic registered-image test first failed because the new field
was absent, then passed with the exact negative shape. Three new ordinary tests
cover provider lifecycle/error/unwind cleanup, owned SDK fixtures with null/count/
size/DER bounds, and distinct fixed report states plus a known SHA-256. Existing
native case identities and sensitive-file canaries remain unchanged. Independent
source review found no actionable issue in borrowed-state lifetime, bounds,
cleanup or authority handling. Focused discovery debug/release each passed 30
library and four diagnostic tests; formatting and strict discovery Clippy passed.

Fresh combined-workspace validation of 229 copied public inputs passed all eleven
stages in 493.862 seconds with Rust 1.90.0: source/dependency/schema/static/workflow
checks, formatting, debug and release complements, eight doctests, optimized
all-target build and strict workspace Clippy. Each complete/disjoint inventory is
401=347+54: 344 passed, three existing owner-run cases ignored and 54 native cases
reserved for the separately mandatory hosted runner. All 254 Python tests passed
with one existing Windows platform skip. Independent additive checker review found
no actionable issue. The lock and original/copied source hashes stayed unchanged;
each owned job emptied successfully. The private summary SHA-256 is
`d25dbc5525d5bcf81691c1fd09f60b92e1392a4ba3202e07f61f3d147ec31a4e`.

These are local snapshot results, preceding final exact-head hosted gates.
Collection must use the explicit registered-metadata route in the
[Q0 procedure](q0-qualification.md), with its own exact build/report binding.
Do not use the nominated-home route to collect this signer observation.

## One exact-build read-only observation, 2026-10-15

The reviewed release diagnostic built from that frozen snapshot was 786,432 bytes,
SHA-256 `df1c0b9b5846147380d1488c024e286959db3111c969552d92c91d8b8019679a`.
One bounded invocation used the already-selected registered Desktop
`26.1007.9446.0` x64 and its package-relative runtime nomination. Only our diagnostic
executed; the installed image did not. No nominated home, real configuration,
auth, history, shutdown, installer or permission change was involved.

Collection exited zero in 0.722 seconds, left its owned job empty and revalidated
the collector/source and registered-image identities. The nominated image remained
334,121,776 bytes with SHA-256
`e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e`.
Cached trust accepted and the primary leaf DER SHA-256 was
`bfb31abc7202332a07499e91ff24bed479c4b670b0b5006a0b9984f51437c498`, matching
the earlier static certificate extraction. Unlike extraction alone, this digest
was obtained from the live successful cached provider state before cleanup.
The private report digest is
`65decfac29d77ec7ef4a253b02de657e41d58aca4ac356af49d5d0a1d43d6dd6`.

The retained [official distribution comparison](runtime-provenance-observation.md)
independently matched the image bytes at its recorded fetch. This joins two
bounded observations; it is not fresh online revocation, independent MSIX
signature verification or official publisher policy approval. Registration and
held-image stability do not establish protected outer ancestry, exclusive runtime
selection, effective home/backend/policy or credential/lifecycle semantics.
Qualification and credential-mutation authority remained false. No catalog
record, complete Q0-Q5 acceptance or production constructor was enabled.
