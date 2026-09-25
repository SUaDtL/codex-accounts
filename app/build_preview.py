"""CA-06A offline, synthetic-only HTML builder. Not a renderer IPC or application.

No input file, environment, home discovery, network, subprocess or credential API.
Only fixed bundled fixture metadata is used by the CLI. Pure rendering functions
are exercised with hostile synthetic labels by the source test suite.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from enum import Enum
from html import escape
from pathlib import Path
import unicodedata

ROOT = Path(__file__).resolve().parent
MAX_PROFILES = 50
MAX_LABEL_CHARS = 256


class PreviewError(ValueError):
    """Typed local failure; never includes rejected metadata."""


class Health(Enum):
    AVAILABLE = "Available"
    LOGIN = "Login required"
    UNSUPPORTED = "Unsupported backend"


class Acceptance(Enum):
    UNKNOWN = "Unknown"
    ACCEPTED = "Accepted by helper (synthetic observation)"
    REJECTED = "Login required"
    UNAVAILABLE = "Online check unavailable"


class Launch(Enum):
    NOT_REQUESTED = "Not requested"
    OPENED = "Desktop opened"
    FAILED = "Desktop did not open"


class Recovery(Enum):
    NONE = "None"
    REQUIRED = "Recovery required"
    CONFLICT = "Conflict needs review"


def text(value: str) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > MAX_LABEL_CHARS:
        raise PreviewError("Invalid display metadata")
    # Preserve Unicode scripts/combining marks; explicit direction overrides and
    # control/surrogate characters cannot hide or reorder security-relevant copy.
    if any(unicodedata.category(c) in {"Cc", "Cs"} or unicodedata.bidirectional(c) in
           {"LRE", "RLE", "LRO", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI"} for c in value):
        raise PreviewError("Invalid display metadata")
    return escape(value, quote=True)


def display(value: str | None, unknown: str) -> str:
    return text(unknown if value is None else value)


@dataclass(frozen=True)
class Profile:
    label: str
    masked_email: str | None = None
    workspace: str | None = None
    captured: str | None = None
    health: Health = Health.AVAILABLE


@dataclass(frozen=True)
class Observation:
    acceptance: Acceptance = Acceptance.UNKNOWN
    launch: Launch = Launch.NOT_REQUESTED
    recovery: Recovery = Recovery.NONE

    def identity(self) -> str:
        if self.recovery != Recovery.NONE:
            return "Unknown; recovery takes priority"
        if self.launch == Launch.OPENED:
            return "Check account in Desktop"
        return "Unknown"


FIXTURES = (
    Profile("Example personal", "p***@example.invalid", None, "Synthetic capture A"),
    Profile("Example team · 開発チーム", "t***@example.invalid", "Example workspace", "Synthetic capture B"),
    Profile("Example session needing login", None, None, None, Health.LOGIN),
)

STATUS_VOCABULARY = (
    ("Available", "A saved entry exists. This is not proof that a provider will accept it."),
    ("Login required", "A separately authorized official sign-in is needed; the preview cannot start it."),
    ("Unsupported backend", "The selected backend has no qualified adapter. No fallback is inferred."),
    ("Clients still running", "Potential writers prevent mutation. No force-close control is offered."),
    ("Credentials installed", "Installation and account verification are different facts."),
    ("Online check unavailable", "An unavailable observation does not imply invalid credentials or zero quota."),
    ("Desktop opened", "Process appearance alone does not confirm which account Desktop is using."),
    ("Check account in Desktop", "Confirmation is still outstanding; no preview action can record it."),
    ("User-confirmed", "Vocabulary specimen only. No current core confirmation adapter or UI control exists."),
    ("Conflict needs review", "Unknown or changed resources must be preserved rather than overwritten."),
    ("Recovery required", "Normal actions stay blocked until separately authorized recovery completes."),
)

RECOVERY_CASES = (
    ("Source unchanged", "No live write recorded", "Inspect registered staging and integrity before discarding it. Retain the latest source.", "Discard verified staging"),
    ("Partial replacement", "Recovery required", "Compare every resource with recorded generations. Restore only through the same write-ahead protocol.", "Restore saved source"),
    ("Installed; helper never started", "Explicit choice needed", "Finishing installation or restoring source needs an owner decision and current generation checks.", "Review installation choices"),
    ("Helper may have written", "Capture before restore", "Stop and reap the owned helper, then preserve its newest matching generation before any restoration.", "Preserve and review"),
    ("Committed; launch failed", "No automatic rollback", "Retain target selection and observe current state. Reopening or switching again is a separate operation.", "Review reopen options"),
    ("Unknown writer or changed policy", "Conflict needs review", "Preserve encrypted evidence. Do not overwrite unknown bytes, force-close applications or retry tokens.", "Review conflict"),
    ("Locked or damaged vault", "Recovery required", "Leave live resources untouched. Unlock or official reauthentication requires a separate supported path.", "Review vault options"),
    ("Provider invalidated session", "Login required", "Filesystem recovery cannot reverse provider invalidation. Only a fresh official login can repair this state.", "Review login requirement"),
)


def button(label: str) -> str:
    return f'<button type="button" disabled aria-describedby="preview-boundary">{text(label)}</button>'


def shell(page: str, title: str, lead: str, content: str) -> str:
    if page not in {"accounts", "status", "recovery"}:
        raise PreviewError("Invalid preview page")
    nav = "\n".join(
        f'<a href="{name}.html"' + (' aria-current="page"' if name == page else '') +
        f'>{label}</a>' for name, label in (("accounts", "Accounts"), ("status", "Status"), ("recovery", "Recovery")))
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'self'; script-src 'none'; connect-src 'none'; img-src 'none'; font-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
<meta name="referrer" content="no-referrer">
<title>{text(title)} · Codex Accounts preview</title>
<link rel="stylesheet" href="preview.css">
</head>
<body>
<a class="skip" href="#main">Skip to main content</a>
<header class="topbar"><span class="brand">Codex Accounts</span><span class="badge">Synthetic preview</span></header>
<div class="layout">
<aside><nav aria-label="Preview pages">{nav}</nav><p class="aside-note">Local account handoff<br>Presentation preparation only</p></aside>
<main id="main" tabindex="-1">
<p class="eyebrow">CA-06A / STATIC PRESENTATION</p>
<h1>{text(title)}</h1><p class="lead">{text(lead)}</p>
<section class="notice" aria-labelledby="preview-label"><h2 id="preview-label">Preview, not the finished application</h2>
<p id="preview-boundary">All metadata and states are synthetic. No account is connected, no installation is qualified, and every operational action is disabled. This page cannot authorize consent or recovery.</p></section>
{content}
<footer>Bundled assets only. No scripts, remote resources, storage, account access or command integration.</footer>
</main></div>
</body></html>
'''


def account_cards(profiles: tuple[Profile, ...]) -> str:
    if not isinstance(profiles, tuple) or len(profiles) > MAX_PROFILES:
        raise PreviewError("Profile limit exceeded or invalid collection")
    if not profiles:
        return '<section class="card"><h2>No saved examples</h2><p>A real application would require qualified onboarding. Nothing is connected here.</p>' + button("Add account") + '</section>'
    cards = []
    for index, p in enumerate(profiles, 1):
        if not isinstance(p, Profile) or not isinstance(p.health, Health):
            raise PreviewError("Invalid profile metadata")
        cards.append(f'''<article class="card" aria-labelledby="profile-{index}">
<div class="card-heading"><span class="eyebrow">SYNTHETIC PROFILE {index:02}</span><span class="badge">{text(p.health.value)}</span></div>
<h2 id="profile-{index}"><bdi>{text(p.label)}</bdi></h2>
<dl><dt>Email</dt><dd><bdi>{display(p.masked_email, 'Not provided')}</bdi></dd>
<dt>Workspace</dt><dd><bdi>{display(p.workspace, 'Unknown workspace')}</bdi></dd>
<dt>Last captured</dt><dd><bdi>{display(p.captured, 'Not captured')}</bdi></dd>
<dt>Quota</dt><dd>Unknown; no online check</dd></dl>
<div class="actions">{button('Switch')}{button('Rename')}{button('Reauthenticate')}{button('Remove inactive')}</div>
</article>''')
    return '<div class="cards">' + '\n'.join(cards) + '</div>'


def accounts(profiles: tuple[Profile, ...] = FIXTURES) -> str:
    content = '''<section class="installation" aria-labelledby="installation-title"><h2 id="installation-title">Installation &amp; backend</h2>
<dl><dt>Installation</dt><dd>Example Desktop installation (unqualified)</dd><dt>Backend</dt><dd>File (synthetic selection only)</dd><dt>Native adapter</dt><dd>Unavailable; mutation disabled</dd></dl></section>
<div class="toolbar">''' + button("Add account") + button("Capture current") + button("Diagnostics") + '</div>'
    return shell("accounts", "Your accounts", "A small account picker that keeps identity, availability and uncertainty separate.", content + account_cards(profiles))


def status(observation: Observation = Observation(Acceptance.ACCEPTED, Launch.OPENED)) -> str:
    if not isinstance(observation, Observation) or not isinstance(observation.acceptance, Acceptance) or not isinstance(observation.launch, Launch) or not isinstance(observation.recovery, Recovery):
        raise PreviewError("Invalid observation metadata")
    facts = ''.join(f'<div><dt>{text(k)}</dt><dd>{text(v)}</dd></div>' for k, v in (
        ("Credential acceptance", observation.acceptance.value),
        ("Desktop launch", observation.launch.value),
        ("Desktop identity", observation.identity()),
        ("Recovery", observation.recovery.value),
    ))
    glossary = ''.join(f'<details><summary>{text(name)}</summary><p>{text(explanation)}</p></details>' for name, explanation in STATUS_VOCABULARY)
    content = f'''<section class="card"><h2>Example observation</h2><p>This is a synthetic state combination, not an installation receipt.</p><dl class="facts">{facts}</dl></section>
<section class="card"><h2>What the status means</h2><p>Installed is not verified. Unknown workspace is not Personal. Unknown quota is not zero.</p><p>Each disclosure below is a vocabulary specimen, not a live state.</p>{glossary}</section>'''
    return shell("status", "Know what is known", "An observation can help explain state. It cannot grant authority or confirm Desktop identity.", content)


def recovery() -> str:
    choices = ''.join(f'''<article class="card"><span class="badge">{text(state)}</span><h2>{text(name)}</h2><p>{text(detail)}</p>{button(action)}</article>''' for name, state, detail, action in RECOVERY_CASES)
    content = '''<section class="card"><h2>Preserve first. Decide explicitly.</h2><p>Recovery is never automatic. Recheck the home lock, writer quiescence, current policy and exact generations before any real operation.</p>
<dl><dt>Primary outcome (synthetic)</dt><dd>Interrupted transaction</dd><dt>Restoration outcome (synthetic)</dt><dd>Not attempted; waiting for separate owner action</dd></dl>
<p>These outcomes stay separate. A restoration failure must not erase the original error.</p></section>
<div class="cards">''' + choices + '</div>'
    return shell("recovery", "A safe way back", "Review the situation without overwriting unknown state or discarding the newest generation.", content)


def pages() -> dict[str, str]:
    return {"accounts.html": accounts(), "status.html": status(), "recovery.html": recovery()}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify committed previews without writing")
    args = parser.parse_args()
    for name, content in pages().items():
        path = ROOT / name
        if args.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != content:
                raise SystemExit("Synthetic preview drift; run python app/build_preview.py")
        else:
            path.write_text(content, encoding="utf-8", newline="\n")
    print("Synthetic preview: checked" if args.check else "Synthetic preview: built")


if __name__ == "__main__":
    main()
