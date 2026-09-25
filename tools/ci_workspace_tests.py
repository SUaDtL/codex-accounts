"""Run each Windows behavior once, with compiled inventory proving the partition.

Native cases still run in independent processes via run_native_tests.py. The CI
aggregate requires BOTH this complement and that exact-name proof in both profiles.
No path-based job skip, cached success, test removal or product authority is added.
"""
from __future__ import annotations
import collections
import json
import platform
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
try:
    from . import run_native_tests as native
except ImportError:
    import run_native_tests as native

ROOT = Path(__file__).resolve().parents[1]
LIMIT = 1024 * 1024

def cargo_command(profile: str) -> list[str]:
    if profile not in {'debug', 'release'}:
        raise ValueError('Invalid test profile')
    command = ['cargo', 'test', '--workspace', '--all-targets', '--no-fail-fast', '--locked', '--offline']
    if profile == 'release':
        command += ['--release']
    return command


def selectors() -> list[str]:
    names = [prefix + case for prefix, case in native.inventory()]
    if not names or len(set(names)) != len(names):
        raise ValueError('Invalid native inventory')
    return ['--exact'] + [value for name in names for value in ['--skip', name]]


def listing(output: bytes, code: int) -> collections.Counter:
    if code != 0 or len(output) > LIMIT:
        raise ValueError('Compiled test inventory unavailable')
    rows = []
    for line in output.decode('utf-8', errors='strict').splitlines():
        match = re.fullmatch(r'([a-zA-Z0-9_:]+): (test|benchmark)', line)
        if match:
            rows.append(match.groups())
        elif line and not re.fullmatch(r'\d+ tests?, \d+ benchmarks?', line):
            raise ValueError('Unrecognized compiled test inventory')
    if not rows:
        raise ValueError('Empty compiled test inventory')
    return collections.Counter(rows)


def verify_partition(full: collections.Counter, remaining: collections.Counter) -> int:
    wanted = collections.Counter((prefix + case, 'test') for prefix, case in native.inventory())
    if not wanted or any(full[name] != 1 for name in wanted):
        raise ValueError('Native names must each exist exactly once in compiled workspace')
    if remaining != full - wanted:
        raise ValueError('Workspace partition removed an unassigned case or retained a duplicate')
    if not sum(remaining.values()):
        raise ValueError('Workspace complement unexpectedly empty')
    return sum(wanted.values())


def read_listing(command: list[str]) -> collections.Counter:
    with tempfile.TemporaryFile() as output:
        # Only stdout contains libtest inventory. Compiler diagnostics stay local;
        # any compilation failure is blocking, not an old binary or empty pass.
        with tempfile.TemporaryFile() as errors:
            result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=errors,
                                    timeout=300, check=False)
        output.seek(0)
        return listing(output.read(LIMIT + 1), result.returncode)


def run(profile: str) -> int:
    command = cargo_command(profile)
    started = time.monotonic()
    isolated = 0
    if platform.system() == 'Windows':
        if platform.machine().lower() not in {'amd64', 'x86_64'}:
            raise ValueError('Windows native lane requires x64')
        filters = selectors()
        full = read_listing(command + ['--', '--list', '--format=terse'])
        remaining = read_listing(command + ['--', '--list', '--format=terse'] + filters)
        isolated = verify_partition(full, remaining)
        print(json.dumps({'workspace_profile': profile, 'compiled_cases': sum(full.values()),
                          'workspace_cases': sum(remaining.values()), 'isolated_native_cases': isolated,
                          'partition': 'complete_and_disjoint'}), flush=True)
        command += ['--'] + filters
    # No retry: keep Cargo's exit code and --no-fail-fast behavior visible.
    result = subprocess.run(command, cwd=ROOT, check=False)
    print(json.dumps({'workspace_profile': profile, 'result': 'passed' if result.returncode == 0 else 'failed',
                      'isolated_native_cases_still_required': isolated,
                      'elapsed_seconds': round(time.monotonic() - started, 3)}), flush=True)
    return result.returncode


if __name__ == '__main__':
    try:
        if len(sys.argv) != 2:
            raise ValueError('Invalid invocation')
        raise SystemExit(run(sys.argv[1]))
    except (OSError, ValueError, subprocess.SubprocessError):
        raise SystemExit('Workspace execution or compiled partition proof failed') from None
