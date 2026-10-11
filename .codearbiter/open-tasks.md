# Open tasks

In-flight and queued work. One top-level `- ` bullet per task; the statusline and
SessionStart hook count these, EXCLUDING done (`- [x]`).

Schema (one task = a lifecycle line + indented content sub-bullets):

```
- [~] poc.auth.0001 - Validate session tokens  (started 2026-06-18)
  - Desc: reject expired/forged tokens at the auth middleware
  - Done when: an expired token returns 401; a valid one passes
  - Boundaries: auth, secrets
```

- Marker: `[ ]` queued | `[~]` in-progress | `[x]` done. In-progress and done
  carry a dated `(started YYYY-MM-DD)` / `(done YYYY-MM-DD)` parenthetical; a
  stale `[~]` is surfaced at SessionStart.
- ID `<group>.<type>.<seq>`: `group` = build phase (poc/mvp1/v1...), `type` =
  domain (auth/api/ui/infra...), `seq` = >=4-digit, numbered within each
  `group.type`. ID + title + marker are required.
- `Desc` / `Done when` / `Boundaries` are filled when known (`TBD` until then).
  Keep `Done when` to one coarse sentence — per-step verification belongs to a
  plan, not here. `Boundaries` names the security/trust boundary the task is
  expected to touch (auth, crypto, secrets, ...).
