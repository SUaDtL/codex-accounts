"""Run and verify the named CA-04B Windows cases; zero filtered tests is not evidence."""
from __future__ import annotations

import json
import platform
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PREFIX = 'native::lifecycle::tests::'
CASES = (
    'ca04b_home_mutex_unicode_alias_and_recursive_contention',
    'ca04b_home_mutex_process_contention_and_exit_release',
    'ca04b_home_replacement_and_non_directory_paths_refuse',
    'ca04b_system_owned_directory_is_not_a_current_user_home',
    'ca04b_home_junction_root_and_ancestor_refuse',
    'ca04b_process_start_identity_and_exit_are_handle_bound',
    'ca04b_normal_quit_requires_actual_process_exit',
    'ca04b_refused_normal_quit_never_terminates_observed_process',
    'ca04b_owned_job_waits_for_late_descendant_and_final_write',
    'ca04b_owned_timeout_waits_after_termination_and_protects_unrelated_child',
    'ca04b_snapshot_and_known_descendants_never_become_home_authority',
)
MAX_OUTPUT = 1024 * 1024


def verify(output: bytes, returncode: int) -> None:
    if returncode != 0 or len(output) > MAX_OUTPUT:
        raise ValueError('Native execution failed or exceeded output bound')
    text = output.decode('utf-8', errors='strict')
    rows = re.findall(r'^test ([a-zA-Z0-9_:]+) \.\.\. (.+)$', text, re.M)
    expected = {PREFIX + name for name in CASES}
    names = [name for name, _ in rows]
    if (len(names) != len(expected) or set(names) != expected
            or any(result.strip() != 'ok' for _, result in rows)):
        raise ValueError('Required native case missing, duplicated, ignored or unsuccessful')
    summaries = re.findall(r'^test result: (.+)$', text, re.M)
    pattern = (rf'ok\. {len(CASES)} passed; 0 failed; 0 ignored; 0 measured; '
               r'\d+ filtered out; finished in [0-9.]+s\s*')
    if len(summaries) != 1 or not re.fullmatch(pattern, summaries[0]):
        raise ValueError('Native result summary does not prove complete execution')


def run() -> None:
    if platform.system() != 'Windows' or platform.machine().lower() not in {'amd64', 'x86_64'}:
        raise ValueError('Windows x64 is required; a portable skip is not a pass')
    for profile in ('debug', 'release'):
        command = ['cargo', 'test', '-p', 'codex-accounts-vault-storage', '--lib']
        if profile == 'release':
            command += ['--release']
        command += ['--locked', '--offline', 'ca04b_', '--', '--test-threads=1', '--format=pretty']
        # Raw test output stays in a temporary local file and is never uploaded or
        # echoed by this verifier. The ordinary workspace step supplies diagnostics.
        with tempfile.TemporaryFile() as output:
            result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=output,
                                    timeout=180, check=False)
            output.seek(0)
            verify(output.read(MAX_OUTPUT + 1), result.returncode)
        print(json.dumps({'profile': profile, 'executed_native_cases': list(CASES),
                          'result': 'passed', 'desktop_qualification': 'not_established'}), flush=True)


if __name__ == '__main__':
    try:
        run()
    except (OSError, ValueError, subprocess.SubprocessError):
        raise SystemExit('Named native execution did not pass; inspect the workspace test step') from None
