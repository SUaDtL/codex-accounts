from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools import q0_inventory as q0


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.desktop = self.root / "desktop.exe"
        self.runtime = self.root / "runtime.exe"
        self.home = self.root / "candidate-home"
        self.home.mkdir()
        self.desktop.write_bytes(b"SYNTHETIC_DESKTOP_NOT_EXECUTABLE")
        self.runtime.write_bytes(b"SYNTHETIC_RUNTIME_NOT_EXECUTABLE")

    def report(self, **kwargs):
        return q0.inventory(self.desktop, self.runtime, self.home, **kwargs)

    def config(self, text):
        (self.home / "config.toml").write_text(text)

    def test_absent_config_remains_unknown(self):
        report = self.report()
        self.assertEqual(report["candidate_home_config"]["declared_backend"], "unknown")
        self.assertFalse(report["qualified"])
        self.assertFalse(report["credential_mutation_enabled"])
        self.assertGreater(len(report["unresolved_evidence"]), 5)

    def test_every_backend_remains_unqualified(self):
        for backend in ("file", "keyring", "auto", "ephemeral", "secrets", "unknown"):
            with self.subTest(backend=backend):
                self.config(f'cli_auth_credentials_store = "{backend}"\n')
                report = self.report()
                self.assertEqual(report["candidate_home_config"]["declared_backend"], backend)
                self.assertFalse(report["qualified"])
                self.assertFalse(report["credential_mutation_enabled"])

    def test_arbitrary_config_value_not_emitted(self):
        self.config('cli_auth_credentials_store = "SYNTHETIC_SECRET_CANARY"\n')
        self.assertNotIn("SYNTHETIC_SECRET_CANARY", json.dumps(self.report()))

    def test_unrelated_config_not_emitted(self):
        self.config('anything = "SYNTHETIC_SECRET_CANARY"\n')
        self.assertNotIn("SYNTHETIC_SECRET_CANARY", json.dumps(self.report()))

    def test_environment_values_not_emitted(self):
        with mock.patch.dict(os.environ, {"OPENAI_API_KEY": "SYNTHETIC_SECRET_CANARY"}):
            report = self.report()
        self.assertIn("OPENAI_API_KEY", report["environment_flags_present"])
        self.assertNotIn("SYNTHETIC_SECRET_CANARY", json.dumps(report))

    def test_default_report_has_no_local_paths(self):
        self.assertNotIn(str(self.root), json.dumps(self.report()))

    def test_local_display_is_explicit(self):
        report = self.report(show_local_paths=True)
        self.assertEqual(report["local_display_only_do_not_share"]["desktop"], str(self.desktop))

    def test_binary_hash_preserves_crlf_and_control_bytes(self):
        self.desktop.write_bytes(b"a\r\nb\x1a\x00")
        expected = hashlib.sha256(self.desktop.read_bytes()).hexdigest()
        self.assertEqual(self.report()["desktop_candidate"]["sha256"], expected)

    def test_symlinked_config_cannot_read_auth(self):
        auth = self.home / "auth.json"
        auth.write_bytes(b"SYNTHETIC_SECRET_CANARY")
        try:
            (self.home / "config.toml").symlink_to(auth)
        except OSError:
            self.skipTest("Host cannot create symlinks; native link behavior NOT PROVEN")
        with self.assertRaises(q0.InventoryError):
            self.report()

    @unittest.skipUnless(hasattr(os, "mkfifo"), "FIFO test is POSIX-only")
    def test_named_pipe_is_rejected_without_opening(self):
        pipe = self.root / "named-pipe"
        os.mkfifo(pipe)
        with self.assertRaises(q0.InventoryError):
            q0.inventory(pipe, self.runtime, self.home)

    def test_hash_matches_executable_bytes(self):
        expected = hashlib.sha256(self.desktop.read_bytes()).hexdigest()
        self.assertEqual(self.report()["desktop_candidate"]["sha256"], expected)

    def test_auth_file_is_never_opened(self):
        auth = self.home / "auth.json"
        auth.write_bytes(b"SYNTHETIC_SECRET_CANARY")
        real_open = os.open

        def guard(path, flags, *args, **kwargs):
            self.assertNotEqual(Path(path).name, "auth.json")
            self.assertEqual(flags & (os.O_WRONLY | os.O_RDWR | os.O_TRUNC | os.O_CREAT), 0)
            return real_open(path, flags, *args, **kwargs)

        with mock.patch.object(q0.os, "open", side_effect=guard):
            report = self.report()
        self.assertNotIn("SYNTHETIC_SECRET_CANARY", json.dumps(report))
        self.assertEqual(auth.read_bytes(), b"SYNTHETIC_SECRET_CANARY")

    def test_cannot_nominate_auth_as_executable(self):
        auth = self.home / "auth.json"
        auth.write_text("not to be read")
        with self.assertRaises(q0.InventoryError):
            q0.inventory(auth, self.runtime, self.home)

    def test_duplicate_toml_key_is_invalid(self):
        self.config('cli_auth_credentials_store="file"\ncli_auth_credentials_store="auto"')
        with self.assertRaises(q0.InventoryError) as caught:
            self.report()
        self.assertEqual(caught.exception.code, "E_CONFIG_INVALID")

    def test_invalid_utf8_is_sanitized(self):
        (self.home / "config.toml").write_bytes(b"\xffSYNTHETIC_SECRET_CANARY")
        with self.assertRaises(q0.InventoryError) as caught:
            self.report()
        self.assertEqual(str(caught.exception), "E_CONFIG_INVALID")

    def test_oversized_config_rejected_before_parse(self):
        (self.home / "config.toml").write_bytes(b"#" * (q0.MAX_CONFIG_BYTES + 1))
        with self.assertRaises(q0.InventoryError) as caught:
            self.report()
        self.assertEqual(caught.exception.code, "E_INPUT_LIMIT")

    def test_wrong_type_backend_is_unknown(self):
        self.config('cli_auth_credentials_store = ["file"]')
        self.assertEqual(self.report()["candidate_home_config"]["declared_backend"], "unknown")

    def test_relative_path_is_refused(self):
        with self.assertRaises(q0.InventoryError):
            q0.inventory(Path("desktop.exe"), self.runtime, self.home)

    def test_parent_traversal_is_refused(self):
        with self.assertRaises(q0.InventoryError):
            q0.inventory(self.root / ".." / self.root.name / "desktop.exe", self.runtime, self.home)

    def test_symbolic_link_is_refused(self):
        link = self.root / "link.exe"
        try:
            link.symlink_to(self.desktop)
        except OSError:
            self.skipTest("Host cannot create symlinks; native link behavior NOT PROVEN")
        with self.assertRaises(q0.InventoryError):
            q0.inventory(link, self.runtime, self.home)

    def test_hard_link_is_refused(self):
        link = self.root / "hardlink.exe"
        try:
            os.link(self.desktop, link)
        except OSError:
            self.skipTest("Host cannot create hardlinks; native link behavior NOT PROVEN")
        with self.assertRaises(q0.InventoryError):
            self.report()

    def test_directory_as_executable_is_refused(self):
        with self.assertRaises(q0.InventoryError):
            q0.inventory(self.home, self.runtime, self.home)

    def test_missing_home_not_confused_with_missing_config(self):
        self.home.rmdir()
        with self.assertRaises(FileNotFoundError):
            self.report()

    def test_permission_error_is_not_missing(self):
        with mock.patch.object(q0, "_read_candidate", side_effect=PermissionError("SYNTHETIC_SECRET_CANARY")):
            with self.assertRaises(PermissionError):
                q0.declared_backend(self.home)

    def test_main_sanitizes_os_errors(self):
        output = io.StringIO()
        with mock.patch.object(q0, "inventory", side_effect=PermissionError("SYNTHETIC_SECRET_CANARY")):
            with contextlib.redirect_stdout(output):
                result = q0.main(["--desktop-file", str(self.desktop), "--runtime-file", str(self.runtime), "--codex-home", str(self.home)])
        self.assertEqual(result, 2)
        self.assertNotIn("SYNTHETIC_SECRET_CANARY", output.getvalue())
        self.assertEqual(json.loads(output.getvalue())["error"], "E_ACCESS_DENIED")

    def test_main_success_is_inventory_not_qualification(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            result = q0.main(["--desktop-file", str(self.desktop), "--runtime-file", str(self.runtime), "--codex-home", str(self.home)])
        self.assertEqual(result, 0)
        self.assertFalse(json.loads(output.getvalue())["qualified"])

    def test_no_fixture_content_changes(self):
        self.config('cli_auth_credentials_store = "file"')
        (self.home / "history.sqlite").write_bytes(b"SYNTHETIC_HISTORY")
        (self.home / "auth.json").write_bytes(b"SYNTHETIC_AUTH")
        before = {p.relative_to(self.root): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.report()
        after = {p.relative_to(self.root): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_unicode_paths_are_accepted(self):
        dest = self.root / "runtime-\u6e2c\u8a66.exe"
        self.runtime.rename(dest)
        self.runtime = dest
        self.assertFalse(self.report()["qualified"])

    def test_detects_file_change_during_read(self):
        real_read = os.read
        changed = False

        def change(fd, count):
            nonlocal changed
            chunk = real_read(fd, count)
            if not changed:
                changed = True
                self.desktop.write_bytes(b"CHANGED_SIZE_AND_CONTENT_SYNTHETIC")
            return chunk

        with mock.patch.object(q0.os, "read", side_effect=change):
            with self.assertRaises(q0.InventoryError) as caught:
                self.report()
        self.assertEqual(caught.exception.code, "E_EXTERNAL_CHANGE")


if __name__ == "__main__":
    unittest.main()
