"""Run blocking CVE/secrets/configuration or source scans with fresh reports."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import base64
import tempfile

from security_tools import install

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / "reports/security"


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
    binary = str(install("trivy" if scanner == "package" else scanner))
    REPORTS.mkdir(parents=True, exist_ok=True)
    report = REPORTS / (scanner + ".sarif")
    report.unlink(missing_ok=True)
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
        environment = {**os.environ, "SEMGREP_SEND_METRICS": "off", "SEMGREP_ENABLE_VERSION_CHECK": "0"}
        tests = subprocess.run([binary, "scan", "--test", "--config", "security/opengrep", "security/opengrep"],
                               cwd=ROOT, env=environment, timeout=180, check=False)
        if tests.returncode:
            return tests.returncode
        command = [binary, "scan", "--config", "security/opengrep", "--error", "--strict", "--disable-nosem",
                   "--jobs", "2", "--timeout", "30", "--no-git-ignore", "--sarif-output", str(report),
                   "--exclude", "security/opengrep", "--exclude", ".review-evidence", "--exclude", "target",
                   "--exclude", "node_modules", "--exclude", ".ci-tools", "--exclude", "dist", "--exclude", "reports",
                   "crates", "tools"]
        result = subprocess.run(command, cwd=ROOT, env=environment, timeout=600, check=False)
    if not report.is_file():
        raise ValueError("Scanner did not produce a current report")
    data = json.loads(report.read_text(encoding="utf-8"))
    if not data.get("runs"):
        raise ValueError("Scanner report contains no completed run")
    return result.returncode or (secret_result.returncode if scanner == "trivy" else 0)


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1]))
