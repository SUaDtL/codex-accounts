"""Run blocking CVE/secrets/configuration or source scans with fresh reports."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import base64
import hashlib
import tempfile
import time

from security_tools import install

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / "reports/security"


def opengrep_health(data: dict, expected: set[str]) -> dict:
    if not isinstance(data, dict) or not isinstance(data.get("results"), list):
        raise ValueError("OpenGrep result schema is invalid")
    if data.get("errors") != []:
        raise ValueError("OpenGrep reported incomplete analysis")
    paths = data.get("paths")
    scanned = paths.get("scanned") if isinstance(paths, dict) else None
    if not isinstance(scanned, list) or not all(isinstance(path, str) for path in scanned):
        raise ValueError("OpenGrep target inventory is unavailable")
    actual = {path.replace("\\", "/") for path in scanned}
    if not expected or not expected.issubset(actual):
        raise ValueError("OpenGrep omitted expected source files: " + ", ".join(sorted(expected - actual)))
    return {"scanned_files": len(actual), "expected_source_files": len(expected),
            "findings": len(data["results"]), "errors": 0,
            "languages": {extension: sum(path.endswith(extension) for path in expected)
                          for extension in (".rs", ".js", ".py")}}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_secret_detection(binary: str) -> None:
    """An unusable synthetic JWT in Markdown must fail the all-severity secret scan."""
    def part(value: dict) -> str:
        return base64.urlsafe_b64encode(json.dumps(value).encode()).decode().rstrip("=")
    token = part({"alg": "HS256", "typ": "JWT"}) + "." + part({"sub": "ci-fixture", "iat": 1700000000}) + "." + "a" * 43
    with tempfile.TemporaryDirectory(prefix="scanner-fixture-") as folder:
        root = Path(folder)
        (root / "note.md").write_text(token, encoding="utf-8")
        result = subprocess.run([binary, "fs", "--scanners", "secret", "--secret-config",
                                 str(ROOT / "security/trivy-secret.yaml"), "--exit-code", "1",
                                 "--format", "json", "--output", str(root / "result.json"), str(root)],
                                cwd=ROOT, timeout=60, capture_output=True, check=False)
        report = root / "result.json"
        data = json.loads(report.read_text(encoding="utf-8")) if report.is_file() else {}
        if result.returncode != 1 or not any(secret.get("RuleID") == "jwt-token"
                for item in data.get("Results", []) for secret in item.get("Secrets", [])):
            raise ValueError("Secret scanner did not reject the synthetic Markdown JWT")
    print("Secret detection fixture: passed (synthetic JWT, Markdown, all severities)")


def run(scanner: str) -> int:
    if scanner not in {"trivy", "opengrep", "package"}:
        raise ValueError("Unknown scanner")
    REPORTS.mkdir(parents=True, exist_ok=True)
    report = REPORTS / (scanner + ".sarif")
    report.unlink(missing_ok=True)
    if scanner == "opengrep":
        json_report = REPORTS / "opengrep.json"
        metadata = REPORTS / "opengrep-meta.json"
        json_report.unlink(missing_ok=True)
        metadata.write_text('{"status":"incomplete"}\n', encoding="utf-8")
    binary = str(install("trivy" if scanner == "package" else scanner))
    if scanner == "package":
        result = subprocess.run([binary, "sbom", "--severity", "HIGH,CRITICAL", "--exit-code", "1",
                                 "--format", "sarif", "--output", str(report), "dist/windows-x64/SBOM.cdx.json"],
                                cwd=ROOT, timeout=600, check=False)
    elif scanner == "trivy":
        verify_secret_detection(binary)
        secrets = REPORTS / "secrets.sarif"
        secrets.unlink(missing_ok=True)
        secret_result = subprocess.run([binary, "fs", "--scanners", "secret", "--secret-config",
            "security/trivy-secret.yaml", "--exit-code", "1", "--format", "sarif", "--output", str(secrets),
            "--skip-dirs", ".git,target,.review-evidence,.ci-tools,dist,reports,node_modules", "."],
            cwd=ROOT, timeout=600, check=False)
        if not secrets.is_file() or not json.loads(secrets.read_text(encoding="utf-8")).get("runs"):
            raise ValueError("Secret scanner did not produce a current report")
        command = [binary, "fs", "--scanners", "vuln,misconfig", "--severity", "HIGH,CRITICAL",
                   "--include-dev-deps", "--exit-code", "1", "--format", "sarif", "--output", str(report),
                   "--skip-dirs", ".git,target,.review-evidence,.ci-tools,dist,reports,node_modules", "."]
        result = subprocess.run(command, cwd=ROOT, timeout=600, check=False)
    else:
        expected = {path.relative_to(ROOT).as_posix() for root in ("crates", "tools")
                    for path in (ROOT / root).rglob("*") if path.is_file() and path.suffix in {".rs", ".js", ".py"}}
        started = time.monotonic()
        with tempfile.TemporaryDirectory(prefix="opengrep-runtime-") as runtime:
            environment = {**os.environ, "SEMGREP_SEND_METRICS": "off", "SEMGREP_ENABLE_VERSION_CHECK": "0",
                           "XDG_CACHE_HOME": str(Path(runtime) / "cache"),
                           "SEMGREP_SETTINGS_FILE": str(Path(runtime) / "settings.yml"),
                           "SEMGREP_LOG_FILE": str(Path(runtime) / "opengrep.log")}
            tests = subprocess.run([binary, "scan", "--test", "--config", "security/opengrep", "security/opengrep"],
                                   cwd=ROOT, env=environment, timeout=180, check=False)
            if tests.returncode:
                return tests.returncode
            command = [binary, "scan", "--config", "security/opengrep", "--error", "--strict", "--disable-nosem",
                       "--jobs", "2", "--timeout", "30", "--no-git-ignore", "--sarif-output", str(report),
                       "--json-output", str(json_report), "crates", "tools"]
            result = subprocess.run(command, cwd=ROOT, env=environment, timeout=600, check=False)
    if not report.is_file():
        raise ValueError("Scanner did not produce a current report")
    data = json.loads(report.read_text(encoding="utf-8"))
    if not data.get("runs"):
        raise ValueError("Scanner report contains no completed run")
    if scanner == "opengrep":
        if data.get("version") != "2.1.0" or any(
                not isinstance(run, dict) or not isinstance(run.get("results"), list) for run in data["runs"]):
            raise ValueError("OpenGrep SARIF schema is invalid")
        health = opengrep_health(json.loads(json_report.read_text(encoding="utf-8")), expected)
        if result.returncode not in (0, 1) or (result.returncode == 1 and not health["findings"]):
            raise ValueError("OpenGrep did not complete successfully")
        metadata.write_text(json.dumps({**health, "status": "complete", "analysis": "structural",
            "source_commit": os.environ.get("GITHUB_SHA"), "engine_sha256": digest(Path(binary)),
            "rules_sha256": {path.name: digest(path) for path in sorted((ROOT / "security/opengrep").glob("*.yml"))},
            "json_sha256": digest(json_report), "sarif_sha256": digest(report),
            "elapsed_seconds": round(time.monotonic() - started, 3)}, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(health))
        return 1 if health["findings"] else 0
    return result.returncode or (secret_result.returncode if scanner == "trivy" else 0)


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1]))
