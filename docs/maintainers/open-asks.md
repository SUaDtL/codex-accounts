# Open asks and narrow blockers

Checkpoint: **2026-09-25**, after CA-04B merged. These are missing evidence or owner
decisions, not claims that the owner promised particular hardware, accounts or
access. None authorizes collecting secrets or changing an existing configuration.
[The roadmap](../implementation-plan.html) owns phase gates; this register identifies
what to collect and what remains safe to build without it.

| Ask | Safe next action and expected redacted result | Blocks | Does not block |
| --- | --- | --- | --- |
| **OA-01: exact Desktop contract** | Use the read-only sequence in [Q0 qualification](../q0-qualification.md). Review the default sanitized candidate report; supply exact-build evidence references for publisher/runtime, effective home/backend/policy, resource semantics, writer/lifecycle and identity observations. Unknown fields stay unknown | Real installation-specific effects, approved runtime integration and qualification | CA-04C synthetic target adapter, pure protocol tests and static UI preparation |
| **OA-02: intended native test lane** | Confirm an owner-controlled ordinary Windows 11 x64 environment or an explicit different supported design lane. On the reviewed Windows checkout run the synthetic commands below; report source commit, OS/build, profile/case and outcome only | Claims about owner Windows 11 and ordinary-user behavior | Hosted Windows Server API tests and portable-core work |
| **OA-03: two-user and unavailable-store protection** | Follow [CA-03B two-user procedure](../ca-03b-crypto.md) using two already-existing ordinary users, creator controls before/after and byte-identical synthetic fixture confirmation. Report NOT RUN when unavailable. Unavailable-store trials need a separately agreed safe test environment, not service stoppage or security bypass | Full native protection/T-16 and release qualification | Envelopes, journal and synthetic fault implementation |
| **OA-04: representative sync/path environments** | Select permitted test directories and provider environments for the existing path-safety tests; report only case/outcome/OS-build and limits. No existing ACL repair, disabling a provider or copying a real home | Qualification of supported storage locations and path-refusal behavior | Existing controlled synthetic path/link/sharing tests |
| **OA-05: directory metadata and power-loss durability** | Agree a disposable, owner-approved failure-injection environment and exact observation procedure. Distinguish file flush, replacement, directory metadata, process restart and actual power-loss effects; retain sanitized state/recovery results | F-012 and complete T-21/T-22/T-30 durability claims; production adapter qualification | Private same-directory adapter code and process-crash/fault simulations explicitly labeled as such |
| **OA-06: independent macOS environment** | Select a macOS arm64 evidence lane and later reviewed native binding scope. Collect platform-specific facts, not copied Windows rules | macOS adapter and release qualification | Windows implementation; independently qualified first-platform release |
| **OA-07: required-check enforcement** | Owner inspects current branch rules and decides whether to require the GitHub Actions `CI gate`. PR #9 recorded enforcement off; that is a past observation, not a current settings guarantee | Repository-level enforcement, not whether the workflow itself executes | Code and documentation PRs; tests remain blocking in the workflow |
| **OA-08: release and consumer decisions** | Owner selects license, package identifiers, signing policy/distribution channel and disposition of proposed C-01..C-06 criteria. Record decisions without signing credentials | Distribution and consumer-release claims | Synthetic implementation, qualification preparation and source review |

## Immediately usable Windows synthetic collection

Prerequisites: reviewed checkout, Python 3.11+, repository-pinned Rust 1.90.0 with
rustfmt/Clippy, an existing ordinary Windows x64 session and local NTFS test storage.
No actual Codex account is required. From the repository root in PowerShell:

```powershell
git rev-parse HEAD
python tools/check_dependencies.py
if ($LASTEXITCODE -ne 0) { throw 'Dependency review failed' }
cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw 'Dependency acquisition failed' }
python tools/run_native_tests.py
if ($LASTEXITCODE -ne 0) { throw 'Named native execution failed' }
```

For the CA-04D review inventory, expect **54 passing case records** (27 named
behaviors in each profile), plus two profile/timing summaries. Each states
`desktop_qualification: "not_established"`. Consult the live verifier if a later
packet adds cases. This is native OS evidence only, not Q0 or a Desktop-switch
qualification; it does not close OA-03, OA-04 or OA-05. Follow parent-owned fixture
cleanup; never delete arbitrary test-looking directories or force-close user apps.

For OA-01, the exact build and invocation sequence is already maintained in the Q0
procedure. The expected report has `qualified: false` and
`credential_mutation_enabled: false`. Exit 0 means collection completed, not switch
support. Do not upload `--show-local-paths` output or paste a real authentication file.

## Safe evidence handling

Share source commit, platform/OS build, named command or scenario, outcomes and
sanitized limitations. Keep account identities, SIDs/usernames, home paths, login
URLs, credential fingerprints, key material, auth bytes, raw helper logs and private
traces out of chat, Git and PRs. The two-user fixture is synthetic but its contents
are unnecessary in a report. No passed fixture is a compatibility permission token.

When evidence arrives, verify its source/build and prerequisites, record the exact
scope it establishes in the relevant procedure/validation record and update this
register. Partial observations do not close the whole ask. Collection unavailable
means NOT RUN; never require unsafe owner actions merely to produce a green result.
Runtime/schema requirements discovered during OA-01 may necessitate a visible
scoped proposal, not a silent change to this product's boundaries.
