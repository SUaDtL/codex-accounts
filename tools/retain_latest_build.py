"""Retain one successful CI build; never download or execute a build artifact."""
from __future__ import annotations

import json
import os
import re
import sys
import urllib.request

PREFIX = "codex-accounts-windows-x64-"


def owner_main_build(run: dict, repository: str) -> bool:
    return (run.get("head_repository", {}).get("full_name") == repository
            and run.get("head_branch") == "main"
            and run.get("event") in {"push", "workflow_dispatch"})


def artifact_key(artifact: dict) -> tuple[int, int] | None:
    match = re.fullmatch(PREFIX + r"([0-9]+)-([0-9]+)", artifact.get("name", ""))
    run_id = artifact.get("workflow_run", {}).get("id")
    return (run_id, int(match[2])) if match and type(run_id) is int else None


def deletion_ids(artifacts: list[dict], runs: dict[tuple[int, int], dict], workflow_id: int) -> list[int]:
    completed = []
    for artifact in artifacts:
        run = runs.get(artifact_key(artifact))
        if not run or run.get("workflow_id") != workflow_id or run.get("status") != "completed":
            continue
        if artifact["name"] != f"{PREFIX}{run['run_number']}-{run['run_attempt']}":
            continue
        completed.append((artifact, run))
    successful = [(artifact, run) for artifact, run in completed
                  if run.get("owner_build") is True and run.get("conclusion") == "success" and not artifact.get("expired", True)]
    keep = max(successful, key=lambda pair: (pair[1]["run_number"], pair[1]["run_attempt"], pair[0]["id"]), default=None)
    keep_id = keep[0]["id"] if keep else None
    return [artifact["id"] for artifact, run in completed
            if artifact["id"] != keep_id and (keep is not None or run.get("owner_build") is not True or run.get("conclusion") != "success")]


def main(apply: bool = False) -> None:
    repository = os.environ["GITHUB_REPOSITORY"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid repository identity")
    base = "https://api.github.com/repos/" + repository

    def request(path: str, method: str = "GET"):
        req = urllib.request.Request(base + path, method=method, headers={
            "Authorization": "Bearer " + os.environ["GH_TOKEN"],
            "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"})
        with urllib.request.urlopen(req, timeout=30) as response:
            return json.load(response) if method == "GET" else None

    workflow_id = request("/actions/workflows/ci.yml")["id"]
    artifacts = []
    for page in range(1, 1001):
        items = request(f"/actions/artifacts?per_page=100&page={page}")["artifacts"]
        artifacts.extend(items)
        if len(items) < 100:
            break
    else:
        raise ValueError("Artifact inventory exceeded the bounded pagination limit")
    keys = {key for artifact in artifacts if (key := artifact_key(artifact)) is not None}
    runs = {}
    for identifier, attempt in keys:
        run = request(f"/actions/runs/{identifier}/attempts/{attempt}")
        # PR, branch and fork builds cannot replace the owner's last good main build.
        run["owner_build"] = owner_main_build(run, repository)
        runs[(identifier, attempt)] = run
    candidates = deletion_ids(artifacts, runs, workflow_id)
    print(json.dumps({"mode": "apply" if apply else "preview", "obsolete_build_artifacts": candidates}))
    if apply:
        for identifier in candidates:
            if type(identifier) is not int or identifier <= 0:
                raise ValueError("Invalid artifact ID")
            request(f"/actions/artifacts/{identifier}", "DELETE")


if __name__ == "__main__":
    if sys.argv[1:] not in ([], ["--apply"]):
        raise SystemExit("Use no arguments for preview or --apply for retention")
    main(sys.argv[1:] == ["--apply"])
