# Registered runtime: partial Q0 evidence

Observed on 2026-10-15 against the owner's selected registered Windows Desktop.
This is a local qualification experiment, not a shipped adapter, complete Q0 or
permission to read credentials. The compatibility catalog remains empty and all
production constructors remain disabled.

## Image and public provenance

The selected current-user registration is `OpenAI.Codex`, version
`26.1007.9446.0`, x64, publisher
`CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B`. The nominated package-relative image is
`app/resources/codex.exe`, 334,121,776 bytes, SHA-256
`e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e`.

The [official Windows deployment instructions](https://learn.chatgpt.com/docs/enterprise/windows-deployment)
linked the x64 MSIX at
`https://persistent.oaistatic.com/codex-app-prod/ChatGPT-x64.msix`.
An ETag-bound, read-only range collection observed archive size 944,929,441 bytes,
ETag `"0x8DF29C21A7CB90B"`, and Last-Modified `14 Oct 2026 07:09:35 GMT`.
The remote manifest's name/version/architecture/publisher matched the selected
registration. The entire runtime entry was decompressed, CRC-checked and hashed;
its bytes matched the nominated installed image. Nothing was installed or
extracted for execution. This HTTPS provenance comparison is not an independent
MSIX signature-chain verification.

The image's extracted signing leaf names OpenAI OpCo, LLC, with issuer
Microsoft ID Verified CS EOC CA03 and certificate DER SHA-256
`bfb31abc7202332a07499e91ff24bed479c4b670b0b5006a0b9984f51437c498`.
Extracting that certificate is not chain verification. The discovery reader's
separate cached WinVerifyTrust observation is limited to its documented policy.

Read-only process metadata also found two immediate `codex.exe` children of
registered-root Desktop processes. Their image paths were outside the package
root, but size and SHA-256 matched the package and public archive image. This
connects those bytes to observed Desktop activity; it does not establish exclusive
runtime selection, protected ancestry, effective home or policy. No command lines,
process environments, memory, configuration, auth or history were read.

## Contained original-image version observation

One fixed `--version` invocation of the original registered image exited 0 and
reported `codex-cli 0.162.0-alpha.17.2`. It ran under a fresh zero-capability
AppContainer with closed stdin, a small explicit environment, read-only scratch
homes/profile and no network capabilities. Root/internal/manifest/image handles
were retained through identity and registration revalidation; the exact public
image hash was required before execution. No copied executable or uncontained
fallback was used.

An owned kill-on-close job admitted one process with 512 MiB commit, five seconds
user CPU, a ten-second operation deadline, a separate five-second cleanup bound
and 64 KiB per output stream. The observed parent was signaled, job active count
was zero, both output drains completed and the new AppContainer profile deleted
successfully. Native calls are not independently preempted; these bounds do not
establish a universal wall-time guarantee or disk quota. Ordinary AppContainer
access to some system resources is not universal filesystem/registry/IPC denial.

The 293-byte stderr was a PATH-alias setup warning: canonicalization of the
read-only scratch `CODEX_HOME` was denied. Its raw path-bearing text remains
private. The ACL was not relaxed and the command was not retried. Exit 0 does not
establish the effective home of an ordinary Desktop session.

The reviewed private experiment source SHA-256 was
`2c8c74eb0516d320779259bf80ab1c669c5b42614b9f05e22b70043aa8638241`;
its contract SHA-256 was
`058038301edc5422e589fa2a652763cd45b7259dbc7c5ac90318f7d904dead29`.
Synthetic identity/refusal, isolation, output/resource-limit and cleanup cases
and independent review preceded this one invocation. Those private experiments
are not part of the shipped reader or hosted CI, and their source digests alone
are not independently reproducible product qualification.

## Contained original-image schema observation

The [official app-server documentation](https://learn.chatgpt.com/docs/app-server)
documents version-specific `app-server generate-json-schema --out` collection.
A separately reviewed fixed-command collector ran that command once against the
same verified original image. Only a unique owned output directory was writable;
scratch homes and generated AppContainer profile stayed read-only. The producer
exited 0, its parent was signaled, job active count was zero, drains completed and
profile deletion succeeded. Its 292-byte stderr was the same class of read-only
scratch-home PATH-alias warning; no ACL was relaxed and no retry occurred.

The collector admitted **317 JSON files / 3,590,402 bytes** after cleanup, using
retained handles, repeat identity/inventory checks, strict bounded UTF-8 JSON and
file/directory alternate-stream refusal. Operational budgets were 512 files,
128 directories, eight directory levels, 16 MiB per file and 32 MiB aggregate.
Snapshots during execution are not a disk quota and may be exceeded between
observations. The previously described one-process/job/CPU/memory/pipe limits
remained in force. An independent review found a directory alternate-stream gap;
owned regressions reproduced it and the fix passed review before this invocation.

The [public manifest](../crates/runtime/schema/codex-0.162.0-alpha.17.2/manifest.json)
retains all generated names/sizes/hashes and collection-source hashes. CA-05F
included fourteen unmodified generated files, 435,254 bytes, for envelope,
handshake and account-read decoding. Its selection manifest SHA-256 was
`d4aa245f05067c9db7cb0917b7584d10f91a5429a03dd0953d5eaa15faa7bda4`.
CA-05G adds five exact managed-login/cancellation schema files from the same
captured output: nineteen selected files / 443,473 bytes. Its manifest records the
prior selection hash and has SHA-256
`8a371a4eef9b1f58797e0704e798fe8c9899c35d4264285f4554d18b4540f7b2`.
The original collection, executable binding and every generated-file hash remain
unchanged; no new runtime invocation occurred. Omitted files remain private and
are represented by metadata only.
The full sorted inventory digest is
`c3e7117ed29452428cc712b4fcea4ac9d1f3a4e65ae50ce27448665fed3db937`.
The original collector entrypoint source SHA-256 was
`dc98401d05c2fc49ae838afa86c6a38150282cca4ee4bf29eb86043004d3e658`;
its contract was
`dfdb195dabdeabb8212492de75dd68a4911daddd8046c590c3d476ebc565e94e`.

Schema descriptions and references remain untrusted data and are never executed
or remotely resolved. These hashes pin protocol input for a private decoder;
they grant no runtime execution or credential capability. Effective ordinary
Desktop home/configuration/policy, auth-resource semantics, writers/lifecycle,
identity, native integration and full Q0-Q5 acceptance remain open. No server,
account operation or real authentication trial has occurred.
