# CA-06B Windows static preview

CA-06B adds a Windows-only Tauri development shell in
`crates/shell-preparation`. It embeds the three static Accounts, Status and
Recovery pages and their shared stylesheet. Controls remain disabled; there is
no account backend, credential operation, runtime adapter, command handler or
plugin. This preview does not make the account switcher operational.

The reviewed source creates one programmatic window. Configuration declares an
inline capability with no granted permissions, no automatically configured
windows, no global Tauri object and no bundling. The source disables JavaScript,
allows navigation only to the three bundled page destinations and their allowed
skip fragment, and denies new windows. Its CSP declares no scripts, connections,
frames, forms or external assets. These source/configuration controls are not
an observation of effective installed browser policy or runtime IPC enforcement.

The application requires an existing WebView runtime and checks its version
before constructing the window. It contains no installer or downloader. A
missing prerequisite is a refusal, not permission to acquire or install one.

## Guarded asset preparation

From the repository root, verify the exact dependency and generated-asset inputs:

```text
python tools/check_dependencies.py
python tools/static_preview.py
```

After an intentional change to the public static files in `app/`, regenerate
only the four fixed staged assets and the original development icon, then check
the result:

```text
python tools/static_preview.py --stage
python tools/static_preview.py
```

The staging tool checks HTML/CSS against the static subset, exact configuration,
source guard presence and icon bytes. It refuses unexpected staged files,
reparse/link inputs, changed copied bytes and enabled application-command routes.
It does not build or launch a window. Review the resulting source and asset diff;
regeneration does not approve new content or grant browser/account authority.

There is no supported public launcher or double-click workflow in this slice.
The shell's internal developer launch contract requires no arguments and a new
owned run directory, then exclusively creates its own WebView profile. It has no
fallback to a normal browser profile. Do not treat an arbitrary environment path,
caller Boolean or successful compilation as launch approval. The internal smoke
launcher and its private profiles are not a product interface.

## Exact dependency scope

[ca-06b-dependencies.json](ca-06b-dependencies.json) is the additive source review
for this slice: **407 external packages = 58 retained + 349 added**. All seven
predecessor reviews, including CA-05F, remain byte-identical. The dependency
checker anchors those seven records and the new review, checks all package
identities/registry sources/checksums, and pins the current lock and all nine
workspace manifests. Rewriting a claimed predecessor or manifest hash does not
bypass the independent direct-edge checks.

The shell's direct edges exist only under `cfg(windows)`:

- `tauri =2.12.4`, default features disabled, with `wry` and `custom-protocol`.
- Build dependency `tauri-build =2.7.3`, default features disabled, with
  `config-json`.

Core/platform remain dependency-free. The runtime's four reviewed direct edges
remain unchanged. Fourteen retained-package metadata feature unions expand as a
new CA-06B change; CA-05F's historical no-expansion review is not rewritten.
In particular, target `serde_json` `raw_value` is admitted only through this exact
Tauri graph. It does not authorize a RawValue/Value API in the application's
runtime codec or disabling its recursion limit. Strict decoded duplicate-key and
depth-64 validation, typed bounded visitors, zeroizing retained sensitive fields
and fixed redacted errors remain required.

The prior Windows-filtered metadata contains **256 external plus eight local
packages**; traversing from the shell reaches **240 external packages**. These
are resolve/package-feature observations, not counts of executed compiler units
or proof that every lockfile package runs on Windows. Resolver host/target units
can differ. The combined local build observed target `serde_json` with
`alloc/default/raw_value/std` and host `serde_json` with
`alloc/default/std/unbounded_depth`; the target unit omitted `unbounded_depth`.

The recorded complete RustSec snapshot
`edb58f5974b83eb8302332821cb6d389777aa351` was matched against all 407 locked
external entries. Two active selected full-lock matches remain disclosed:
`glib 0.18.5` / RUSTSEC-2024-0429 and
`proc-macro-error 1.0.4` / RUSTSEC-2024-0370. Both are absent from the exact
Windows-filtered package/resolve set. This supports Windows-target exclusion,
not a clean full-lock advisory claim, vulnerable-function analysis or Linux/BSD
shell support. The unmaintained-package flag remains part of the recorded review.

The dependency record also retains scoped build-script/proc-macro review,
checksum-bound source and notice evidence, and byte comparison of the nine
WebView2 loader members with Microsoft.Web.WebView2 1.0.3800.47. These establish
their stated source/provenance observations, not comprehensive upstream security
or reproducible Microsoft binary builds. Future distribution still requires the
applicable license notices and covered-source obligations. No application license
has been selected by this dependency review.

## Local validation of the frozen actual workspace

A controlled offline copy of **225 actual source/input files** completed all
11 stages in **503.70 seconds**: dependency, schema, static-preview, canonical
specification and workflow checks; formatting; debug workspace complement;
workspace doctests; optimized all-target build; release complement; and strict
workspace Clippy with `-D warnings`. Rust was pinned to 1.90.0. This used the real
existing crate sources under the reviewed Tauri feature expansions, rather than
the earlier empty-target dependency preparation.

| Scope | Observed result |
| --- | --- |
| Debug compiled partition | 398 = 344 complement + 54 isolated native cases |
| Debug complement execution | 341 passed, 3 existing ignored, 54 filtered |
| Release compiled partition | 398 = 344 complement + 54 isolated native cases |
| Release complement execution | 341 passed, 3 existing ignored, 54 filtered |
| Workspace doctests | 8 passed |
| Optimized all-target build and strict Clippy | Passed |

Both compiled partitions were complete and disjoint. The runtime's actual-driver
escaped-duplicate/depth gate, ignored-upsell hostile-payload validation and the
shell's pure URL regression passed in both profiles. The 54 named isolated native
cases per profile were compiled/listed but excluded from this execution; their
separate hosted execution remains required. The three ignored owner/two-user
cases are not claimed passed.

Every stage reported an empty owned job after cleanup and unchanged original
and copied input hashes. Only the four permitted Tauri-generated JSON files were
admitted, totaling 301248 bytes. File-discovered capabilities were empty and
inline granted permissions were zero. Nonempty core ACL manifests are permission
definitions, not evidence that permissions were granted or runtime IPC was tested.

The source inventory SHA256 was
`c6fcbeaa3b42816a3175e807555f95e65003c4eee12482856540dc54d77fd632`;
the private summary SHA256 was
`c12e2d02bb4024ad87071cb484e4597341f7c4292d853779f6c7c194ef702add`.
This is local frozen-snapshot evidence. Subsequent source/documentation changes
and a final Git head still require their own combined checks and hosted CI.

The build used fresh Cargo/source/target/temp locations, sanitized inherited
environment and owned process-tree cleanup. Cargo offline prevents dependency
acquisition; it does not block sockets or filesystem access by native build
scripts/tests. No broad network/filesystem sandbox or hard disk quota is claimed.

## Private context-render smoke

One root-owned launch of the separately pinned private context executable created
a responsive 800-by-600 client window and a fresh owned WebView profile. It
accepted WM_CLOSE, exited with code zero, left its owned job empty and retained
the held executable's identity. The optional owned-client capture returned no
accepted image. **Rendered HTML, interaction and accessibility remain UNOBSERVED.**

The first attempt stopped before launch because Cargo's output had two hard
links. An exclusively created byte-identical copy satisfied the unchanged
single-link check. The launched image was 12,753,920 bytes, SHA256
`6f65eb02ae051defa644117ba0585e6b902eef8f316d46650e746f941f25723b`.
It is the separate static-context build, not the current public workspace's
final-head artifact. Responsive window creation alone does not establish that
the expected HTML rendered. No global screen capture or installed Desktop
execution occurred.

No installer, signing, release, distribution, backend or account capability is
added. Installed-runtime behavior, effective policy/home, credential preservation,
actual account operations and full Q0–Q5 qualification remain separate gates.
The compatibility catalog and production account authority remain disabled.
