"""Fixed, bounded execution proof for reviewed Rust cases; never Desktop authority."""
from __future__ import annotations

import ctypes
import math
import os
from pathlib import Path
import queue
import re
import signal
import subprocess
import sys
import threading
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PIN = "1.90.0"
BOOTSTRAP_CASE = "tests::declaration_is_not_precedence"
# Later tasks add only their implemented, reviewed exact case names here.
FIXTURE_CASE = "native::tests::native_profile_fixture_parent_is_protected_and_unsafe_parents_refuse"
FIXTURE_REFUSAL_CASE = "native::tests::native_fixture_final_check_refusal_preserves_owned_ancestor"
SLOT_CASES = (
    "storage_location::tests::exhaustive_slot_decision_table",
    "storage_location::tests::uncertain_slots_never_become_absence_or_selection",
    "storage_location::tests::opening_requires_exactly_one_unambiguous_existing_slot",
    "storage_location::tests::creation_refuses_every_existing_slot",
)
LOCATION_CASES = (
    "native::tests::native_location_mutex_recursive_and_protected_object",
    "native::tests::native_location_mutex_process_contention_release_and_abandonment",
    "native::tests::native_location_read_only_probe_refuses_denial_and_wrong_type",
    "native::tests::native_location_collision_and_replacement_refuse_before_access",
    "native::tests::native_location_failure_releases_mutex_and_retains_root_lock",
)
PROFILE_LEGACY_CASES = (
    "native::tests::native_layout_namespace_precedes_known_folder_resolution",
    "native::tests::native_layout_valid_staged_initial_anchor_reconciles_without_key_regeneration",
    "native::tests::native_layout_descriptor_drift_and_legacy_container_absence_are_lifetime_guards",
    "native::tests::native_layout_creation_failures_and_declared_pending_registry_recovery",
    "native::tests::native_layout_fresh_nested_round_trip_and_container_precedence",
    "native::tests::native_layout_reserved_collision_and_empty_container_beside_legacy_preserve_bytes",
    "native::tests::native_layout_existing_lock_is_read_only_and_lock_first",
    "native::tests::native_layout_unrecognized_initialization_and_key_matrix_are_preserved",
    "native::tests::native_layout_container_inventory_and_lifetime_changes_refuse_before_effects",
    "native::tests::native_layout_authenticated_stages_and_torn_recovery_keep_real_methods_reachable",
    "native::tests::native_layout_legacy_metadata_unsafe_grants_never_authorize_data_or_denial_fallback",
)
CASES = (("codex-accounts-discovery", BOOTSTRAP_CASE),
         ("codex-accounts-vault-storage", FIXTURE_CASE),
         ("codex-accounts-vault-storage", FIXTURE_REFUSAL_CASE),
         *(("codex-accounts-vault-storage", case) for case in SLOT_CASES + LOCATION_CASES + PROFILE_LEGACY_CASES))
MAX_OUTPUT = 1024 * 1024
TIMEOUT = 180
REAP_TIMEOUT = 5


class ProofError(ValueError):
    """A fixed refusal category, with no raw process output or exception body."""


def cargo_command(profile: str, case: str = BOOTSTRAP_CASE) -> list[str]:
    packages = [package for package, name in CASES if name == case]
    if profile not in {"debug", "release"} or len(packages) != 1:
        raise ProofError("unreviewed_invocation")
    command = ["cargo", "test", "-p", packages[0], "--lib"]
    if profile == "release":
        command.append("--release")
    return command + ["--locked", "--offline", case, "--", "--exact",
                      "--test-threads=1", "--format=pretty"]


def verify(output: bytes, returncode: int, case: str = BOOTSTRAP_CASE) -> dict[str, str]:
    if case not in {name for _, name in CASES}:
        raise ProofError("unreviewed_invocation")
    if returncode != 0 or not isinstance(output, bytes) or len(output) > MAX_OUTPUT:
        raise ProofError("failed_or_oversized_result")
    try:
        lines = output.decode("utf-8", errors="strict").splitlines()
    except UnicodeError:
        raise ProofError("malformed_result") from None
    rows = [line for line in lines if line.startswith("test ")
            and not line.startswith("test result: ")]
    summaries = [line for line in lines if line.startswith("test result: ")]
    running = [line for line in lines if line.startswith("running ")]
    pattern = (r"test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; "
               r"[0-9]+ filtered out; finished in [0-9]+(?:\.[0-9]+)?s")
    if (rows != [f"test {case} ... ok"] or running != ["running 1 test"]
            or len(summaries) != 1 or not re.fullmatch(pattern, summaries[0])):
        raise ProofError("incomplete_or_unsuccessful_result")
    # These are actual matched rows, never a replacement for captured process data.
    return {"named_result": rows[0], "summary": summaries[0]}


class _WindowsJob:
    """Anonymous kill-on-close job, containing only this invocation and its children."""
    def __init__(self) -> None:
        from ctypes import wintypes

        class BasicLimit(ctypes.Structure):
            _fields_ = [("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64),
                        ("flags", wintypes.DWORD), ("minimum", ctypes.c_size_t),
                        ("maximum", ctypes.c_size_t), ("active", wintypes.DWORD),
                        ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD),
                        ("scheduling", wintypes.DWORD)]

        class IoCounters(ctypes.Structure):
            _fields_ = [(name, ctypes.c_uint64) for name in
                        ("read_ops", "write_ops", "other_ops", "read_bytes",
                         "write_bytes", "other_bytes")]

        class ExtendedLimit(ctypes.Structure):
            _fields_ = [("basic", BasicLimit), ("io", IoCounters),
                        ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                        ("peak_process", ctypes.c_size_t), ("peak_job", ctypes.c_size_t)]

        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        self.kernel.CreateJobObjectW.restype = wintypes.HANDLE
        self.kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int,
                                                       ctypes.c_void_p, wintypes.DWORD]
        self.kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        self.kernel.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
        self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.handle = self.kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise ProofError("owned_process_containment_unavailable")
        limits = ExtendedLimit()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not self.kernel.SetInformationJobObject(
                self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close()
            raise ProofError("owned_process_containment_unavailable")

    def assign(self, process: subprocess.Popen) -> None:
        if not self.kernel.AssignProcessToJobObject(self.handle, int(process._handle)):
            raise ProofError("owned_process_containment_unavailable")

    def resume(self, process: subprocess.Popen) -> None:
        # Popen closes the primary thread handle. Resume only this owned process
        # after assignment, as in the installed bounded artifact runner.
        native = ctypes.WinDLL("ntdll", use_last_error=True)
        native.NtResumeProcess.argtypes = [ctypes.c_void_p]
        native.NtResumeProcess.restype = ctypes.c_long
        if native.NtResumeProcess(int(process._handle)) != 0:
            raise ProofError("owned_process_resume_failed")

    def terminate(self) -> None:
        if self.handle:
            self.kernel.TerminateJobObject(self.handle, 1)

    def close(self) -> None:
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


def capture(argv: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None,
            timeout: float = TIMEOUT, limit: int = MAX_OUTPUT) -> tuple[bytes, int]:
    """Capture a finite stream, terminate the owned tree on refusal, never echo it."""
    if (not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or timeout <= 0
            or not isinstance(limit, int) or not 0 < limit <= MAX_OUTPUT):
        raise ProofError("invalid_process_bound")
    process = None
    job = None
    reader = None
    stopped = threading.Event()
    chunks: queue.Queue[bytes | None] = queue.Queue(maxsize=2)
    started = time.monotonic()
    try:
        if sys.platform == "win32":
            job = _WindowsJob()
        process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   bufsize=0, shell=False, close_fds=True,
                                   start_new_session=sys.platform != "win32",
                                   creationflags=0x204 if sys.platform == "win32" else 0)
        if job is not None:
            job.assign(process)
            job.resume(process)

        def read_output() -> None:
            try:
                while not stopped.is_set():
                    data = process.stdout.read(4096)
                    while not stopped.is_set():
                        try:
                            chunks.put(data or None, timeout=0.05)
                            break
                        except queue.Full:
                            continue
                    if not data:
                        break
            except OSError:
                stopped.set()

        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()
        output = bytearray()
        while True:
            remaining = timeout - (time.monotonic() - started)
            if remaining <= 0:
                raise ProofError("process_timeout")
            try:
                data = chunks.get(timeout=remaining)
            except queue.Empty:
                raise ProofError("process_timeout") from None
            if data is None:
                break
            if len(output) + len(data) > limit:
                raise ProofError("output_bound")
            output.extend(data)
        remaining = timeout - (time.monotonic() - started)
        if remaining <= 0:
            raise ProofError("process_timeout")
        returncode = process.wait(timeout=remaining)
        return bytes(output), returncode
    except ProofError:
        raise
    except subprocess.TimeoutExpired:
        raise ProofError("process_timeout") from None
    except (OSError, subprocess.SubprocessError, ValueError):
        raise ProofError("process_unavailable") from None
    finally:
        stopped.set()
        cleanup_error = None
        try:
            if process is not None:
                # close_fds + redirected stdio restrict inherited handles in Popen.
                # The Windows child starts suspended, so assignment denial never
                # permits it to execute or create an uncontained descendant.
                if job is not None:
                    job.terminate()
                    job.close()  # Kill-on-close also applies if termination failed.
                elif sys.platform != "win32":
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=REAP_TIMEOUT)
        except (OSError, subprocess.SubprocessError):
            cleanup_error = "owned_process_cleanup_failed"
        finally:
            if job is not None:
                job.close()
        if reader is not None:
            reader.join(timeout=1)
        if reader is not None and reader.is_alive():
            # Closing a FileIO while another thread reads it can block forever.
            cleanup_error = "owned_output_reap_timeout"
        elif process is not None and process.stdout is not None:
            try:
                process.stdout.close()
            except OSError:
                cleanup_error = "owned_output_cleanup_failed"
        if cleanup_error is not None:
            raise ProofError(cleanup_error) from None


def _cargo_home(environment: dict[str, str]) -> Path:
    value = environment.get("CARGO_HOME")
    if value:
        home = Path(value)
        return home if home.is_absolute() else ROOT / home
    # Never combine a frozen child environment with ambient Path.home().
    value = environment.get("USERPROFILE" if sys.platform == "win32" else "HOME")
    if not value or not Path(value).is_absolute():
        raise ProofError("unreviewed_cargo_configuration")
    return Path(value) / ".cargo"


def _check_cargo_config(environment: dict[str, str]) -> None:
    """Refuse ambient execution policy without opening config or credential files."""
    try:
        home = _cargo_home(environment)
        directories = {home, *(parent / ".cargo" for parent in (ROOT, *ROOT.parents))}
        for directory in directories:
            for name in ("config", "config.toml"):
                try:
                    (directory / name).lstat()
                except FileNotFoundError:
                    continue
                # A file, directory, or dangling link is unreviewed policy.
                raise ProofError("unreviewed_cargo_configuration")
    except (OSError, RuntimeError, ValueError):
        raise ProofError("unreviewed_cargo_configuration") from None


def _reviewed_environment(environment: dict[str, str] | None = None) -> dict[str, str]:
    # Freeze once: late changes to the caller's environment cannot select a runner.
    snapshot = dict(os.environ if environment is None else environment)
    if snapshot.get("RUSTUP_TOOLCHAIN", PIN) != PIN or any(snapshot.get(name) for name in (
            "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")):
        raise ProofError("toolchain_not_pinned")
    if any(snapshot.get(name) for name in ("RUSTFLAGS", "RUSTDOCFLAGS", "RUSTDOC", "RUSTC_BOOTSTRAP")):
        raise ProofError("unreviewed_compilation_environment")
    # CARGO_HOME only chooses the offline cache; both its config names are checked.
    # Reject every other Cargo setting, including target/cfg runners and flags.
    if any(value and name.upper().startswith("CARGO_") and name.upper() != "CARGO_HOME"
           for name, value in snapshot.items()):
        raise ProofError("unreviewed_cargo_environment")
    # Cargo and every boundary check receive the same explicit absolute selection.
    snapshot["CARGO_HOME"] = str(_cargo_home(snapshot))
    _check_cargo_config(snapshot)
    return snapshot


def pinned_tools(environment: dict[str, str] | None = None) -> dict[str, str]:
    # Neither rustup which nor direct installed binaries acquire a missing toolchain.
    environment = _reviewed_environment(environment)
    try:
        with (ROOT / "rust-toolchain.toml").open("rb") as source:
            raw = source.read(16385)
        if len(raw) > 16384 or tomllib.loads(raw.decode("utf-8"))["toolchain"]["channel"] != PIN:
            raise ProofError("toolchain_not_pinned")
    except (OSError, UnicodeError, KeyError, TypeError, tomllib.TOMLDecodeError):
        raise ProofError("toolchain_not_pinned") from None
    # Search the frozen PATH for the exact native filename. Ambient PATH/PATHEXT
    # changes must not alter lookup, and relative PATH entries are not admitted.
    filename = "rustup.exe" if sys.platform == "win32" else "rustup"
    rustup = next((str(candidate) for directory in os.get_exec_path(environment)
                   if (candidate := Path(directory) / filename).is_absolute()
                   and candidate.is_file() and os.access(candidate, os.X_OK)), None)
    if rustup is None:
        raise ProofError("toolchain_unavailable")
    tools = {}
    for name in ("cargo", "rustc"):
        _check_cargo_config(environment)
        output, code = capture([rustup, "which", "--toolchain", PIN, name], cwd=ROOT,
                               env=environment, timeout=10, limit=16384)
        try:
            lines = output.decode("utf-8", errors="strict").splitlines()
            path = Path(lines[0]) if len(lines) == 1 else None
            if code != 0 or path is None or not path.is_absolute() or not path.is_file():
                raise ProofError("toolchain_unavailable")
        except UnicodeError:
            raise ProofError("toolchain_unavailable") from None
        tools[name] = str(path)
    for name in ("cargo", "rustc"):
        _check_cargo_config(environment)
        output, code = capture([tools[name], "--version"], cwd=ROOT, env=environment,
                               timeout=10, limit=16384)
        try:
            version = output.decode("utf-8", errors="strict").strip()
        except UnicodeError:
            raise ProofError("toolchain_not_pinned") from None
        if code != 0 or not re.fullmatch(rf"{name} {re.escape(PIN)} \([a-zA-Z0-9 -]+\)", version):
            raise ProofError("toolchain_not_pinned")
        tools[name + "_version"] = version
    return tools


def run_command(argv: list[str]) -> dict:
    choices = [(profile, package, case) for profile in ("debug", "release")
               for package, case in CASES if argv == cargo_command(profile, case)]
    if len(choices) != 1:
        raise ProofError("unreviewed_invocation")
    profile, package, case = choices[0]
    environment = _reviewed_environment()
    tools = pinned_tools(environment)
    environment.update(RUSTUP_TOOLCHAIN=PIN, RUSTC=tools["rustc"])
    _check_cargo_config(environment)
    output, code = capture([tools["cargo"]] + argv[1:], cwd=ROOT, env=environment)
    # Cooperative-host drift checks bracket the invocation; they do not lock the
    # host filesystem or detect a config inserted and removed during execution.
    _check_cargo_config(environment)
    rows = verify(output, code, case)
    return {"profile": profile, "package": package, "inner_case": case, "result": "passed",
            "named_passes": 1, "toolchain": PIN, "cargo_version": tools["cargo_version"],
            "rustc_version": tools["rustc_version"], **rows,
            "evidence_kind": ("portable_bootstrap" if case == BOOTSTRAP_CASE else
                              "pure_slot_policy" if case in SLOT_CASES else "native_case"),
            "desktop_qualification": "not_established"}


def run_case(profile: str, case: str = BOOTSTRAP_CASE) -> dict:
    return run_command(cargo_command(profile, case))
