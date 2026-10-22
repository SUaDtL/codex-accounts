"""Acquire reviewed scanner binaries by immutable release URL and SHA-256."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import platform
import shutil
import sys
import tarfile
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def install(name: str) -> Path:
    config = json.loads((ROOT / "security/tools.json").read_text(encoding="utf-8"))
    host = "windows" if platform.system() == "Windows" else "linux"
    if platform.machine().lower() not in {"amd64", "x86_64"} or platform.system() not in {"Windows", "Linux"}:
        raise ValueError("Scanner installer supports x64 Windows and Linux only")
    tool = config[name]
    asset = tool[host]
    directory = ROOT / ".ci-tools" / name / tool["version"]
    directory.mkdir(parents=True, exist_ok=True)
    executable = directory / (name + (".exe" if host == "windows" else ""))
    # Verify the downloaded payload on every invocation, including cache hits.
    archive = directory / asset["name"]
    if not archive.is_file():
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=directory, delete=False) as stream:
                temporary = Path(stream.name)
                with urllib.request.urlopen(asset["url"], timeout=60) as response:
                    shutil.copyfileobj(response, stream)
            temporary.replace(archive)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != asset["sha256"]:
        raise ValueError("Scanner download checksum mismatch")
    if archive.suffix == ".gz":
        with tarfile.open(archive) as bundle:
            member = bundle.getmember(name)
            if not member.isfile():
                raise ValueError("Scanner archive member is not a regular file")
            with bundle.extractfile(member) as source, executable.open("wb") as target:
                shutil.copyfileobj(source, target)
    elif archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            executable.write_bytes(bundle.read(name + ".exe"))
    elif archive != executable:
        shutil.copyfile(archive, executable)
    executable.chmod(0o755)
    return executable


if __name__ == "__main__":
    print(install(sys.argv[1]))
