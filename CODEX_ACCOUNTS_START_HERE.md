# Codex Accounts: start here

Bootstrap revision **2.0.0** · Checkpoint **2026-09-25** · Repository **SUaDtL/codex-accounts**.
This is session routing, not an application release or a second roadmap.

## Resume from live work, not the original bootstrap

**CA-04C update:** implementation is open in [PR #11](https://github.com/SUaDtL/codex-accounts/pull/11).
Resume that branch and its current checks; the dated CA-04B checkpoint below is
historical. Read `docs/next-pr.md` before selecting any later packet.

CA-04B is **merged in PR #9**, at `614eb1d68559b37ea4e32014214519b4d2a62e54`.
Its final source head was `821ebdc26ee6081b5c74ff1da79763bef2f9ea6f`.
**CA-04C is the next implementation packet**, not CA-02. It has not started at this
checkpoint. No Desktop installation is qualified; live credential authority stays
disabled. These are dated observations, not instructions to reset a newer branch.

The earlier Project kit (revision 1.0.0, September 22) has been superseded for
routing. The repository's working roadmap already adopted that plan. Preserve old
specification/research inputs as provenance, not as replacement implementation.

## Minimum reading order

| Read | Responsibility |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Repository execution and safety contract. |
| [Session handoff](docs/maintainers/session-handoff.md) | Actual merged checkpoint, code map, known limits, test commands and continuation rules. |
| [Next packet](docs/next-pr.md) | Selected CA-04C scope and acceptance; resolve against live open PRs. |
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
“lay out parallel work” does not execute CA-04C, start all lanes, merge, release,
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
