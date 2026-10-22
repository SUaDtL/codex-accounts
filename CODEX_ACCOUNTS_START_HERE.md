# Codex Accounts: start here

Checkpoint **2026-10-22** · UI accepted; Windows application and CI delivery in PR #25.

Current build: `dist/windows-x64-refined/codex-accounts-desktop.exe`.
Includes the unified vault/settings design, compact controls and read-only usage
polling while Codex stays open, with expiry and reset handling.
See validation for measured scope; the confirmed account trial used the prior build.

The owner requests a functioning app with focused checks. The integrated checkout
is `.worktrees/native-signer182`, branch `codex/fix-discovery-factory-lifetime`.
The parent project directory is not Git. Preserve all dirty work and other worktrees.

Capture, isolated managed Add, rename/remove, health/quota, switch and recovery
are connected to the Tauri application. The optimized Windows executable builds.
The owner has confirmed real A -> B -> A switching and an additional return to B.
Both accounts are saved; the UI displays the last known active account. The local
Windows application is functioning. Release qualification and publication remain
separate from this completed owner trial.

The owner explicitly removed exact-build approval. Use current official
registration/runtime/storage/policy checks, with no version allowlist or repeated
approval. See the specification amendment dated 2026-10-21.

Main was last verified at `587b18c64f82e13dbbf4e5386e427ef48c246855`; PRs #14–#24
were merged. The integration is committed in [PR #25](https://github.com/SUaDtL/codex-accounts/pull/25).
Read its current checks for exact-head hosted results. No merge or release has been
authorized. Historical packet claims below describe earlier snapshots, including
the now-repaired discovery crash.

## Minimum reading order

| Read | Responsibility |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Repository execution and safety contract. |
| [Session handoff](docs/maintainers/session-handoff.md) | Actual merged checkpoint, code map, known limits, test commands and continuation rules. |
| [Next packet](docs/next-pr.md) | Selected independent preparations and continuation gates; resolve against live open PRs. |
| [Working roadmap](docs/implementation-plan.html) | Single maintained delivery sequence, decisions and requirement ownership. Read the selected packet, not the entire project history. |
| [Parallel work](docs/maintainers/parallel-work.md) | Proposed independent lanes, write ownership and integration barriers. Listing a lane does not start it. |
| [Open asks](docs/maintainers/open-asks.md) | Missing owner evidence/decisions and exactly what each blocks. |
| [Validation](docs/validation.md) | Exact-source execution evidence, not transferable green status. |

Then read the relevant symbols in [the normative specification](docs/local-codex-switcher-spec.html)
and the source files named by the packet. The embedded `artifact-model` is canonical;
its visible projection must agree. Plans and Project instructions cannot amend a
MUST requirement. The T-07/provenance amendment is already recorded in the repository;
do not repropose or undo it from the old attached specification.

Use `python tools/spec_index.py list`, `show SYMBOL`, and `check` in a repository
checkout. The tool reads specification symbols, not arbitrary Markdown/plan headings.
Use Files for Project sources and the GitHub connector for live repository reads and
publication. Read [the connector index](docs/maintainers/GITHUB_CONNECTOR_INDEX.md)
and rediscover the current actions; do not infer permissions from an earlier session.

## Select the right continuation

First read current `main`, matching branches, open PRs, review comments and checks.
If a matching packet PR exists, inspect and resume it; do not create a duplicate or
reset it. If a prerequisite PR remains open or failed, resolve that scope before
normal dependent integration. A closed unmerged PR is not delivered work. If new
work has advanced the packet, follow the new evidence rather than this checkpoint.

“Run the next PR” selects one eligible bounded packet. “Update bootstrap docs” or
“lay out parallel work” does not execute dependent operations, start all lanes, merge, release,
change permissions, or mutate a live account. Independent preparatory lanes need
explicit selection and a bounded deliverable. One integrator owns each review
branch; no concurrent pushes to a shared branch or edits to shared control files.

## Project attachments and instructions

[Project instructions](docs/maintainers/project-instructions.md) are the maintained
text for the Project settings field. A downloaded kit contains snapshot exports
named `CODEX_ACCOUNTS_*`; replace older routing exports and paste the refreshed
instructions manually. The kit is not a repository overlay, and uploading it does
not change repository refs or qualify an adapter. Keep the existing specification
and connector index available, but resolve revisions against the live repository.

A new session can start with:

> Read CODEX_ACCOUNTS_START_HERE.md in SUaDtL/codex-accounts. Refresh main, open PRs,
> AGENTS.md and docs/next-pr.md. Resume the matching implementation PR, or execute
> the next eligible packet. Use the parallel-work guide only for explicitly selected
> independent work. Preserve qualification gates. Return the actual PR/head/checks;
> do not merge, release, reset branches or touch live accounts.

Current functioning build: dist/windows-x64-cleanup-fix/codex-accounts-desktop.exe.
The owner confirmed the account round trip on 2026-10-21. Preserve this executable
and vault; do not repeat login or switching merely to update documentation.
