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
FAULTS = {b'AccessDenied', b'Disappeared', b'Incomplete', b'Changed', b'Bound',
          b'QuitUnavailable', b'Timeout', b'HelperStuck', b'QualificationMissing'}


def failure_categories(output: bytes) -> list[str]:
    # Match only the fixed enum after Rust's unwrap diagnostic, never arbitrary
    # panic text. Unknown values and canaries are not exported.
    values = re.findall(rb'called `Result::unwrap\(\)` on an `Err` value: ([A-Za-z]+)(?:\r?\n|$)', output[:MAX_OUTPUT])
    return sorted({value.decode('ascii') for value in values if value in FAULTS})


def verify(output: bytes, returncode: int, cases: tuple[str, ...] = CASES) -> None:
    if returncode != 0 or len(output) > MAX_OUTPUT:
        raise ValueError('Native execution failed or exceeded output bound')
    text = output.decode('utf-8', errors='strict')
    rows = re.findall(r'^test ([a-zA-Z0-9_:]+) \.\.\. (.+)$', text, re.M)
    expected = {PREFIX + name for name in cases}
    names = [name for name, _ in rows]
    if (len(names) != len(expected) or set(names) != expected
            or any(result.strip() != 'ok' for _, result in rows)):
        raise ValueError('Required native case missing, duplicated, ignored or unsuccessful')
    summaries = re.findall(r'^test result: (.+)$', text, re.M)
    pattern = (rf'ok\. {len(cases)} passed; 0 failed; 0 ignored; 0 measured; '
               r'\d+ filtered out; finished in [0-9.]+s\s*')
    if len(summaries) != 1 or not re.fullmatch(pattern, summaries[0]):
        raise ValueError('Native result summary does not prove complete execution')


def run() -> None:
    if platform.system() != 'Windows' or platform.machine().lower() not in {'amd64', 'x86_64'}:
        raise ValueError('Windows x64 is required; a portable skip is not a pass')
    failed = False
    for profile in ('debug', 'release'):
        for case in CASES:
            command = ['cargo', 'test', '-p', 'codex-accounts-vault-storage', '--lib']
            if profile == 'release':
                command += ['--release']
            command += ['--locked', '--offline', PREFIX + case, '--', '--exact',
                        '--test-threads=1', '--format=pretty', '--nocapture']
            # Separate processes preserve other case evidence after a native abort.
            # Raw output stays in a temporary local file, never an uploaded log.
            output_bytes = b''
            code = None
            try:
                with tempfile.TemporaryFile() as output:
                    result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=output,
                                            timeout=180, check=False)
                    code = result.returncode
                    output.seek(0)
                    output_bytes = output.read(MAX_OUTPUT + 1)
                    verify(output_bytes, code, (case,))
                passed = True
            except (OSError, ValueError, subprocess.SubprocessError):
                passed = False
                failed = True
            lines = re.findall(rb'lifecycle_tests\.rs:(\d{1,5}):', output_bytes)
            print(json.dumps({'profile': profile, 'native_case': case,
                              'result': 'passed' if passed else 'failed',
                              'failure_source_lines': [] if passed else [int(n) for n in lines[:8]],
                              'failure_categories': [] if passed else failure_categories(output_bytes),
                              'desktop_qualification': 'not_established'}), flush=True)
    if failed:
        raise ValueError('One or more required native cases did not pass')


if __name__ == '__main__':
    try:
        run()
    except (OSError, ValueError, subprocess.SubprocessError):
        raise SystemExit('Named native execution did not pass; inspect the workspace test step') from None
