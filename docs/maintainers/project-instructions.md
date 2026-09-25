# Codex Accounts project instructions

Work on `SUaDtL/codex-accounts`: manual, local, recoverable account handoff for the
official Codex Desktop application. Direction: Rust core plus a thin Tauri 2 shell.
Deliver concrete code/tests in bounded review PRs. Do not recreate merged work,
produce repeated research reports or use empty scaffolds as implementation closure.

## Sources and startup

Read `CODEX_ACCOUNTS_START_HERE.md`, live `AGENTS.md`,
`docs/maintainers/session-handoff.md`, `docs/next-pr.md` and the relevant sections of
`docs/implementation-plan.html`. The repository plan is the single working roadmap.
Project `CODEX_ACCOUNTS_*` files are snapshot exports/routing; the old CA-02 bootstrap
is superseded. Refresh live main, matching branches, open PRs, review comments and
checks before selecting work. CA-xx is a stable work ID, not a GitHub PR number.
A snapshot hash is not a reset target. Reuse a matching open PR when appropriate.

`docs/local-codex-switcher-spec.html` is normative; its embedded `artifact-model`
is canonical and the visible projection must agree. Read relevant specification
symbols and full affected source definitions, including nested AGENTS.md. Use
`tools/spec_index.py list/show/check` in a checkout. Preserve reviewed amendments
and source provenance; do not restore an older attached spec or quietly change a
MUST requirement through a plan. Keep findings, proposals, implementation, executed
tests and native qualification distinct. Pending conflicts remain restrictive.

Use Files for Project sources and the GitHub connector for live repository work.
Retrieve only needed sections and follow truncation. Do not replay old chats or
load the whole comparison for routine execution. Consult current primary vendor
sources only when an implementation depends on behavior/API/dependency facts not
established by the pinned evidence. Do not invent exact versions or native facts.

## Execution and parallel work

“Run the next PR” means resolve and execute one eligible bounded packet: inspect,
implement, test, review, publish a normal branch and open/update its PR. At the
2026-09-25 checkpoint CA-04B is merged through PR #9 and CA-04C is next; live state
may supersede this. Scoped check repairs are included; unrelated features and
automatic phase advancement are not. Stop after returning the actual PR and checks.

A bootstrap/documentation/parallelization request does not start implementation.
Use `docs/maintainers/parallel-work.md` when delegating: independent explicit scopes,
separate branches/worktrees/test roots, agreed private interfaces, exclusive file
ownership, one integrator per review branch and final combined-head validation.
Preparation is not operational integration. Do not start all proposed lanes by
default. Coordinate manifests, lockfiles, review records, workflows, module wiring,
journal/recovery interfaces and planning records through the integrator. Never race
shared-branch writes or silently stack an unreviewed prerequisite.

Missing owner evidence does not stop safe synthetic code/tests or justify fabricated
receipts. Keep unsupported cases explicit and dependent production authority disabled.
Use `docs/maintainers/open-asks.md` for precise collection steps and redacted results.
Windows 11 x64 and macOS arm64 are proposed lanes, not facts about the owner's machine.
Maintain independent macOS qualification. Do not add Linux/WSL support.

## Product boundaries

Authorized accounts only. No inference proxy, prompts, quota priming, automatic
rotation, task replay, browser cookies, host/runtime patches, policy bypass, broad
home/history migration, credential CLI/MCP/HTTP interface, remote assets or updater.
Do not change the user's backend to file mode. An existing auth.json is not effective
file-storage evidence. Do not guess package paths, publishers, services or companions.

Preserve exact qualified bytes and newest generations with encrypted inactive
storage and official managed login/refresh. Implement write-ahead recovery with
the operation, not afterward. No user-application force-close. An owned helper's
signal receipt is not exit proof. Installation, credential acceptance, launch,
Desktop identity and recovery are separate outcomes; helper observation is not
Desktop confirmation. Switching accounts does not isolate shared local history.

Keep credentials, real account IDs, login URLs, user paths and raw helper output
out of source, PRs, logs and diagnostics. Use obviously synthetic fixtures. Never
request a real auth file. Native bindings/dependencies require scoped review and
exact pins. Keep unsafe native boundaries small, justified and tested; retain safe
core restrictions. A caller flag or fixture cannot grant production qualification.

## GitHub and validation

Read `docs/maintainers/GITHUB_CONNECTOR_INDEX.md`. Rediscover `create`, `ref` and
identity/repository actions every session. Use the unfiltered catalog/exact action
when filtered discovery is incomplete. Invoke named actions; missing local gh or
an earlier tool result does not make the connector read-only. Report concrete
returned errors. Never evade an access control or tool safety rejection through
another route, encoding or split request.

For multi-file updates preserve the observed base tree/file modes, create a commit
with its observed parent, re-read the review branch and advance only that branch
with `force:false`. Inspect unexpected movement, never overwrite it. Verify the
resulting ref and complete changed-file set before opening/updating the PR. A Git
object is not a published branch. No merge, force push, branch reset, release,
permission/protection change or live account operation without separate authorization.

Use the pinned toolchain and blocking workflow: source/projection/provenance and
dependency review, Python tests, fmt, debug/release compilation/tests, doctests,
Clippy, exact named Windows cases and hosted network-isolated checks as applicable.
Acquire only reviewed dependencies; keep locked/offline validation distinct from
OS network isolation. Preserve independent failure visibility. Fix failures without
deleting tests, weakening assertions/lints or adding continue-on-error. Inspect
completed checks/logs for the actual final head and test merge/tree. A superseded
SHA's green run is not a new pass. Report unavailable tools and pending checks.

Before handoff review scope/security/recovery; tie closure to complete scenarios
and exact evidence, not counts/coverage/compilation. Update the existing roadmap,
validation, next-packet pointer and handoff concisely. Regenerate Project exports
from those records rather than maintaining another live plan. Preserve source
provenance for amendments. Return PR link, commit, concrete changes, executed checks,
pending native evidence and the next eligible action. Do not claim background work,
consumer readiness or an owner merge that has not occurred.
