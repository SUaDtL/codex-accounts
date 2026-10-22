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

The Windows application connects current official installation discovery, encrypted
vault storage, managed login and journaled switching/recovery. See the dated
handoff for the active build and real-account trial status. No test fixture,
caller-supplied boolean, mock helper output or passing CI substitutes for actual
runtime, backend, policy and process checks. Complete release acceptance is open.
The owner removed per-build approval on 2026-10-21. Implement and check the current
runtime, file-storage, policy and recovery contract; normal official updates do
not need catalog entries. Preserve actual safety checks and truthful observations.

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
