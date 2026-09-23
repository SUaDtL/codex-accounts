"""Deterministic projection of the repository's single working roadmap."""
from __future__ import annotations
from html import escape
import json
import hashlib
from pathlib import Path
import re

MODEL = re.compile(r'<script type="application/json" id="artifact-model">(.*?)</script>', re.S)


def table(headers: list[str], rows: list[list[str]]) -> str:
    return '<div class="table"><table><thead><tr>' + ''.join(
        '<th>' + escape(v) + '</th>' for v in headers) + '</tr></thead><tbody>' + ''.join(
        '<tr>' + ''.join('<td>' + escape(v) + '</td>' for v in row) + '</tr>' for row in rows
    ) + '</tbody></table></div>'


def render(data: dict) -> str:
    out = ['<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">',
           '<title>Codex Accounts | implementation plan</title><style>body{font:16px/1.6 system-ui,sans-serif;color:#192d3b;background:#f4f6f5;margin:0}main{max-width:1120px;margin:auto;padding:36px 24px}h1{font-size:36px;line-height:1.15}h2{margin-top:36px}a{color:#126658}a:focus-visible{outline:3px solid #93691a}table{border-collapse:collapse;width:100%;background:white;font-size:14px}th,td{text-align:left;vertical-align:top;padding:12px;border:1px solid #cfdad5;overflow-wrap:anywhere}th{background:#192d3b;color:white}.notice{background:#fff2d8;padding:18px;border-left:5px solid #94651e}.table{overflow:auto}nav{display:flex;gap:18px;flex-wrap:wrap}code{overflow-wrap:anywhere}@media(max-width:700px){table{min-width:540px}h1{font-size:28px}}@media print{body{background:white}main{padding:0}tr{break-inside:avoid}}</style><main>',
           '<header><p>CODEX ACCOUNTS / WORKING ROADMAP</p><h1>Delivery and evidence gates</h1><p>' + escape(data['status']) + '</p></header>',
           '<nav aria-label="Plan sections"><a href="#packets">Work packets</a><a href="#decisions">Decisions</a><a href="#requirements">Requirements</a><a href="#evidence">Evidence</a><a href="next-pr.md">Next packet</a></nav>',
           '<p class="notice">No Desktop installation is qualified. All complete product requirements and T-01 through T-34 release scenarios remain open. Neither this plan nor passing tests enable a credential operation.</p>',
           '<p>' + escape(data['adoption']) + '</p><section id="packets"><h2>Work packets</h2>',
           table(['Packet / phase', 'Status', 'Deliverable and next gate'],
                 [[p['id'] + ' / ' + p['phase'], p['status'], p['deliverable'] + ' ' + p['gate']] for p in data['packets']]), '</section>',
           '<section id="decisions"><h2>Decisions and prerequisites</h2>',
           table(['ID', 'State', 'Decision'], [[d['id'], d['state'], d['text']] for d in data['decisions']]),
           '<p>Production integration depends on the exact native contract. Separately bounded generic/synthetic development may proceed while evidence is pending; it does not close Q0 or qualify a later integrated phase.</p></section>',
           '<section id="requirements"><h2>Requirement ownership</h2><p>Primary packet is scheduling ownership, not exclusive test scope. All full-requirement states remain open.</p>',
           table(['Requirement', 'Primary packet', 'Acceptance references', 'State'],
                 [[r['id'], r['packet'], r['tests'], 'Open'] for r in data['requirements']]), '</section>',
           '<section id="evidence"><h2>Validation and release</h2>']
    out += ['<p>' + escape(text) + '</p>' for text in data['evidence']]
    out += ['<h3>Proposed consumer lifecycle criteria</h3>',
            table(['ID', 'Proposed criterion'], [[c['id'], c['text']] for c in data['consumer_proposals']]),
            '<p>These C-criteria are proposed delivery elaborations, not silently added F-/S- requirements. Adoption, native qualification, signing and release publication require their own review and authorization.</p></section>',
            '<footer><p>Read <a href="local-codex-switcher-spec.html">the normative specification</a>, <a href="validation.md">the validation record</a> and <a href="q0-qualification.md">the safe native collection procedure</a>. No force push, automatic merge, release or live account mutation.</p></footer></main>',
            '<script type="application/json" id="artifact-model">' + json.dumps(data, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/') + '</script></html>\n']
    return '\n'.join(out)


def check(root: Path) -> None:
    path = root / 'docs/implementation-plan.html'
    text = path.read_text(encoding='utf-8')
    matches = MODEL.findall(text)
    if len(matches) != 1:
        raise ValueError('Expected one working plan model')
    data = json.loads(matches[0])
    if render(data) != text:
        raise ValueError('Working plan model/projection mismatch')
    if data['basis']['sha256'] != hashlib.sha256(
        (root / 'docs/local-codex-switcher-spec.html').read_bytes()).hexdigest():
        raise ValueError('Working plan uses a different specification')
    if len(data['requirements']) != 43 or data['full_acceptance_status'] != 'open':
        raise ValueError('Unexpected requirement/acceptance closure')
