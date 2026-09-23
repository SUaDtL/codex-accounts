# CA-02 native reader boundary

The production implementation is a read-only Windows x64 registered-package
reader. It is not a trusted-installation capability, process controller, safe
credential writer or a qualified Windows 11 Desktop adapter.

## Native calls and ownership

`crates/discovery/src/native.rs` is the only application-authored unsafe boundary.
The enclosing discovery crate denies unsafe code, and core/platform/runtime keep
`forbid(unsafe_code)`. Every native call has a local ownership/lifetime comment.

| Boundary | Purpose and ownership |
| --- | --- |
| `RoInitialize` / `RoUninitialize` | A non-Send guard balances a successful apartment initialization on the same thread. |
| Current-user `PackageManager` query | An empty SID requests current-user registrations. Name substrings only find candidates; they never establish an official publisher. |
| `CreateFileW` / `File` | Existing local disk objects only, read/read-control access, no create/truncate. Successful handles transfer once to `File`. Reparse leaves, hardlinks and non-disk objects are refused. Ancestor handles exclude delete-sharing. |
| `GetFileInformationByHandle` | Records and rechecks leaf identity, size, attributes, link count and last-write metadata. |
| `GetSecurityInfo` / `AccessCheck` | Observes current-user modification/replacement rights using a temporary duplicate token. Does not install impersonation, change privileges or modify ACLs. Returned descriptors use `LocalFree`; token guards close owned handles. |
| `WinVerifyTrust` | Uses held file identity, no UI, cached-only URL retrieval, and always closes verification state. A successful result is cached trust, not reviewed official publisher identity or fresh online revocation evidence. |
| WinRT XML / cryptography | Bounded package manifest with DTD/external entities disabled and depth 64. SHA-256 hashes bytes without loading or executing the image. |

The caller re-observes package registration and retains opened image handles until
the inspection completes. A nominated runtime must be an executable-relative path
inside the selected registration. Its role and protocol remain unqualified.

## Explicit limitations

No reviewed official publisher/package reference, exact-build default, effective
home/precedence/policy rule, runtime schema, credential resource set or lifecycle
contract exists. These stay unknown. A valid unrelated signature, string match or
caller-supplied flag cannot enable any account operation.

This read adapter is not S-008's later protected-write/vault implementation. It
does not establish a vault location, known-sync exclusion, complete ownership
policy, or protection from hostile same-user changes. AccessCheck describes the
current process token, not every other local principal. Held handles and repeated
observations reduce substitution exposure; no perfect exclusion is claimed.

No process enumeration/termination, registry modification, credential-store access,
auth-file read or intentional network request is implemented. Cached-only trust
flags prevent URL retrieval by that verification operation; complete OS-actor
network/filesystem traces on an owner installation remain qualification evidence,
not a property inferred from successful compilation. XML external resolution is off.

## Tests and evidence

The Windows hosted run for `39a74ab8f4f31e6b7d3c1fde4fe21d74850449d4`
executed the native ACL-denial, junction, hardlink, sharing-writer, leaf-replacement,
Unicode/bounds, SHA-256, current-user inventory and selected-file instrumentation
cases. See run 35794749871, Windows job 106971327239. It ran on Windows Server
2025 x64, not the owner's Windows 11 installation. The inventory test accepts a
typed policy denial as a refusal-path result, not proof of successful enumeration.

Synthetic ACL/junction changes occur only in test-created temporary objects.
The selected-file test holds a synthetic auth file with exclusive sharing, records
all native opened paths, and verifies its bytes and other protected fixtures stay
unchanged. Default reports omit raw paths/configuration; debug/errors redact bodies.
The CLI additionally refuses redirected `--show-local-paths` output.

No installed official Desktop trial, packet capture, signed package fixture,
consumer support claim or complete T-03/T-04/T-17 closure follows from these tests.

Primary API references (implementation semantics, not installed-product evidence):
- https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.findpackagesforuser
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
- https://learn.microsoft.com/en-us/windows/win32/api/wintrust/ns-wintrust-wintrust_data
- https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-accesscheck
