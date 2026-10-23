"""Enforce product boundaries that must not drift into application code."""
from __future__ import annotations

from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def _text_files(root: Path):
    crates = root / "crates"
    for path in crates.rglob("*"):
        if path.is_file() and path.suffix in {".rs", ".toml"}:
            yield path


def check(root: Path = ROOT) -> None:
    violations: list[str] = []
    files = list(_text_files(root))
    for path in files:
        text = path.read_text(encoding="utf-8")
        relative = path.relative_to(root).as_posix()
        if relative.endswith("native/lifecycle/process.rs") and "TerminateProcess" in text:
            violations.append(f"{relative}: force termination")
        if re.search(r"(?i)tauri[-_]plugin[-_]updater|check_update|install_update", text):
            violations.append(f"{relative}: application updater")

    for crate in ["vault", "vault-crypto", "vault-storage"]:
        manifest = root / "crates" / crate / "Cargo.toml"
        if not manifest.is_file():
            continue
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        dependency_tables = [
            parsed.get("dependencies", {}),
            *[
                target.get("dependencies", {})
                for target in parsed.get("target", {}).values()
                if isinstance(target, dict)
            ],
        ]
        for dependencies in dependency_tables:
            forbidden = {"reqwest", "hyper", "oauth2"}.intersection(dependencies)
            for name in sorted(forbidden):
                violations.append(f"{manifest.relative_to(root).as_posix()}: {name} dependency")

    for manifest in (root / "crates").glob("*/Cargo.toml"):
        parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        package = parsed.get("package", {})
        name = str(package.get("name", ""))
        if any(term in name for term in ("credential-cli", "vault-cli")):
            violations.append(f"{manifest.relative_to(root).as_posix()}: credential CLI")

    if violations:
        raise ValueError("Prohibited product surface found: " + "; ".join(violations))


if __name__ == "__main__":
    check()
    print("Product prohibitions: passed")
