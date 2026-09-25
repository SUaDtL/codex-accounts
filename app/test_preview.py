"""Dependency-free static structure/escaping checks; not a screen-reader audit."""
from __future__ import annotations

import ast
from dataclasses import replace
from html.parser import HTMLParser
from pathlib import Path
import re
import unittest

from app import build_preview as ui


class Document(HTMLParser):
    def __init__(self, markup: str):
        super().__init__(convert_charrefs=True)
        self.elements: list[tuple[str, dict]] = []
        self.text: list[str] = []
        self.feed(markup)
        self.close()

    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, dict(attrs)))

    def handle_data(self, data):
        self.text.append(data)

    def tags(self, tag):
        return [attrs for name, attrs in self.elements if name == tag]


def luminance(hex_color: str) -> float:
    rgb = [int(hex_color[i:i+2], 16) / 255 for i in (1, 3, 5)]
    rgb = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in rgb]
    return sum(a * b for a, b in zip(rgb, (.2126, .7152, .0722)))


class PreviewTests(unittest.TestCase):
    def test_committed_previews_are_exact_builder_output(self):
        for name, markup in ui.pages().items():
            self.assertEqual((ui.ROOT / name).read_text(encoding="utf-8"), markup)

    def test_landmarks_titles_current_page_and_skip_target(self):
        for markup in ui.pages().values():
            doc = Document(markup)
            self.assertEqual(doc.tags("html"), [{"lang": "en"}])
            for tag in ("title", "main", "h1", "nav", "header", "footer"):
                self.assertEqual(len(doc.tags(tag)), 1, tag)
            self.assertEqual(doc.tags("main")[0]["id"], "main")
            self.assertEqual(doc.tags("main")[0]["tabindex"], "-1")
            self.assertTrue(any(a.get("href") == "#main" for a in doc.tags("a")))
            self.assertEqual(sum(a.get("aria-current") == "page" for a in doc.tags("a")), 1)
            self.assertTrue(doc.tags("nav")[0].get("aria-label"))
            self.assertIn("Preview, not the finished application", markup)

    def test_unique_ids_and_all_accessibility_references_resolve(self):
        for markup in ui.pages().values():
            doc = Document(markup)
            ids = [attrs["id"] for _, attrs in doc.elements if "id" in attrs]
            self.assertEqual(len(ids), len(set(ids)))
            for _, attrs in doc.elements:
                for key in ("aria-describedby", "aria-labelledby"):
                    for ident in attrs.get(key, "").split():
                        self.assertIn(ident, ids)
                self.assertNotIn("aria-hidden", attrs)
                self.assertNotIn("accesskey", attrs)
                if "tabindex" in attrs:
                    self.assertLessEqual(int(attrs["tabindex"]), 0)

    def test_no_script_active_resource_or_command_surface(self):
        for markup in ui.pages().values():
            doc = Document(markup)
            for tag in ("script", "iframe", "object", "embed", "img", "form", "input", "dialog", "base", "audio", "video"):
                self.assertFalse(doc.tags(tag), tag)
            for _, attrs in doc.elements:
                self.assertFalse(any(k.startswith("on") for k in attrs))
                self.assertNotIn("style", attrs)
                self.assertNotIn("data-command", attrs)
                for key in ("href", "src", "action", "srcset"):
                    if key in attrs:
                        self.assertIn(attrs[key], {"#main", "preview.css", "accounts.html", "status.html", "recovery.html"})
            for attrs in doc.tags("button"):
                self.assertIn("disabled", attrs)
                self.assertEqual(attrs["type"], "button")
                self.assertEqual(attrs["aria-describedby"], "preview-boundary")

    def test_csp_disables_network_scripts_forms_and_object_resources(self):
        for markup in ui.pages().values():
            metas = Document(markup).tags("meta")
            csp = [m["content"] for m in metas if m.get("http-equiv") == "Content-Security-Policy"]
            self.assertEqual(len(csp), 1)
            for part in ("default-src 'none'", "script-src 'none'", "connect-src 'none'", "object-src 'none'", "form-action 'none'", "base-uri 'none'", "style-src 'self'"):
                self.assertIn(part, csp[0])
            self.assertNotIn("*", csp[0])
            self.assertNotIn("unsafe-inline", csp[0])
            self.assertNotIn("user-scalable=no", markup)
            self.assertNotIn("maximum-scale", markup)

    def test_every_metadata_field_escapes_hostile_text(self):
        payloads = ('<script>alert("synthetic")</script>', '<img src=x onerror="synthetic">',
                    '\"><a href="javascript:synthetic">click</a>', "&lt;script&gt;", "' & < > \\")
        for field in ("label", "masked_email", "workspace", "captured"):
            for payload in payloads:
                profile = replace(ui.FIXTURES[0], **{field: payload})
                doc = Document(ui.accounts((profile,)))
                self.assertIn(payload, doc.text)
                self.assertFalse(doc.tags("script"))
                self.assertFalse(doc.tags("img"))
                self.assertEqual(len(doc.tags("a")), 4)

    def test_controls_surrogates_and_direction_overrides_fail_with_redacted_error(self):
        for bad in ("", " ", "x\x00", "x\n", "x\x7f", "x\ud800", "x\u202e", "x\u2066", "x" * 257, 42):
            with self.assertRaisesRegex(ui.PreviewError, "^Invalid display metadata$"):
                ui.accounts((ui.Profile(bad),))

    def test_long_unicode_labels_remain_plain_text_without_truncation(self):
        for label in ("界" * 256, "é" * 128, "مثال عربي", "🦀" * 256):
            doc = Document(ui.accounts((ui.Profile(label),)))
            self.assertIn(label, doc.text)
            self.assertGreater(len(doc.tags("bdi")), 0)

    def test_profile_cap_and_empty_state(self):
        markup = ui.accounts(tuple(ui.Profile("Synthetic " + str(i)) for i in range(50)))
        self.assertEqual(len(Document(markup).tags("article")), 50)
        with self.assertRaises(ui.PreviewError):
            ui.accounts(tuple(ui.Profile("Synthetic") for _ in range(51)))
        self.assertIn("No saved examples", ui.accounts(()))
        with self.assertRaises(ui.PreviewError):
            ui.accounts([ui.Profile("Synthetic")])

    def test_unknown_workspace_and_quota_are_not_invented(self):
        markup = ui.accounts((ui.Profile("Synthetic"),))
        self.assertIn("Unknown workspace", markup)
        self.assertIn("Unknown; no online check", markup)
        self.assertNotIn("Personal</", markup)
        self.assertNotIn("0%", markup)

    def test_invalid_enum_and_page_values_do_not_render(self):
        with self.assertRaises(ui.PreviewError):
            ui.accounts((ui.Profile("Synthetic", health="Available"),))
        with self.assertRaises(ui.PreviewError):
            ui.status(ui.Observation(acceptance="Accepted"))
        with self.assertRaises(ui.PreviewError):
            ui.shell("../arbitrary", "Title", "Lead", "")

    def test_every_observation_combination_preserves_identity_boundary(self):
        for acceptance in ui.Acceptance:
            for launch in ui.Launch:
                for recovery in ui.Recovery:
                    obs = ui.Observation(acceptance, launch, recovery)
                    markup = ui.status(obs)
                    self.assertIn(obs.identity(), markup)
                    self.assertNotIn("User-confirmed", obs.identity())
                    if recovery != ui.Recovery.NONE:
                        self.assertEqual(obs.identity(), "Unknown; recovery takes priority")
                    elif launch == ui.Launch.OPENED:
                        self.assertEqual(obs.identity(), "Check account in Desktop")
                    else:
                        self.assertEqual(obs.identity(), "Unknown")

    def test_vocabulary_and_recovery_scenarios_are_complete(self):
        self.assertEqual(len(ui.STATUS_VOCABULARY), 11)
        self.assertEqual(len(Document(ui.status()).tags("details")), 11)
        self.assertEqual(len(Document(ui.status()).tags("summary")), 11)
        self.assertEqual(len(ui.RECOVERY_CASES), 8)
        markup = ui.recovery()
        for phrase in ("Primary outcome", "Restoration outcome", "newest matching generation", "No automatic rollback", "Recovery is never automatic"):
            self.assertIn(phrase, markup)
        self.assertEqual(len(Document(markup).tags("button")), 8)

    def test_layout_motion_focus_and_bundled_css(self):
        css = (ui.ROOT / "preview.css").read_text(encoding="utf-8")
        for token in (":focus-visible", ".skip:focus", "overflow-wrap: anywhere", "min-width: 0", "prefers-reduced-motion", "forced-colors", "@media (max-width: 48rem)", "minmax(min(100%", "flex-wrap: wrap"):
            self.assertIn(token, css)
        for forbidden in ("@import", "url(", "text-overflow: ellipsis", "outline: none", "overflow-x: hidden"):
            self.assertNotIn(forbidden, css)

    def test_text_and_focus_tokens_meet_contrast_floor(self):
        css = (ui.ROOT / "preview.css").read_text(encoding="utf-8")
        colors = dict(re.findall(r"--([a-z]+): (#[0-9a-f]{6});", css))
        for fg, bg in (("ink", "surface"), ("ink", "notice"), ("muted", "canvas"),
                       ("muted", "surface"), ("muted", "notice"), ("muted", "disabled"),
                       ("accent", "canvas"), ("accent", "surface"), ("surface", "accent")):
            a, b = sorted((luminance(colors[fg]), luminance(colors[bg])))
            self.assertGreaterEqual((b + .05) / (a + .05), 4.5, (fg, bg))

    def test_builder_imports_have_no_network_process_or_os_authority(self):
        tree = ast.parse((ui.ROOT / "build_preview.py").read_text(encoding="utf-8"))
        allowed = {"__future__", "argparse", "dataclasses", "enum", "html", "pathlib", "unicodedata"}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                self.assertTrue(all(n.name in allowed for n in node.names))
            elif isinstance(node, ast.ImportFrom):
                self.assertIn(node.module, allowed)
        self.assertNotIn("consent", {f.name for f in __import__('dataclasses').fields(ui.Profile)})


if __name__ == "__main__":
    unittest.main()
