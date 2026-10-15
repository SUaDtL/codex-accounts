"""Parser and real-process adverse controls for the scoped proof bridge."""
from __future__ import annotations

import ctypes
from contextlib import redirect_stderr, redirect_stdout
import importlib
import importlib.util
import io
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from tools import windows_test_proof as proof

CASE = "tests::declaration_is_not_precedence"
ROW = f"test {CASE} ... ok"
SUMMARY = ("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; "
           "12 filtered out; finished in 0.01s")
VALID = f"\nrunning 1 test\n{ROW}\n\n{SUMMARY}\n\n".encode()
CANARY = "SYNTHETIC_PRIVATE_FAILURE_BODY"
SYNTHETIC_TOOLS = {"cargo": "cargo", "rustc": "rustc",
                   "cargo_version": "SYNTHETIC_UNUSED", "rustc_version": "SYNTHETIC_UNUSED"}


class FixtureChild:
    """Retain the synthetic child's identity before timeout; clean up on failure."""
    def __init__(self, pid_file: Path) -> None:
        self.pid_file = pid_file
        self.handle = None
        self.ready = threading.Event()
        self.stopped = threading.Event()
        if sys.platform == "win32":
            self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            self.kernel.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
            self.kernel.OpenProcess.restype = ctypes.c_void_p
            self.kernel.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            self.kernel.TerminateProcess.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            self.kernel.CloseHandle.argtypes = [ctypes.c_void_p]
        self.watcher = threading.Thread(target=self.retain, daemon=True)
        self.watcher.start()

    def retain(self) -> None:
        deadline = time.monotonic() + 4
        while not self.stopped.is_set() and time.monotonic() < deadline:
            try:
                pid = int(self.pid_file.read_text())
            except (FileNotFoundError, ValueError):
                time.sleep(0.01)
                continue
            if sys.platform == "win32":
                # SYNCHRONIZE | PROCESS_TERMINATE; only our freshly started child.
                self.handle = self.kernel.OpenProcess(0x00100001, False, pid)
            elif hasattr(os, "pidfd_open"):
                self.handle = os.pidfd_open(pid)
            self.ready.set()
            return

    def exited(self) -> bool:
        if not self.ready.wait(1) or self.handle is None or self.handle == 0:
            return False
        # Job termination is asynchronous. Observe the retained identity for one
        # finite second, far before the fixture's 20-second self-exit watchdog.
        started = time.monotonic()
        if sys.platform == "win32":
            result = self.kernel.WaitForSingleObject(self.handle, 1000)
            exited = result == 0
        else:
            exited = bool(select.select([self.handle], [], [], 1)[0])
            result = 0 if exited else 258
        print(json.dumps({"synthetic_child_exit_wait": result,
                          "wait_seconds": round(time.monotonic() - started, 3),
                          "deadline_seconds": 1}), flush=True)
        return exited

    def close(self) -> None:
        self.stopped.set()
        self.watcher.join(timeout=1)
        if self.handle is not None and self.handle != 0:
            if sys.platform == "win32":
                if self.kernel.WaitForSingleObject(self.handle, 0) != 0:
                    self.kernel.TerminateProcess(self.handle, 1)
                    self.kernel.WaitForSingleObject(self.handle, 1000)
                self.kernel.CloseHandle(self.handle)
            else:
                if not select.select([self.handle], [], [], 0)[0]:
                    signal.pidfd_send_signal(self.handle, signal.SIGKILL)
                os.close(self.handle)


class WindowsProofRunnerTests(unittest.TestCase):
    def bridge_case_type(self):
        spec = importlib.util.spec_from_file_location(
            "test_windows_compatibility_execution",
            proof.ROOT / "tests" / "native_windows" / "test_windows_compatibility_execution.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module.WindowsCompatibilityExecution

    def test_bridge_representation_preserves_declared_full_identity(self) -> None:
        case = self.bridge_case_type()("test_bridge_contract")
        identity = "test_windows_compatibility_execution.WindowsCompatibilityExecution.test_bridge_contract"
        self.assertEqual(case.id(), identity)
        self.assertEqual(str(case), identity)

    def test_direct_identity_labels_keep_genuine_pass_failure_and_skip(self) -> None:
        bridge_type = self.bridge_case_type()
        executed = []

        class SyntheticRepresentationCases(bridge_type):
            def test_synthetic_pass(self):
                executed.append("pass")
                self.assertEqual(2 + 2, 4)

            def test_synthetic_failure(self):
                executed.append("failure")
                self.fail("SYNTHETIC_ASSERTION_FAILURE")

            def test_synthetic_skip(self):
                executed.append("skip")
                self.skipTest("SYNTHETIC_SKIP")

        cases = [SyntheticRepresentationCases(name) for name in
                 ("test_synthetic_pass", "test_synthetic_failure", "test_synthetic_skip")]
        identities = [case.id() for case in cases]
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.TestSuite(cases))
        self.assertEqual(executed, ["pass", "failure", "skip"])
        self.assertEqual(result.testsRun, 3)
        self.assertEqual(len(result.failures), 1)
        self.assertEqual(len(result.skipped), 1)
        self.assertEqual(result.errors, [])
        self.assertFalse(result.wasSuccessful(), "real failure must remain unsuccessful")
        self.assertEqual(result.failures[0][0].id(), identities[1])
        self.assertEqual(result.skipped[0][0].id(), identities[2])
        for index, status in enumerate(("ok", "FAIL", "skipped 'SYNTHETIC_SKIP'")):
            with self.subTest(status=status):
                self.assertTrue(f"{identities[index]} ... {status}" in stream.getvalue().splitlines(),
                                "stock runner must label the genuine status with its complete identity")

    def assert_runner_absent(self, environment) -> None:
        self.assertFalse("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER" in environment,
                         "late ambient override entered invocation")

    def assert_refused(self, operation, *, category=None) -> None:
        output = io.StringIO()
        with redirect_stdout(output), redirect_stderr(output):
            with self.assertRaises(proof.ProofError) as failure:
                operation()
        self.assertNotIn(CANARY, str(failure.exception))
        self.assertIsNone(failure.exception.__cause__)
        self.assertNotIn(CANARY, output.getvalue())
        if category is not None:
            self.assertEqual(str(failure.exception), category)

    def test_exact_single_result_retains_actual_rows(self) -> None:
        for data in (VALID, VALID.replace(b"\n", b"\r\n")):
            with self.subTest(newlines=data.count(b"\r")):
                parsed = proof.verify(data, 0, CASE)
                self.assertEqual(parsed, {"named_result": ROW, "summary": SUMMARY})

    def test_missing_duplicate_ignored_failed_and_mutated_results_refuse(self) -> None:
        variants = [
            b"", VALID.replace(ROW.encode(), b""),
            VALID.replace(ROW.encode(), f"{ROW}\n{ROW}".encode()),
            VALID.replace(b" ... ok", b" ... ignored"),
            VALID.replace(b" ... ok", b" ... FAILED"),
            VALID.replace(CASE.encode(), b"tests::other"),
            VALID.replace(b"1 passed; 0 failed", b"0 passed; 1 failed"),
            VALID.replace(b"0 ignored", b"1 ignored"),
            VALID.replace(b"running 1 test", b"running 0 tests"),
            VALID + SUMMARY.encode() + b"\n",
            VALID.replace(SUMMARY.encode(), b""),
            VALID.replace(b"0 measured", b"1 measured"),
            VALID + b"test invalid-name ... ok\n",
            VALID + b"running 0 tests\n",
        ]
        for index, data in enumerate(variants):
            with self.subTest(control=index):
                self.assert_refused(lambda: proof.verify(data, 0, CASE))
        self.assert_refused(lambda: proof.verify(VALID, 7, CASE))
        self.assert_refused(lambda: proof.verify(VALID, 0, "tests::unreviewed"))

    def test_malformed_oversized_and_invalid_utf8_refuse(self) -> None:
        for index, data in enumerate((b"unstructured " + CANARY.encode(), b"\xff" + VALID,
                                      VALID + b"x" * proof.MAX_OUTPUT,
                                      VALID.replace(b"0.01s", b"NaNs"),
                                      VALID.replace(b" ... ok", b" ... ok extra"))):
            with self.subTest(control=index):
                self.assert_refused(lambda: proof.verify(data, 0, CASE))

    def test_unreviewed_argv_refuse_without_process(self) -> None:
        fixed = ["cargo", "test", "-p", "codex-accounts-discovery", "--lib",
                 "--locked", "--offline", CASE, "--", "--exact",
                 "--test-threads=1", "--format=pretty"]
        mutated = [fixed + ["--nocapture"], fixed + [CANARY], fixed[1:],
                   ["cargo", "+stable"] + fixed[1:],
                   [word for word in fixed if word != "--offline"],
                   [word for word in fixed if word != "--exact"],
                   ["rustc"] + fixed[1:]]
        with patch.object(proof, "capture") as capture:
            for index, command in enumerate(mutated):
                with self.subTest(control=index):
                    self.assert_refused(lambda: proof.run_command(command))
            with self.subTest(control="profile"):
                self.assert_refused(lambda: proof.run_case("custom", CASE))
            with self.subTest(control="case"):
                self.assert_refused(lambda: proof.run_case("debug", "tests::later_native"))
            capture.assert_not_called()

    def test_non_pinned_toolchain_refuses_without_cargo(self) -> None:
        with patch.dict(os.environ, {"RUSTUP_TOOLCHAIN": "stable"}), \
             patch.object(proof, "capture") as capture:
            self.assert_refused(lambda: proof.run_case("debug"))
            capture.assert_not_called()

    def test_real_cargo_runner_override_refuses_replacement_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="synthetic-cargo-runner-") as directory:
            runner = Path(directory) / "runner.py"
            marker = Path(directory) / "ran.txt"
            runner.write_text("from pathlib import Path\n"
                              "Path(__file__).with_name('ran.txt').write_text('SYNTHETIC_RUNNER')\n"
                              f"print({VALID.decode()!r}, flush=True)\n", encoding="utf-8")
            replacement = subprocess.list2cmdline([sys.executable, str(runner)])
            targets = ("X86_64_PC_WINDOWS_MSVC", "AARCH64_PC_WINDOWS_MSVC",
                       "X86_64_UNKNOWN_LINUX_GNU", "AARCH64_UNKNOWN_LINUX_GNU",
                       "X86_64_APPLE_DARWIN", "AARCH64_APPLE_DARWIN")
            overrides = {f"CARGO_TARGET_{target}_RUNNER": replacement for target in targets}
            try:
                with patch.dict(os.environ, overrides):
                    self.assert_refused(lambda: proof.run_case("debug"),
                                        category="unreviewed_cargo_environment")
                self.assertFalse(marker.exists(), "replacement runner must never execute")
            finally:
                print(json.dumps({"synthetic_replacement_runner_executed": marker.exists()}), flush=True)

    def test_cargo_and_compilation_environment_refuse_before_launch(self) -> None:
        overrides = {
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER": "unreviewed_cargo_environment",
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER": "unreviewed_cargo_environment",
            "CARGO_BUILD_TARGET": "unreviewed_cargo_environment",
            "CARGO_BUILD_RUSTFLAGS": "unreviewed_cargo_environment",
            "CARGO_ENCODED_RUSTFLAGS": "unreviewed_cargo_environment",
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER": "unreviewed_cargo_environment",
            "RUSTFLAGS": "unreviewed_compilation_environment",
            "RUSTDOCFLAGS": "unreviewed_compilation_environment",
            "RUSTDOC": "unreviewed_compilation_environment",
            "RUSTC_BOOTSTRAP": "unreviewed_compilation_environment",
            "RUSTC": "toolchain_not_pinned",
            "RUSTC_WRAPPER": "toolchain_not_pinned",
            "RUSTC_WORKSPACE_WRAPPER": "toolchain_not_pinned",
            "CARGO_BUILD_RUSTC": "toolchain_not_pinned",
            "CARGO_BUILD_RUSTC_WRAPPER": "toolchain_not_pinned",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER": "toolchain_not_pinned",
        }
        for name, category in overrides.items():
            with self.subTest(variable=name), patch.dict(os.environ, {name: CANARY}), \
                 patch.object(proof, "capture", side_effect=AssertionError(
                     "unreviewed execution override reached a tool process")) as capture:
                self.assert_refused(lambda: proof.run_case("debug"), category=category)
                capture.assert_not_called()

    def test_ambient_cargo_config_runner_and_wrapper_refuse_before_launch(self) -> None:
        configurations = (
            '[target.x86_64-pc-windows-msvc]\nrunner=["SYNTHETIC_UNREVIEWED"]\n',
            '[target.\'cfg(windows)\']\nrunner=["SYNTHETIC_UNREVIEWED"]\n',
            '[build]\nrustc-wrapper="SYNTHETIC_UNREVIEWED"\n',
            '[build]\nrustc-workspace-wrapper="SYNTHETIC_UNREVIEWED"\n',
        )
        tools = SYNTHETIC_TOOLS
        for location in ("cargo_home", "cwd", "ancestor"):
            for filename in ("config", "config.toml"):
                for index, content in enumerate(configurations):
                    with self.subTest(location=location, filename=filename, selection=index), \
                         tempfile.TemporaryDirectory(prefix="synthetic-cargo-config-") as directory:
                        parent = Path(directory)
                        root = parent / "workspace"
                        root.mkdir()
                        home = parent / "cargo-home"
                        config_dir = {"cargo_home": home, "cwd": root / ".cargo",
                                      "ancestor": parent / ".cargo"}[location]
                        config_dir.mkdir()
                        (config_dir / filename).write_text(content, encoding="utf-8")
                        with patch.object(proof, "ROOT", root), \
                             patch.dict(os.environ, {"CARGO_HOME": str(home)}), \
                             patch.object(proof, "pinned_tools", return_value=tools), \
                             patch.object(proof, "capture", return_value=(VALID, 0)) as capture:
                            self.assert_refused(lambda: proof.run_case("debug"),
                                                category="unreviewed_cargo_configuration")
                            capture.assert_not_called()

    def test_uncertain_cargo_config_metadata_refuses_before_launch(self) -> None:
        with patch.object(Path, "lstat", side_effect=PermissionError(CANARY)), \
             patch.object(proof, "pinned_tools", return_value=SYNTHETIC_TOOLS), \
             patch.object(proof, "capture", return_value=(VALID, 0)) as capture:
            self.assert_refused(lambda: proof.run_case("debug"),
                                category="unreviewed_cargo_configuration")
            capture.assert_not_called()

    def test_cargo_config_appearing_at_invocation_boundaries_refuses(self) -> None:
        for boundary in ("after_lookup", "during_cargo"):
            with self.subTest(boundary=boundary), tempfile.TemporaryDirectory(prefix="synthetic-cargo-drift-") as directory:
                home = Path(directory)
                config = home / "config.toml"
                tools = SYNTHETIC_TOOLS

                def lookup(*args, **kwargs):
                    if boundary == "after_lookup":
                        config.write_text('[build]\nrustc-wrapper="SYNTHETIC_UNREVIEWED"\n')
                    return tools

                def invoke(*args, **kwargs):
                    config.write_text('[target.\'cfg(windows)\']\nrunner=["SYNTHETIC_UNREVIEWED"]\n')
                    return VALID, 0

                with patch.dict(os.environ, {"CARGO_HOME": str(home)}), \
                     patch.object(proof, "pinned_tools", side_effect=lookup), \
                     patch.object(proof, "capture", side_effect=invoke) as capture:
                    self.assert_refused(lambda: proof.run_case("debug"),
                                        category="unreviewed_cargo_configuration")
                    self.assertEqual(capture.call_count, 0 if boundary == "after_lookup" else 1)

    def test_environment_snapshot_cannot_acquire_late_runner_override(self) -> None:
        runner_key = "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER"

        def lookup(*args, **kwargs):
            os.environ[runner_key] = CANARY
            return SYNTHETIC_TOOLS

        def invoke(*args, **kwargs):
            self.assert_runner_absent(kwargs["env"])
            raise proof.ProofError("synthetic_launch_stopped")

        with patch.dict(os.environ), patch.object(proof, "pinned_tools", side_effect=lookup), \
             patch.object(proof, "capture", side_effect=invoke):
            self.assert_refused(lambda: proof.run_case("debug"), category="synthetic_launch_stopped")

    def test_environment_failure_diagnostics_are_sanitized(self) -> None:
        synthetic = {"CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER": CANARY,
                     "PRIVATE_PATH": "SYNTHETIC_PRIVATE_HOME_PATH",
                     "PRIVATE_IDENTITY": "SYNTHETIC_PRIVATE_IDENTITY",
                     "PRIVATE_SECRET": "SYNTHETIC_PRIVATE_SECRET"}
        output = io.StringIO()
        suite = unittest.TestSuite([unittest.FunctionTestCase(lambda: self.assert_runner_absent(synthetic))])
        result = unittest.TextTestRunner(stream=output).run(suite)
        self.assertEqual(result.testsRun, 1)
        self.assertEqual(len(result.failures), 1, "control must actually exercise an assertion failure")
        self.assertEqual(result.errors, [])
        for index, value in enumerate(synthetic.values()):
            with self.subTest(canary=index):
                self.assertFalse(value in output.getvalue(), "failure diagnostic disclosed a synthetic private value")

    def test_default_cargo_home_checks_remain_bound_to_frozen_environment(self) -> None:
        tools = proof.pinned_tools()
        with tempfile.TemporaryDirectory(prefix="synthetic-frozen-home-") as directory:
            parent = Path(directory)
            root = parent / "workspace"
            root.mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname="codex-accounts-discovery"\nversion="0.0.0"\nedition="2021"\n')
            (root / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname="codex-accounts-discovery"\nversion="0.0.0"\n')
            (root / "src").mkdir()
            (root / "src" / "lib.rs").write_text(
                '#[cfg(test)] mod tests { #[test] fn declaration_is_not_precedence() { '
                'std::fs::write("rust-ran.txt", "SYNTHETIC_RUST_EXECUTION").unwrap(); '
                'panic!("SYNTHETIC_DELIBERATE_FAILURE"); } }')
            frozen_home = parent / "frozen-home"
            ambient_home = parent / "ambient-home"
            frozen_home.mkdir()
            ambient_home.mkdir()
            runner = parent / "runner.py"
            marker = parent / "replacement-ran.txt"
            runner.write_text("from pathlib import Path\n"
                              "Path(__file__).with_name('replacement-ran.txt').write_text('SYNTHETIC_RUNNER')\n"
                              f"print({VALID.decode()!r}, flush=True)\n", encoding="utf-8")
            home_key = "USERPROFILE" if sys.platform == "win32" else "HOME"
            with patch.dict(os.environ, {home_key: str(frozen_home)}):
                os.environ.pop("CARGO_HOME", None)
                environment = dict(os.environ)
                environment.update(RUSTUP_TOOLCHAIN=proof.PIN, RUSTC=tools["rustc"])
                # Establish that actual Rust execution of this negative fixture fails.
                _, code = proof.capture([tools["cargo"]] + proof.cargo_command("debug")[1:],
                                        cwd=root, env=environment, timeout=30)
                self.assertNotEqual(code, 0, "deliberately failing Rust fixture must actually fail")
                rust_marker = root / "rust-ran.txt"
                self.assertTrue(rust_marker.is_file(), "actual Rust negative fixture must execute")
                rust_marker.unlink()
                config = frozen_home / ".cargo" / "config.toml"

                def lookup(*args, **kwargs):
                    config.parent.mkdir(exist_ok=True)
                    cfg = "windows" if sys.platform == "win32" else "unix"
                    config.write_text(f"[target.'cfg({cfg})']\nrunner="
                                      + json.dumps([sys.executable, str(runner)]) + "\n")
                    os.environ[home_key] = str(ambient_home)
                    return tools

                try:
                    with patch.object(proof, "ROOT", root), patch.object(proof, "pinned_tools", side_effect=lookup):
                        self.assert_refused(lambda: proof.run_case("debug"),
                                            category="unreviewed_cargo_configuration")
                    self.assertTrue(config.is_file(), "persistent frozen-home config must remain observable")
                    self.assertFalse(marker.exists(), "replacement output must never count as Rust execution")
                    self.assertFalse(rust_marker.exists(), "config refusal must precede the Rust invocation")
                finally:
                    print(json.dumps({"synthetic_frozen_home_replacement_executed": marker.exists(),
                                      "persistent_synthetic_config": config.is_file()}), flush=True)

    def test_unknown_default_cargo_home_refuses_before_launch(self) -> None:
        home_key = "USERPROFILE" if sys.platform == "win32" else "HOME"
        for value in (None, "", "SYNTHETIC_RELATIVE_HOME"):
            with self.subTest(selection="missing" if value is None else "invalid"), patch.dict(os.environ), \
                 patch.object(proof, "pinned_tools", return_value=SYNTHETIC_TOOLS), \
                 patch.object(proof, "capture", side_effect=AssertionError(
                     "uncertain default home reached a tool process")) as capture:
                os.environ.pop("CARGO_HOME", None)
                if value is None:
                    os.environ.pop(home_key, None)
                else:
                    os.environ[home_key] = value
                self.assert_refused(lambda: proof.run_case("debug"),
                                    category="unreviewed_cargo_configuration")
                capture.assert_not_called()

    def test_tool_lookup_uses_frozen_search_environment(self) -> None:
        environment = dict(os.environ)
        with patch.dict(os.environ, {"PATH": CANARY, "PATHEXT": ".SYNTHETIC"}):
            try:
                tools = proof.pinned_tools(environment)
            except proof.ProofError:
                tools = {}
            self.assertTrue(tools.get("cargo_version", "").startswith("cargo 1.90.0 "),
                            "tool lookup must use the frozen search environment")
            self.assertTrue(tools.get("rustc_version", "").startswith("rustc 1.90.0 "),
                            "compiler lookup must use the frozen search environment")

    def test_unavailable_or_wrong_toolchain_lookup_refuses(self) -> None:
        with patch.object(proof, "capture", return_value=(b"rustc 1.91.0 " + CANARY.encode(), 0)):
            self.assert_refused(proof.pinned_tools)

    def test_toolchain_file_and_compiler_drift_refuse(self) -> None:
        with tempfile.TemporaryDirectory(prefix="synthetic-toolchain-") as directory:
            root = Path(directory)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel="stable"\n')
            with patch.object(proof, "ROOT", root), patch.object(proof, "capture") as capture:
                self.assert_refused(proof.pinned_tools)
                capture.assert_not_called()
        installed_path = (str(Path(sys.executable).resolve()) + "\n").encode()
        for version in (b"rustc 1.91.0 (SYNTHETIC)", b"rustc 1.90.0-nightly (SYNTHETIC)"):
            responses = [(installed_path, 0), (installed_path, 0),
                         (b"cargo 1.90.0 (SYNTHETIC)", 0), (version, 0)]
            with self.subTest(version=version.decode()), patch.object(proof, "capture", side_effect=responses):
                self.assert_refused(proof.pinned_tools)

    def test_failed_process_is_not_retried_or_disclosed(self) -> None:
        tools = {"cargo": "cargo", "rustc": "rustc", "cargo_version": "SYNTHETIC_UNUSED",
                 "rustc_version": "SYNTHETIC_UNUSED"}
        with patch.object(proof, "pinned_tools", return_value=tools), \
             patch.object(proof, "capture", return_value=(CANARY.encode(), 7)) as capture:
            self.assert_refused(lambda: proof.run_case("debug"))
            self.assertEqual(capture.call_count, 1)

    def test_process_spawn_failure_is_sanitized(self) -> None:
        with patch.object(subprocess, "Popen", side_effect=OSError(CANARY)):
            self.assert_refused(lambda: proof.capture([sys.executable, "-c", "pass"]))

    @unittest.skipUnless(sys.platform == "win32", "Windows suspended-startup control")
    def test_real_suspended_startup_cannot_run_before_assignment(self) -> None:
        with tempfile.TemporaryDirectory(prefix="synthetic-startup-") as directory:
            marker = Path(directory) / "ran.txt"
            script = "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('SYNTHETIC_RAN');print('SYNTHETIC_RAN')"
            original_assign = proof._WindowsJob.assign
            assignments = []

            def assign(job, process):
                # Allow a real scheduling interval at the pre-assignment boundary.
                deadline = time.monotonic() + 0.25
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertFalse(marker.exists(), "user program ran before Job assignment")
                original_assign(job, process)
                assignments.append(process.pid)

            with patch.object(proof._WindowsJob, "assign", assign):
                output, code = proof.capture([sys.executable, "-c", script, str(marker)], timeout=3)
            self.assertEqual(code, 0)
            self.assertEqual(output.strip(), b"SYNTHETIC_RAN")
            self.assertEqual(marker.read_text(), "SYNTHETIC_RAN")
            self.assertEqual(len(assignments), 1)

    @unittest.skipUnless(sys.platform == "win32", "Windows suspended-startup control")
    def test_assignment_and_resume_failures_never_run_owned_program(self) -> None:
        original_launch = subprocess.Popen
        for operation, category in (("assign", "owned_process_containment_unavailable"),
                                    ("resume", "owned_process_resume_failed")):
            with self.subTest(operation=operation), tempfile.TemporaryDirectory(prefix="synthetic-startup-") as directory:
                marker = Path(directory) / "ran.txt"
                processes = []

                def launch(*args, **kwargs):
                    process = original_launch(*args, **kwargs)
                    processes.append(process)
                    return process

                script = "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('SYNTHETIC_RAN')"
                with patch.object(proof._WindowsJob, operation, side_effect=proof.ProofError(category)) as refusal, \
                     patch.object(subprocess, "Popen", side_effect=launch):
                    self.assert_refused(lambda: proof.capture(
                        [sys.executable, "-c", script, str(marker)], timeout=3), category=category)
                refusal.assert_called_once()
                self.assertEqual(len(processes), 1)
                self.assertIsNotNone(processes[0].poll(), "failed startup process must be reaped")
                self.assertFalse(marker.exists(), "failed startup must never execute the program")

    def test_invalid_process_bounds_refuse_without_launch(self) -> None:
        for timeout, limit in ((0, 1), (float("inf"), 1), (float("nan"), 1), (1, 0),
                               (1, proof.MAX_OUTPUT + 1)):
            with self.subTest(timeout=str(timeout), limit=limit), patch.object(subprocess, "Popen") as launch:
                self.assert_refused(lambda: proof.capture(
                    [sys.executable, "-c", "pass"], timeout=timeout, limit=limit),
                    category="invalid_process_bound")
                launch.assert_not_called()

    def test_real_timeout_reaps_owned_child_without_disclosure(self) -> None:
        with tempfile.TemporaryDirectory(prefix="synthetic-proof-") as directory:
            pid_file = Path(directory) / "owned-child.pid"
            child = FixtureChild(pid_file)
            script = ("import subprocess,sys,time; "
                      "child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(20)']); "
                      "open(sys.argv[1],'w').write(str(child.pid)); "
                      f"print('{CANARY}',flush=True); time.sleep(20)")
            try:
                started = time.monotonic()
                self.assert_refused(lambda: proof.capture(
                    [sys.executable, "-c", script, str(pid_file)], timeout=1), category="process_timeout")
                self.assertLess(time.monotonic() - started, 8)
                self.assertTrue(pid_file.is_file(), "real synthetic child must start")
                self.assertTrue(child.exited(), "identity-bound owned descendant must be terminated")
            finally:
                child.close()

    def test_real_oversized_process_refuses_without_disclosure(self) -> None:
        script = ("import sys,time; sys.stdout.write('" + CANARY + "' + 'x'*65536); "
                  "sys.stdout.flush(); time.sleep(20)")
        started = time.monotonic()
        self.assert_refused(lambda: proof.capture(
            [sys.executable, "-c", script], limit=4096, timeout=2), category="output_bound")
        self.assertLess(time.monotonic() - started, 8)

    def test_no_execution_on_import(self) -> None:
        with patch.object(subprocess, "Popen") as launch:
            importlib.reload(proof)
            launch.assert_not_called()
        # An imported module may define the bridge; it may not claim a pass.
        self.assertTrue(callable(getattr(proof, "run_case", None)))
        self.assertEqual(proof.cargo_command("debug"), [
            "cargo", "test", "-p", "codex-accounts-discovery", "--lib",
            "--locked", "--offline", CASE, "--", "--exact",
            "--test-threads=1", "--format=pretty"])


if __name__ == "__main__":
    unittest.main()
