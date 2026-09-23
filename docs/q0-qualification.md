# Q0: collect the exact Desktop contract

Status: discovery reader implemented; no Desktop installation qualified.
Windows 11 x64 is the proposed first lane. The hosted native test evidence is
Windows Server 2025 x64 with synthetic resources, not the owner's installation.
macOS remains a separate qualification lane. Linux/WSL are not product targets.

## Prerequisites

Use an owner-controlled ordinary-user Windows x64 session. Acquire the reviewed
Rust 1.90.0 toolchain and dependencies as documented below. No admin elevation,
security-control bypass, ACL repair, app shutdown, login or real auth file is needed.
Do not send real authentication files, raw paths or local-only output to an agent.

## Read-only collection

From the reviewed checkout, in PowerShell:

```powershell
python tools/check_dependencies.py
cargo fetch --locked
cargo build -p codex-accounts-discovery --locked --offline
.\target\debug\codex-accounts-discovery.exe
```

The first report lists bounded current-user registered candidates using package
name hints. Zero candidates is an explicit outcome; unpackaged installations are
not searched or supported. One or multiple candidates still require an explicit
opaque ID for detailed inspection. This is not an installation-selection guess.

```powershell
$id = Read-Host 'Candidate ID from the sanitized inventory'
.\target\debug\codex-accounts-discovery.exe --candidate $id
```

Optional local inspection, in a terminal only:

```powershell
.\target\debug\codex-accounts-discovery.exe --candidate $id --show-local-paths
```

Do not redirect, record, screenshot or upload this local-only output. The program
refuses this flag when stdout is not a terminal. For deeper observation, nominate
the actual package-relative runtime executable and candidate home found through
local owner inspection; neither is inferred from PATH or a hard-coded location:

```powershell
$runtime = Read-Host 'Actual executable-relative path within the selected package'
$home = Read-Host 'Candidate home absolute path, not assumed effective'
.\target\debug\codex-accounts-discovery.exe --candidate $id --runtime-relative $runtime --candidate-home $home
```

The reader opens only package metadata/executable candidates and the nominated
home's bounded `config.toml`. It does not open `auth.json`, credential stores or
history, launch candidates, request login, stop processes or change configuration.
The older Python inventory is a legacy synthetic developer aid, not an alternate
native discovery or authorization path. Use this native entry point for Q0.

## Expected redacted result

A completed report always has `qualified: false` and
`credential_mutation_enabled: false`. A selected candidate may report stable
registration, image hashes, PE architecture, cached signature observations and
read-handle identity. It must keep official publisher binding, runtime role,
effective home/backend/policy and exact-build rules unresolved until evidenced.

A declaration of `file` is not effective-file proof. An absent setting, missing
home, permission denial, ambiguous package or `auto` never qualifies a fallback
file. Exit 0 means collection finished; exit 2 means a typed refusal. Neither
means switch-ready. Do not copy inventory JSON into the compatibility catalog.

Share only the default sanitized report after local review. Exclude all local-only
fields, environment values, account identities, login URLs and raw OS output.
Retain private native traces outside Git. A typed error and its stage are enough
to report a failed observation; do not change security settings to make it pass.

## Evidence still required before live integration

1. Review the actual application/package publisher and immutable runtime binding;
   identify the real runtime role and exact protocol schema without substituting
   an unrelated standalone CLI.
2. Establish effective home, configuration precedence and every relevant managed
   restriction for that build. Native-reader unknown fields are not waivers.
3. Establish each auth resource, supported schema/identity extractor, presence
   semantics and permitted replacement order. No companion file is presumed.
4. Establish shared-home writers, normal quit, actual exit proof, safe reopen,
   launch side effects and what account/workspace confirmation is possible.
5. After vault/coordinator/runtime implementation, run separately authorized,
   isolated A->B->A, newest-generation and failure/recovery trials. No trial is
   authorized by this read-only collection procedure.

CA-02's T-07 wording is a visible proposal in `spec-amendments/ca-02-t07.md`, not a
qualification record. Both source-level mutation gates and the empty catalog stay
in force while review and native evidence are pending.

## Verification and rollback

Reader operations leave application state unchanged, so there is no app-state
rollback. Source repairs use ordinary follow-up commits. Synthetic/OS tests and
owner traces must distinguish switcher operations from normal Windows/app activity.
CA-03 may implement bounded generic in-memory components independently; no pending
Q0 fact may be invented to enable a live credential read or write.
