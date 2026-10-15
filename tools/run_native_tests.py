"""Run reviewed Windows native cases; zero filtered tests is not evidence."""
from __future__ import annotations

import json
import platform
import re
import subprocess
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
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
TARGET_PREFIX = 'native::lifecycle::target_tests::'
TARGET_CASES = (
    'ca04c_exact_bytes_absence_and_empty_are_distinct',
    'ca04c_expected_old_foreign_stage_and_invalid_slot_refuse',
    'ca04c_sharing_hardlink_and_namespace_changes_refuse',
    'ca04c_cleanup_requires_registered_authenticated_bytes',
    'ca04c_journal_preserves_newest_source_and_committed_target',
    'ca04c_midpoint_reopen_restores_with_separate_failures',
    'ca04c_external_change_and_stale_parent_never_overwrite',
    'ca04c_cleanup_failure_blocks_until_verified_retry',
    'ca04c_changed_acl_and_oversized_input_refuse',
    'ca04c_owned_helper_generation_survives_source_restoration',
    'ca04c_process_restart_forward_boundaries',
    'ca04c_process_restart_restoration_boundaries',
)
REPAIR_CASES = (
    'ca04d_torn_stage_archived_before_explicit_restoration',
    'ca04d_staging_repair_refuses_writer_links_and_unregistered_files',
    'ca04d_staging_archive_failure_preserves_plaintext_and_live_state',
    'ca04d_repair_process_restart_boundaries',
)
STORAGE_PREFIX = 'native::tests::'
STORAGE_CASES = (
    'native_profile_fixture_parent_is_protected_and_unsafe_parents_refuse',
    'native_fixture_final_check_refusal_preserves_owned_ancestor',
    'native_location_mutex_recursive_and_protected_object',
    'native_location_mutex_process_contention_release_and_abandonment',
    'native_location_read_only_probe_refuses_denial_and_wrong_type',
    'native_location_collision_and_replacement_refuse_before_access',
    'native_location_failure_releases_mutex_and_retains_root_lock',
    'native_layout_namespace_precedes_known_folder_resolution',
    'native_layout_valid_staged_initial_anchor_reconciles_without_key_regeneration',
    'native_layout_descriptor_drift_and_legacy_container_absence_are_lifetime_guards',
    'native_layout_creation_failures_and_declared_pending_registry_recovery',
    'native_layout_fresh_nested_round_trip_and_container_precedence',
    'native_layout_reserved_collision_and_empty_container_beside_legacy_preserve_bytes',
    'native_layout_existing_lock_is_read_only_and_lock_first',
    'native_layout_unrecognized_initialization_and_key_matrix_are_preserved',
    'native_layout_container_inventory_and_lifetime_changes_refuse_before_effects',
    'native_layout_authenticated_stages_and_torn_recovery_keep_real_methods_reachable',
    'native_layout_legacy_metadata_unsafe_grants_never_authorize_data_or_denial_fallback',
)
DISCOVERY_CASES = (
    ('native::tests::', 'native_registered_root_retains_internal_handles_without_outer_observation'),
    ('windows_reader::tests::', 'native_registered_metadata_observes_each_declared_image_without_sensitive_reads'),
    ('windows_reader::tests::', 'native_registered_metadata_refuses_manifest_mismatch_limits_and_external_paths'),
    ('windows_reader::tests::', 'native_registered_metadata_refuses_sharing_hardlinks_and_registration_drift'),
    ('windows_reader::tests::', 'native_registered_metadata_retains_manifest_root_and_images_through_reobservation'),
    ('windows_reader::tests::', 'native_registered_metadata_accepts_one_utf8_bom_and_refuses_invalid_prefixes'),
    ('windows_reader::tests::', 'native_registered_image_is_separate_redacted_and_retained_through_reobservation'),
    ('windows_reader::tests::', 'native_registered_image_refuses_invalid_leaf_and_registration_changes'),
    ('windows_reader::tests::', 'native_registered_image_rejects_junction_ancestor_without_target_read'),
)
WORKERS = 2

def inventory() -> tuple[tuple[str, str], ...]:
    return tuple([(PREFIX, case) for case in CASES]
                 + [(TARGET_PREFIX, case) for case in TARGET_CASES + REPAIR_CASES]
                 + [(STORAGE_PREFIX, case) for case in STORAGE_CASES]) + DISCOVERY_CASES

MAX_OUTPUT = 1024 * 1024
FAULTS = {b'AccessDenied', b'Disappeared', b'Incomplete', b'Changed', b'Bound',
          b'QuitUnavailable', b'Timeout', b'HelperStuck', b'QualificationMissing'}


def failure_categories(output: bytes) -> list[str]:
    # Match only the fixed enum after Rust's unwrap diagnostic, never arbitrary
    # panic text. Unknown values and canaries are not exported.
    values = re.findall(rb'called `Result::unwrap\(\)` on an `Err` value: ([A-Za-z]+)(?:\r?\n|$)', output[:MAX_OUTPUT])
    return sorted({value.decode('ascii') for value in values if value in FAULTS})


def verify(output: bytes, returncode: int, cases: tuple[str, ...] = CASES, *, prefix: str = PREFIX) -> None:
    if returncode != 0 or len(output) > MAX_OUTPUT:
        raise ValueError('Native execution failed or exceeded output bound')
    text = output.decode('utf-8', errors='strict')
    rows = re.findall(r'^test ([a-zA-Z0-9_:]+) \.\.\. (.+)$', text, re.M)
    expected = {prefix + name for name in cases}
    names = [name for name, _ in rows]
    if (len(names) != len(expected) or set(names) != expected
            or any(result.strip() != 'ok' for _, result in rows)):
        raise ValueError('Required native case missing, duplicated, ignored or unsuccessful')
    summaries = re.findall(r'^test result: (.+)$', text, re.M)
    pattern = (rf'ok\. {len(cases)} passed; 0 failed; 0 ignored; 0 measured; '
               r'\d+ filtered out; finished in [0-9.]+s\s*')
    if len(summaries) != 1 or not re.fullmatch(pattern, summaries[0]):
        raise ValueError('Native result summary does not prove complete execution')


def case_command(profile: str, prefix: str, case: str) -> list[str]:
    if profile not in {'debug', 'release'} or (prefix, case) not in inventory():
        raise ValueError('Unreviewed native invocation')
    package = ('codex-accounts-discovery' if (prefix, case) in DISCOVERY_CASES
               else 'codex-accounts-vault-storage')
    command = ['cargo', 'test', '-p', package, '--lib']
    if profile == 'release':
        command += ['--release']
    return command + ['--locked', '--offline', prefix + case, '--', '--exact',
                      '--test-threads=1', '--format=pretty', '--nocapture']


def run_case(profile: str, prefix: str, case: str) -> dict:
    output_bytes = b''
    started = time.monotonic()
    try:
        # Each case still has its own process and synthetic roots. No retry loop.
        # The job limit and the existing child watchdogs bound native hangs.
        with tempfile.TemporaryFile() as output:
            result = subprocess.run(case_command(profile, prefix, case), cwd=ROOT,
                                    stdout=output, stderr=output, timeout=180, check=False)
            output.seek(0)
            output_bytes = output.read(MAX_OUTPUT + 1)
            verify(output_bytes, result.returncode, (case,), prefix=prefix)
        passed = True
    except (OSError, ValueError, subprocess.SubprocessError):
        passed = False
    lines = re.findall(rb'(?:lifecycle_tests|target_tests|target_restart_tests|target_repair_tests)\.rs:(\d{1,5}):', output_bytes)
    return {'profile': profile, 'native_case': case,
            'result': 'passed' if passed else 'failed',
            'elapsed_seconds': round(time.monotonic() - started, 3),
            'failure_source_lines': [] if passed else [int(n) for n in lines[:8]],
            'failure_categories': [] if passed else failure_categories(output_bytes),
            'desktop_qualification': 'not_established'}


def run_profile(profile: str) -> bool:
    # Build once before parallel Cargo invocations to avoid concurrent cold builds.
    command = ['cargo', 'test', '-p', 'codex-accounts-vault-storage',
               '-p', 'codex-accounts-discovery', '--lib',
               '--no-run', '--locked', '--offline']
    if profile == 'release':
        command += ['--release']
    try:
        with tempfile.TemporaryFile() as output:
            result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=output,
                                    timeout=180, check=False)
        if result.returncode != 0:
            return False
    except (OSError, subprocess.SubprocessError):
        return False
    failed = False
    # Restart suites already fan out to four owned fixture workers internally.
    # Run them alone under the SAME per-case deadline, then use two workers for
    # ordinary cases. Avoid multiplying disk-heavy restart fanout across suites.
    cases = sorted(inventory(), key=lambda item: ('restart' not in item[1], item[1]))
    with ThreadPoolExecutor(max_workers=WORKERS) as executor:
        def collect(future):
            nonlocal failed
            try:
                record = future.result()
                print(json.dumps(record), flush=True)
                failed |= record['result'] != 'passed'
            except Exception:
                # Infrastructure exceptions remain failure; finish collecting
                # independent cases without exporting arbitrary exception text.
                failed = True
        for prefix, case in cases:
            if 'process_restart' in case:
                collect(executor.submit(run_case, profile, prefix, case))
        futures = [executor.submit(run_case, profile, prefix, case)
                   for prefix, case in cases if 'process_restart' not in case]
        for future in as_completed(futures):
            collect(future)
    return not failed


def run() -> None:
    if platform.system() != 'Windows' or platform.machine().lower() not in {'amd64', 'x86_64'}:
        raise ValueError('Windows x64 is required; a portable skip is not a pass')
    failed = False
    for profile in ('debug', 'release'):
        started = time.monotonic()
        passed = run_profile(profile)
        failed |= not passed
        print(json.dumps({'native_profile': profile, 'result': 'passed' if passed else 'failed',
                          'required_cases': len(inventory()), 'workers': WORKERS,
                          'elapsed_seconds': round(time.monotonic() - started, 3),
                          'desktop_qualification': 'not_established'}), flush=True)
    if failed:
        raise ValueError('One or more required native cases did not pass')


if __name__ == '__main__':
    try:
        run()
    except (OSError, ValueError, subprocess.SubprocessError):
        raise SystemExit('Named native execution did not pass; inspect the workspace test step') from None
