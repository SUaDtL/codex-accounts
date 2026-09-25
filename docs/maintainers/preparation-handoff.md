# CA-05A / CA-06A / macOS preparation handoff

Historical PR #12 handoff. PR #12 is now merged; PR #13 integrates its work with
CA-04D and CA-05B. Current routing is `../next-pr.md`.

Checkpoint: September 25, 2026. Review PR #12, branch
`feat/ca-05-06-macos-preparation`. The actual remote head and completed checks govern
continuation, not this dated note. Main at task start was owner-merged CA-04C,
`455baa2eeee614586b26d419ee961475fe1ad23b`, tree
`25e5f0ebb19f0b9b793ae22ddb7111698e680a70`. The source archive was checked against
that exact tree. No PR was open; a separate preparation review was opened. Existing
unfinished CA-04D work was preserved and is not part of this review.

## Source map and ownership

- Protocol: `crates/runtime/src/lib.rs`, `crates/runtime/tests/framing_adversarial.rs`
  and `docs/ca-05a-protocol-preparation.md`. Raw transport stays untrusted. Fourteen
  new integration tests retain all nine inherited unit tests.
- Presentation: `app/build_preview.py`, three generated HTML pages, `preview.css`,
  `test_preview.py` and `README.md`. The standard Python suite imports sixteen
  presentation tests through `tests/python/test_presentation.py`.
- macOS: `docs/macos/native-adapter-packet.md`, `interface-baseline.json`,
  `tools/macos_preparation.py` and nine tests in `tests/python/test_macos_preparation.py`.
  The checker is source-only and cannot declare native success or qualification.

These tracks are independently reviewable. No shared storage/coordinator/key
interface, native implementation, dependency, manifest or workflow is changed.
One integrator owns subsequent joins. The older parallel-work guide describes
eligible lanes, not permission to begin their dependent operational portions.

## Reproduction and evidence limits

```text
python app/build_preview.py --check
python tools/macos_preparation.py
python -m unittest discover -s tests/python -v
python tools/spec_index.py check
python tools/check_dependencies.py
cargo fmt --all -- --check
cargo test --workspace --all-targets --no-fail-fast --locked --offline
cargo test --workspace --doc --no-fail-fast --locked --offline
cargo build --workspace --all-targets --release --locked --offline
cargo test --workspace --all-targets --release --no-fail-fast --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Local Linux source preparation passed all 143 Python tests (118 inherited, sixteen
presentation, nine macOS), including deterministic preview parity and shared-source
drift checks. Rust tooling was unavailable in that local container, so Rust proof
comes from the existing hosted lanes. First protocol-only head `ae8f0d0` passed
all 23 runtime tests in both profiles, full Ubuntu workspace/doctests/Clippy and
isolated Linux execution in run `36126434474`, but its formatting check failed.
The formatting differences are corrected in the combined preparation source;
only a completed final-head run can establish its pass. Consult the PR's validation
summary and checks rather than substituting this historical first run.

Browser navigation was blocked before the preview loaded. Browser layout, keyboard,
200% zoom and screen-reader checks are **NOT RUN**, not passed; the manual matrix is
in `app/README.md`. Static contrast/structure/escaping checks are not full WCAG
conformance or native WebView tests. All macOS native cases, owner-platform tests,
physical-power-loss trials and exact Desktop qualification are **NOT RUN**.

## Do not cross these boundaries

No invented official schema; no production helper, login, refresh or credential
access. No renderer-controlled consent or production command integration. The
preview does not assert current identity, zero quota or a Personal workspace from
missing information. It cannot record user confirmation or recovery consent.

CA-04D must be resolved and the seven shared interfaces reviewed again before
macOS native coding. Source hashes are a drift warning, not a security signature or
approval. Use the packet's bounded synthetic-only first native scope; never treat
Windows results, macOS compilation, fixtures or a manifest field as qualification.
All operational dependencies, empty-catalog and always-refuse protections remain.
No merge, reset, force push, release or permission change is part of this work.
