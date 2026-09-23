"""Read canonical embedded HTML model by symbol; never create an editable mirror."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from html.parser import HTMLParser
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "docs/local-codex-switcher-spec.html"


class ModelParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=False)
        self.capture = False
        self.parts: list[str] = []
        self.count = 0
        self.ids: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        data = dict(attrs)
        if data.get("id"):
            self.ids.append(str(data["id"]))
        if tag == "script" and data.get("id") == "artifact-model":
            self.capture = True
            self.count += 1

    def handle_endtag(self, tag: str) -> None:
        if tag == "script":
            self.capture = False

    def handle_data(self, data: str) -> None:
        if self.capture:
            self.parts.append(data)


def model(path: Path = SPEC) -> tuple[dict[str, Any], list[str]]:
    parser = ModelParser()
    parser.feed(path.read_text(encoding="utf-8"))
    if parser.count != 1:
        raise ValueError("Expected exactly one canonical artifact-model")
    return json.loads("".join(parser.parts)), parser.ids


def symbols(data: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for section in data["sections"]:
        result[section["id"]] = section
        for block in section["blocks"]:
            if block.get("type") == "requirement":
                result[block["id"]] = block
            if section["id"] == "SPEC-TESTS" and block.get("type") == "table":
                for test_id, text in block["rows"]:
                    result[test_id] = {"id": test_id, "text": text}
    return result


def check(root: Path = ROOT) -> dict[str, Any]:
    data, ids = model(root / "docs/local-codex-switcher-spec.html")
    if __package__:
        from . import spec_projection, spec_proposal, plan_projection
    else:
        import spec_projection
        import spec_proposal
        import plan_projection
    spec_projection.check((root / "docs/local-codex-switcher-spec.html").read_text(encoding="utf-8"), data)
    spec_proposal.check_candidate(root)
    plan_projection.check(root)
    table = symbols(data)
    requirements = {key for key in table if re.fullmatch(r"[FS]-\d{3}", key)}
    tests = {key for key in table if re.fullmatch(r"T-\d{2}", key)}
    if len(requirements) != 43 or len(tests) != 34:
        raise ValueError("Unexpected specification requirement/test topology")
    if len(ids) != len(set(ids)) or not set(table).issubset(ids):
        raise ValueError("Missing or duplicate HTML symbol anchors")
    for key in requirements:
        if not set(table[key]["tests"].split()).issubset(tests):
            raise ValueError("Broken requirement acceptance reference")
    manifest = json.loads((root / "docs/source-inputs.json").read_text())
    for entry in manifest["files"]:
        path = root / entry["path"]
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError("Source input changed; review provenance deliberately")
    catalog = json.loads((root / "compatibility/catalog.json").read_text())
    if catalog != {"schema": "codex-accounts/compatibility-catalog/v1", "qualified_records": []}:
        raise ValueError("Q0 bootstrap must have an empty compatibility catalog")
    return {"requirements": len(requirements), "acceptance_scenarios": len(tests),
            "qualified_records": 0, "source_integrity": "passed"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["list", "show", "check"])
    parser.add_argument("symbol", nargs="?")
    args = parser.parse_args()
    try:
        if args.command == "check":
            value: Any = check()
        else:
            data, _ = model()
            entries = symbols(data)
            if args.command == "list":
                value = [{"id": key, "title": item.get("title", "Acceptance scenario")}
                         for key, item in entries.items()]
            else:
                if args.symbol not in entries:
                    parser.error("Unknown symbol; use list")
                value = entries[args.symbol]
        print(json.dumps(value, ensure_ascii=False, indent=2))
        return 0
    except (ValueError, OSError, KeyError) as exc:
        # These errors concern repository-owned documentation, never credentials.
        parser.exit(1, f"Specification check failed: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
