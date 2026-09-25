# Repository execution contract

Start with `CODEX_ACCOUNTS_START_HERE.md` and `docs/maintainers/session-handoff.md`.
Resolve `docs/next-pr.md` against live refs/open PRs. Use the single working roadmap
in `docs/implementation-plan.html`, not stale Project bootstrap status.

Read `docs/local-codex-switcher-spec.html` by stable symbol before implementing.
Use `python tools/spec_index.py show F-020` or `show SPEC-TRANSACTION` to retrieve
only the relevant canonical model blocks. Use `list` to locate IDs; `check` verifies
source hashes, requirement/test topology and the empty compatibility catalog.
Do not maintain an independently editable JSON or Markdown copy of the spec.

## Current boundary

Discovery, vault/storage, the private coordinator and CA-04B lifecycle primitives
are implemented; this is not complete Q0/Q1/Q2 qualification or a working account
switcher. See the dated handoff for evidence and refresh live source before editing.
The product's authority remains disabled. No test fixture, caller-supplied boolean,
mock helper output, hand-edited receipt or passing CI may qualify an installation.
Native evidence must be owner-run on exact builds and reviewed before an adapter
can be introduced. Preserve the empty catalog until that process is implemented.

Follow Q0 -> Q1 -> Q2 -> Q3 -> Q4 -> Q5 dependencies in the spec. Do not introduce
plaintext vaults, direct OAuth refresh, proxying, broad home copies, force-close,
host/runtime patches, cloud sync, a credential CLI/MCP surface or an updater.
Developer inventory is read-only and is not a credential-management interface.
No Rust type or synthetic result alone is proof that real OS effects occurred.
Native OS tests establish only their named behavior, not the Desktop contract.

## Development

Run Python tests and `tools/spec_index.py check`; run pinned Rust fmt, clippy and
workspace tests offline after dependency/toolchain acquisition. Record unavailable
native or compiler checks as NOT RUN, not passed. Test mapping is partial until an
entire normative T-xx scenario has real evidence. Do not count model tests as
completed release acceptance. Keep synthetic fixtures obviously synthetic.

No credentials, home paths, login URLs, real account IDs or sensitive output in
source, commits, PR descriptions, screenshots or test reports. Never weaken gates
to obtain a green test. Do not add dependencies without exact pins, lockfiles,
source/license/advisory review and a documented need. Do not choose a project
license for the owner.

For GitHub publishing, read `docs/maintainers/GITHUB_CONNECTOR_INDEX.md`, rediscover
current actions, read the target ref, then use authorized non-force updates.
Returned tool evidence determines whether publication happened. Do not merge,
release, reset branches, alter app permissions or evade repository protections.

## Parallel execution

Use `docs/maintainers/parallel-work.md` for explicitly selected independent work.
One integrator owns each review branch and shared manifests, workflows, module
wiring, journal/recovery interfaces and planning records. Separate worker branches,
checkouts and test roots; no concurrent writes to a shared branch or fixture.
Preparation does not waive operational dependencies or start all proposed lanes.
Validate the final combined head. Missing owner inputs are scoped in
`docs/maintainers/open-asks.md`; do not request real authentication material.
