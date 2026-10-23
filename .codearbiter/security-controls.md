# Existing security controls

This document summarizes existing normative requirements and implemented reviewed
primitives, not a newly invented approval or compliance claim. Authority remains the
owner's request, docs/local-codex-switcher-spec.html (especially S-003 through S-012),
its dated amendments, and the dependency reviews in docs/ca-03b-dependencies.json.
The Windows app is preparing its first 1.0 release candidate. Stable 1.0 acceptance remains open.

## Credential stores and access

The approved product design explicitly manages local authentication material: the
current official Codex file backend, owned isolated official-login homes, and the
per-user encrypted vault. Exact credential bytes are preserved (S-003); inactive
generations and recovery journals are encrypted (S-006). Transient native buffers
may hold secrets during an operation and use best-effort zeroization. The vault root
key remains protected by current-user Windows DPAPI. This product necessarily retains
an unlocked root key in native memory for its vault lifetime; it never crosses UI IPC.
No environment variable, plaintext application database, renderer storage or credential
export is an approved product credential source/sink.

CI has a separate short-lived GitHub-issued token. The metadata-only retention job
receives actions:write through GH_TOKEN. The Preview and versioned-release publishers
receive contents:write, actions:read and id-token:write to publish verified successful
owner/main artifacts. All publisher jobs execute only
default-branch code. Downloaded artifacts are bounded, parsed as data and never executed;
the GitHub token is never forwarded to artifact storage. Publication precedes retention,
and a failed replacement preserves the old release bytes. This is CI API authorization,
not a product credential. No long-lived PAT or signing subscription is required.
Pinned Cosign uses the ambient GitHub Actions
OIDC provider directly; identity tokens are not passed through command arguments,
saved by repository code, or logged. Its subprocess receives no GH_TOKEN/GITHUB_TOKEN.
Public Sigstore/Fulcio/Rekor services receive the intended CI signing identity and
artifact digest, never app account material. Signatures must verify against the exact
`artifact-retention.yml@refs/heads/main` or `release.yml@refs/heads/main` publisher
identity and GitHub OIDC issuer before promotion.
This supplies consumer integrity verification, not Windows publisher trust.

## Cryptography

The existing reviewed implementation uses a random 256-bit root key, current-user
DPAPI, XChaCha20-Poly1305 (256-bit key, fresh 192-bit OS-random nonce), context-separated
HKDF-SHA256 encryption/fingerprint keys and HMAC-SHA256 keyed fingerprints. SHA-256
also identifies source/build/dependency bytes and native files. Implementations are
reviewed pinned libraries or OS APIs, not custom cryptographic primitives.

Never use machine-wide DPAPI, home-built ciphers/KDFs, MD5, SHA1, DES/3DES, RC2, RC4,
Blowfish or weakened certificate validation. TLS uses normal Windows chain/hostname
validation and OS protocol policy. The usage adapter has a fixed HTTPS host/path,
disables redirects/cookies/automatic authentication and bounds response/time sizes.

## Data flow and operations

Credentials may flow only through native credential/storage/runtime boundaries and
the fixed read-only usage request. They never enter logs, diagnostic exports, renderer
IPC, screenshots, telemetry, Git or support output. Synthetic test fixtures must not
contain usable account material. Account mutation requires supported runtime/storage/
policy and stopped clients; monitoring uses existing tokens without OAuth refresh or
reset consumption. Do not patch hosts, force-close clients or rewrite ACLs to pass.

Verified Codex-managed read-only group and parent capability permissions follow the
owner-approved docs/spec-amendments/managed-parent-permissions-2026-10-21.md. Arbitrary
readers/writers, unsafe links and unsupported layouts remain refusals. Recovery must
preserve interrupted state; no automated expiry of unresolved journals.

## Evidence and delivery

Run blocking Trivy and OpenGrep checks plus review of changed secret/TLS paths. Passing
scanners do not establish whole-product security. No account material is uploaded in CI.
The artifact has no Windows Authenticode signature. Cosign adds explicit verification.
The repository separately enforces the `CI gate` on `main`.
