"""Read-only Q0 candidate inventory; never an authorization or qualification.

No subprocess, network, auth-file access, or write operations. Native handle-based
path/ACL/publisher validation is not implemented; all reports remain unqualified.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import stat
import sys
import tomllib
from pathlib import Path
from typing import Any

MAX_CONFIG_BYTES = 1024 * 1024
MAX_EXECUTABLE_BYTES = 1024 * 1024 * 1024
ENV_NAMES = (
    "CODEX_HOME", "CODEX_CLI_PATH", "OPENAI_API_KEY", "OPENAI_BASE_URL",
    "CODEX_API_KEY", "NODE_OPTIONS", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES",
)
UNRESOLVED = (
    "official_package_and_publisher", "exact_desktop_and_runtime_versions",
    "effective_home_and_configuration_layers", "effective_managed_policy",
    "native_path_acl_and_race_safety", "complete_auth_resource_contract",
    "shared_home_process_discovery", "normal_quit_and_reopen",
    "version_specific_rpc_schema", "helper_side_effects_and_refresh_ownership",
    "actual_desktop_identity_observation", "owner_run_native_integration",
)


class InventoryError(Exception):
    """Contains only a fixed error code, never a path or raw operating-system error."""

    def __init__(self, code: str):
        super().__init__(code)
        self.code = code


def _safe_candidate(path: Path, *, directory: bool = False) -> os.stat_result:
    if not path.is_absolute() or ".." in path.parts:
        raise InventoryError("E_PATH_UNSAFE")
    # Preliminary read-only screening only. It is NOT the future native write API.
    for item in reversed((path, *path.parents)):
        info = item.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise InventoryError("E_PATH_UNSAFE")
    info = path.lstat()
    if directory:
        if not stat.S_ISDIR(info.st_mode):
            raise InventoryError("E_PATH_UNSAFE")
    elif not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise InventoryError("E_PATH_UNSAFE")
    return info


def _stamp(info: os.stat_result) -> tuple[int, int, int, int, int]:
    return info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_nlink


def _read_candidate(path: Path, limit: int, *, digest_only: bool) -> bytes | dict[str, Any]:
    # Refuse well-known auth filenames even if nominated as an executable.
    if path.name.casefold() in {"auth.json", "cap_sid"}:
        raise InventoryError("E_PATH_UNSAFE")
    before = _safe_candidate(path)
    if before.st_size > limit:
        raise InventoryError("E_INPUT_LIMIT")
    flags = (os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
             | getattr(os, "O_NONBLOCK", 0))
    fd = os.open(path, flags)
    try:
        opened = os.fstat(fd)
        if not stat.S_ISREG(opened.st_mode) or _stamp(opened) != _stamp(before):
            raise InventoryError("E_EXTERNAL_CHANGE")
        digest = hashlib.sha256()
        chunks: list[bytes] = []
        total = 0
        while True:
            chunk = os.read(fd, min(1024 * 1024, limit - total + 1))
            if not chunk:
                break
            total += len(chunk)
            if total > limit:
                raise InventoryError("E_INPUT_LIMIT")
            digest.update(chunk)
            if not digest_only:
                chunks.append(chunk)
        after = os.fstat(fd)
        current = _safe_candidate(path)
        if _stamp(before) != _stamp(after) or _stamp(after) != _stamp(current):
            raise InventoryError("E_EXTERNAL_CHANGE")
        if digest_only:
            return {"size_bytes": total, "sha256": digest.hexdigest(), "publisher": "unknown"}
        return b"".join(chunks)
    finally:
        os.close(fd)


def declared_backend(home: Path) -> dict[str, str]:
    _safe_candidate(home, directory=True)
    config = home / "config.toml"
    try:
        raw = _read_candidate(config, MAX_CONFIG_BYTES, digest_only=False)
    except FileNotFoundError:
        # Recheck the parent. A disappearing home is not a missing optional file.
        _safe_candidate(home, directory=True)
        return {"config_status": "absent", "declared_backend": "unknown"}
    assert isinstance(raw, bytes)
    try:
        parsed = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError, RecursionError):
        raise InventoryError("E_CONFIG_INVALID") from None
    value = parsed.get("cli_auth_credentials_store")
    known = {"file", "auto", "keyring", "ephemeral", "secrets"}
    backend = value if isinstance(value, str) and value in known else "unknown"
    return {"config_status": "present", "declared_backend": backend}


def inventory(desktop: Path, runtime: Path, home: Path, *, show_local_paths: bool = False) -> dict[str, Any]:
    desktop_info = _read_candidate(desktop, MAX_EXECUTABLE_BYTES, digest_only=True)
    runtime_info = _read_candidate(runtime, MAX_EXECUTABLE_BYTES, digest_only=True)
    config = declared_backend(home)
    report: dict[str, Any] = {
        "schema": "codex-accounts/q0-inventory/v1",
        "report_kind": "candidate_inventory_not_diagnostics",
        "qualified": False,
        "credential_mutation_enabled": False,
        "blocker": "E_COMPAT_UNKNOWN",
        "host": {"os": platform.system(), "architecture": platform.machine()},
        "desktop_candidate": desktop_info,
        "runtime_candidate": runtime_info,
        "candidate_home_config": config,
        "environment_flags_present": [name for name in ENV_NAMES if name in os.environ],
        "unresolved_evidence": list(UNRESOLVED),
    }
    if show_local_paths:
        report["local_display_only_do_not_share"] = {
            "desktop": str(desktop), "runtime": str(runtime), "candidate_home": str(home),
        }
    return report


class SafeParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        # argparse normally echoes arbitrary supplied arguments. Keep errors fixed.
        self.exit(2, "E_USAGE: supply explicit absolute candidate paths; see --help.\n")


def main(argv: list[str] | None = None) -> int:
    parser = SafeParser(description=__doc__)
    parser.add_argument("--desktop-file", type=Path, required=True)
    parser.add_argument("--runtime-file", type=Path, required=True)
    parser.add_argument("--codex-home", type=Path, required=True)
    parser.add_argument("--show-local-paths", action="store_true", help="Local display only; do not upload output.")
    args = parser.parse_args(argv)
    try:
        report = inventory(args.desktop_file, args.runtime_file, args.codex_home,
                           show_local_paths=args.show_local_paths)
    except InventoryError as exc:
        report = {"error": exc.code, "qualified": False, "credential_mutation_enabled": False}
    except PermissionError:
        report = {"error": "E_ACCESS_DENIED", "qualified": False, "credential_mutation_enabled": False}
    except OSError:
        report = {"error": "E_INVENTORY_IO", "qualified": False, "credential_mutation_enabled": False}
    print(json.dumps(report, ensure_ascii=True, indent=2))
    return 2 if "error" in report else 0


if __name__ == "__main__":
    sys.exit(main())
