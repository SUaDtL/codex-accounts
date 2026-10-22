# Documentation

## Using and building the app

| Document | Purpose |
| --- | --- |
| [User guide](user-guide.md) | First use, account controls, monitoring and recovery. |
| [Contributing](../CONTRIBUTING.md) | Build prerequisites and focused development checks. |
| [Architecture](architecture.md) | Native components and credential boundaries. |
| [CI and build delivery](ci.md) | Jobs, security checks, downloads and retention. |
| [Security](../SECURITY.md) | Protection limits and safe reporting. |
| [Security tooling](../security/README.md) | Scanner pins, rules and update policy. |
| [Dependency policy](dependency-policy.md) | Current acquisition policy and dated reviews. |

## Maintainer references

The [normative specification](local-codex-switcher-spec.html) and
[implementation plan](implementation-plan.html) are the maintained requirements and roadmap.
Use `python tools/spec_index.py list`, `show SYMBOL` and `check` to inspect stable symbols.

[Validation](validation.md) records dated evidence and its limits.
[Session handoff](maintainers/session-handoff.md), [next packet](next-pr.md) and
[open asks](maintainers/open-asks.md) preserve continuation state. Refresh live GitHub
refs and checks before relying on historical revision or CI claims.

The CA-numbered packets, source preparation records, static preview and macOS preparation
notes are historical or scoped engineering evidence. They do not supersede current user
instructions or mean the working Windows application is still a disabled preview.
