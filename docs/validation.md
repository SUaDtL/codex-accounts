# Bootstrap validation

Date: 2026-09-22. Scope: Q0 development foundations, not completed product acceptance.

| Check | Actual result |
| --- | --- |
| Python interpreter | Python 3.13.5, Linux x86_64 |
| `python -m unittest discover -s tests/python -v` | PASS: 35 tests, no skips in this environment |
| `python tools/spec_index.py check` | PASS: supplied source hashes, 43 requirements, 34 acceptance IDs, anchor/reference topology, empty catalog |
| `python tools/spec_index.py show F-020` | PASS: returned the selected canonical requirement only |
| Python AST and TOML parsing | PASS for all repository Python and TOML files |
| Implementation-plan embedded model | PASS: parses; all 43 requirements and 34 full acceptance scenarios remain open |
| Rust workspace tests | NOT RUN: cargo/rustc unavailable in creation environment |
| Rust fmt / clippy / lockfile acceptance | NOT RUN: compiler toolchain unavailable |
| Rust test definitions | 19 source tests; not counted as executed or passed |
| Windows/macOS native behavior | NOT RUN |
| Official Desktop, OAuth, file-mode handoff | NOT RUN; no qualification exists |
| GitHub Actions | NOT RUN; workflow definition is included, not execution evidence |
| Native packages, signing, reproducibility, SBOM | NOT RUN / not produced |

The 35 executed Python tests are developer-tool tests, not the specification's
34 complete release acceptance scenarios. They cover bounded candidate inventory,
config classification, typed errors, credential-file exclusion, read-only open
flags, default path/value redaction, symlink/hardlink/FIFO refusal, synthetic
external-change detection, Unicode paths, binary hashing, source integrity and
basic source boundaries. Python/static checks do not prove Rust behavior.

Native handle-level path/ACL trust, effective policy resolution, process ownership,
write durability, token preservation, crash recovery and Desktop identity are
unimplemented and unqualified. The inventory's preliminary path screening is not
advertised as a native write-safety implementation. No real credential was used.

The workflow must be executed and any compiler, formatter or platform issue fixed
before treating the Rust increment as build-verified. The repository's current
code cannot mutate credentials even when all development tests pass.
