"""Package the built pre-1.0 application with a lockfile-bound dependency inventory."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sbom(root: Path, version: str) -> dict:
    packages = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))["package"]
    components = []
    for package in packages:
        if "source" not in package:
            continue
        component = {"type": "library", "name": package["name"], "version": package["version"],
                     "purl": f"pkg:cargo/{package['name']}@{package['version']}"}
        if "checksum" in package:
            component["hashes"] = [{"alg": "SHA-256", "content": package["checksum"]}]
        components.append(component)
    return {"bomFormat": "CycloneDX", "specVersion": "1.6", "version": 1,
            "metadata": {"component": {"type": "application", "name": "Codex Accounts", "version": version},
                         "properties": [{"name": "codex-accounts:inventory-scope", "value": "Cargo.lock including target and build dependencies"},
                                        {"name": "codex-accounts:lock-sha256", "value": sha(root / "Cargo.lock")}]},
            "components": components}


def package(executable: Path, destination: Path) -> None:
    if not executable.is_file():
        raise ValueError("A built Windows executable is required")
    with executable.open("rb") as stream:
        if stream.read(2) != b"MZ":
            raise ValueError("A built Windows executable is required")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    if not version.startswith("0."):
        raise ValueError("This package lane is for pre-1.0 versions")
    destination.mkdir(parents=True, exist_ok=False)
    payload = destination / "codex-accounts-desktop.exe"
    shutil.copy2(executable, payload)
    shutil.copy2(ROOT / "Cargo.lock", destination / "Cargo.lock")
    (destination / "SBOM.cdx.json").write_text(json.dumps(sbom(ROOT, version), indent=2) + "\n", encoding="utf-8")
    metadata = {"application": "Codex Accounts", "version": version, "platform": "windows-x86_64",
                "kind": "unsigned-ci-artifact", "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "built_at": datetime.now(timezone.utc).isoformat(), "payload_sha256": sha(payload),
                "lock_sha256": sha(ROOT / "Cargo.lock"), "workflow_run": os.environ.get("GITHUB_RUN_ID"),
                "publication": "not-a-release"}
    (destination / "BUILD.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    (destination / "README.txt").write_text(
        "Codex Accounts " + version + " (pre-1.0)\n\n"
        "Windows x64, official Codex Desktop and WebView2 required.\n"
        "Close the older Accounts window before launching this executable.\n"
        "Existing saved accounts carry over. Keep Codex closed for account changes.\n"
        "This is an unsigned CI build, not a signed release or installer.\n"
        "Verify SHA256SUMS and consult BUILD.json for the source revision.\n"
        "SBOM.cdx.json inventories Cargo.lock, including build and other-target dependencies.\n"
        "Documentation: https://github.com/SUaDtL/codex-accounts\n", encoding="utf-8")
    (destination / "SHA256SUMS").write_text("".join(f"{sha(p)}  {p.name}\n" for p in sorted(destination.iterdir()) if p.is_file()), encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=ROOT / "target/release/codex-accounts-desktop.exe")
    parser.add_argument("--out", type=Path, default=ROOT / "dist/windows-x64")
    arguments = parser.parse_args()
    package(arguments.exe, arguments.out)
