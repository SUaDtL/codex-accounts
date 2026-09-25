# CA-06A: synthetic static presentation preview

Open `accounts.html`, `status.html` or `recovery.html` with `preview.css` beside them.
All assets are bundled. This is **not the finished application** or a Tauri shell.
There is no JavaScript, IPC, network, credential access, storage API or command
integration. Every operational button is natively disabled. The only interactions
are local navigation and semantic status disclosures. No renderer consent exists.

`build_preview.py` is a dependency-free, offline development generator, not a
production renderer or CLI for account data. Its CLI accepts no metadata file, path,
URL, account or credential input and generates only fixed synthetic fixtures:

```text
python app/build_preview.py
python app/build_preview.py --check
python -m unittest app.test_preview -v
```

The existing mandatory Python source suite discovers `tests/python/test_presentation.py`.
It runs the same tests; no new workflow, dependency, package manifest or install step
is needed. Generated HTML is committed and deterministic parity is checked in CI.

## Covered states and text boundaries

Accounts show explicit unqualified installation/backend, masked example.invalid
addresses, known/unknown workspace, capture placeholders and unknown quota. Status
keeps credential acceptance, launch, identity and recovery separate. User-confirmed
appears only as a clearly labelled vocabulary specimen, never as an asserted state.
The recovery page covers all eight SPEC-RECOVERY cases, with separate primary and
restoration outcomes and no automatic recovery action.

Every metadata field is escaped as text, with direction isolation for Unicode.
Control/surrogate/direction-override input is rejected with fixed errors. At most 50
profile cards are rendered. The 256-character rendering bound is a synthetic stress
limit, not a change to the backend's stricter ProfileText validation or a public DTO.
No profile/operation identifier or authority is manufactured for this preview.

## Accessibility verification and remaining manual checks

Automated source tests cover semantic landmarks and headings, a visible keyboard
skip link, valid ARIA references, no positive tabindex, disabled native actions,
status text independent of color, disclosure semantics, escaped hostile metadata,
long Unicode, profile limits, reduced-motion and forced-color styles, responsive
wrap rules and text/focus color-token contrast. They do not claim full WCAG
conformance or prove browser layout.

Before integrated UI review, record the actual browser/OS/zoom and run this matrix:

| Check | Expected result |
| --- | --- |
| Tab from page start; activate Skip | Skip link is visible and focus moves to main; no hidden focus trap. |
| Tab through navigation and status disclosures | Native order, visible focus; Enter and Space toggle details. Disabled actions never execute or join the tab order. |
| 200% native browser zoom, plus 320/640/1280 CSS-pixel widths | No overlapping, clipped or horizontally lost content; labels and controls wrap. |
| 50 synthetic profiles and long CJK, combining, Arabic and emoji labels | Text remains legible and direction-isolated, with no HTML interpretation or lost actions. |
| Reduced motion and forced colors | No motion dependency; focus and state remain perceivable. |
| Screen reader | Landmarks, heading order, labels, disclosure states and disabled action explanations are understandable. |
| Network inspection | Only the local HTML/CSS assets are loaded; no outbound request, font, image or script. |

Browser execution was attempted in the preparation environment, but navigation was
blocked by administrator policy before pages loaded. Browser layout/keyboard/zoom
and screen-reader checks are therefore **NOT RUN**, not passed. No policy was
changed to make them run. Native WebView tests, native consent focus trapping/return,
reviewed Tauri/npm pins and real backend intent integration belong to later CA-06.
