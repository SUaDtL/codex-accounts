"""No omitted or twice-executed native cases; process failures stay independent."""
import collections
from contextlib import redirect_stdout
import io
import json
import subprocess
import unittest
from unittest.mock import patch
from tools import ci_workspace_tests as workspace, run_native_tests as native

class PartitionTests(unittest.TestCase):
    def full(self):
        return collections.Counter([(prefix+case,'test') for prefix,case in native.inventory()]
                                   + [('tests::ordinary','test'),('tests::ordinary','test'),('tests::bench','benchmark')])
    def rest(self):
        return collections.Counter({('tests::ordinary','test'):2,('tests::bench','benchmark'):1})
    def test_compiled_partition_is_complete_disjoint_and_counts_duplicate_cross_crate_names(self):
        self.assertEqual(workspace.verify_partition(self.full(),self.rest()),len(native.inventory()))
        self.assertEqual(len(native.inventory()),27)
        args=workspace.selectors()
        self.assertEqual(args[0],'--exact')
        self.assertEqual(args[1::2],['--skip']*27)
        self.assertEqual(args[2::2],[p+c for p,c in native.inventory()])
    def test_missing_duplicated_extra_or_unassigned_skips_cannot_pass(self):
        first=next(iter(self.full()))
        wrong_full=self.full();wrong_full[first]+=1
        missing=self.full();del missing[first]
        extra=self.rest();extra[('tests::invented','test')]=1
        for full,remaining in [(wrong_full,self.rest()),(missing,self.rest()),(self.full(),extra),
                               (self.full(),collections.Counter()),(self.full(),self.full())]:
            with self.assertRaises(ValueError):workspace.verify_partition(full,remaining)
        # A new function sharing a skip substring may not silently disappear.
        full=self.full();full[(first[0]+'_additional','test')]=1
        with self.assertRaises(ValueError):workspace.verify_partition(full,self.rest())
    def test_listing_rejects_failure_empty_malformed_oversized_and_invalid_utf8(self):
        text=b'tests::ordinary: test\r\ntests::bench: benchmark\r\n1 test, 1 benchmark\r\n'
        self.assertEqual(sum(workspace.listing(text,0).values()),2)
        for data,code in [(text,1),(b'',0),(b'0 tests, 0 benchmarks\n',0),
                          (b'Unknown output',0),(b'\xff',0),(b'x'*(workspace.LIMIT+1),0)]:
            with self.assertRaises(ValueError):workspace.listing(data,code)
    def test_fixed_commands_keep_all_targets_locked_offline_and_no_fail_fast(self):
        for profile in ['debug','release']:
            command=workspace.cargo_command(profile)
            for flag in ['--workspace','--all-targets','--no-fail-fast','--locked','--offline']:
                self.assertIn(flag,command)
            self.assertEqual('--release' in command,profile=='release')
        with self.assertRaises(ValueError):workspace.cargo_command('--ignored')
        with self.assertRaises(ValueError):native.case_command('debug','','arbitrary')
    def test_windows_execution_requires_inventory_proof_then_preserves_cargo_failure(self):
        with patch.object(workspace.platform,'system',return_value='Windows'), \
             patch.object(workspace.platform,'machine',return_value='AMD64'), \
             patch.object(workspace,'read_listing',side_effect=[self.full(),self.rest()]) as listing, \
             patch.object(workspace.subprocess,'run',return_value=subprocess.CompletedProcess([],7)) as run, \
             redirect_stdout(io.StringIO()):
            self.assertEqual(workspace.run('debug'),7)
            self.assertEqual(listing.call_count,2)
            self.assertEqual(run.call_args.args[0],workspace.cargo_command('debug')+['--']+workspace.selectors())
        with patch.object(workspace.platform,'system',return_value='Windows'), \
             patch.object(workspace.platform,'machine',return_value='AMD64'), \
             patch.object(workspace,'read_listing',side_effect=[self.full(),self.full()]), \
             patch.object(workspace.subprocess,'run') as run:
            with self.assertRaises(ValueError):workspace.run('debug')
            run.assert_not_called()
    def test_other_os_runs_the_whole_workspace_and_windows_arm_refuses(self):
        with patch.object(workspace.platform,'system',return_value='Linux'), \
             patch.object(workspace,'read_listing') as listing, \
             patch.object(workspace.subprocess,'run',return_value=subprocess.CompletedProcess([],0)) as run, \
             redirect_stdout(io.StringIO()):
            self.assertEqual(workspace.run('release'),0)
            listing.assert_not_called()
            self.assertEqual(run.call_args.args[0],workspace.cargo_command('release'))
        with patch.object(workspace.platform,'system',return_value='Windows'), \
             patch.object(workspace.platform,'machine',return_value='ARM64'):
            with self.assertRaises(ValueError):workspace.run('debug')
    def test_partition_evidence_is_not_replaced_by_a_literal_or_previous_result(self):
        # Each invocation rereads both compiled inventories; no receipt/cache input.
        self.assertNotIn('receipt',workspace.cargo_command('debug'))
        with patch.object(native,'inventory',return_value=()):
            with self.assertRaises(ValueError):workspace.selectors()
        with patch.object(native,'inventory',return_value=(('x','y'),('x','y'))):
            with self.assertRaises(ValueError):workspace.selectors()

class IndependentNativeTests(unittest.TestCase):
    def test_failed_case_is_not_retried_and_other_cases_still_execute(self):
        seen=[]
        def case(profile,prefix,name):
            seen.append((prefix,name))
            return {'native_case':name,'result':'failed' if name==native.CASES[0] else 'passed'}
        with patch.object(native.subprocess,'run',return_value=subprocess.CompletedProcess([],0)) as build, \
             patch.object(native,'run_case',side_effect=case),redirect_stdout(io.StringIO()):
            self.assertFalse(native.run_profile('debug'))
        self.assertCountEqual(seen,native.inventory())
        self.assertEqual(len(seen),len(set(seen)))
        self.assertEqual(build.call_count,1)
        self.assertEqual(native.WORKERS,2)
        self.assertIn('--no-run',build.call_args.args[0])
    def test_infrastructure_exception_is_failure_but_does_not_abandon_other_cases(self):
        seen=[]
        def case(profile,prefix,name):
            seen.append((prefix,name))
            if name==native.CASES[0]:raise RuntimeError('SYNTHETIC_PRIVATE_PATH')
            return {'native_case':name,'result':'passed'}
        output=io.StringIO()
        with patch.object(native.subprocess,'run',return_value=subprocess.CompletedProcess([],0)), \
             patch.object(native,'run_case',side_effect=case),redirect_stdout(output):
            self.assertFalse(native.run_profile('release'))
        self.assertCountEqual(seen,native.inventory())
        self.assertNotIn('SYNTHETIC_PRIVATE_PATH',output.getvalue())
    def test_failed_preparation_never_executes_stale_test_binaries(self):
        with patch.object(native.subprocess,'run',return_value=subprocess.CompletedProcess([],1)), \
             patch.object(native,'run_case') as case:
            self.assertFalse(native.run_profile('debug'))
            case.assert_not_called()
    def test_case_timing_and_failure_records_never_echo_raw_output(self):
        prefix,name=native.inventory()[0]
        def fail(command,**kwargs):
            kwargs['stdout'].write(b'SYNTHETIC_SECRET_CANNOT_LEAVE_LOCAL_OUTPUT\n')
            return subprocess.CompletedProcess(command,1)
        with patch.object(native.subprocess,'run',side_effect=fail):
            record=native.run_case('debug',prefix,name)
        self.assertEqual(record['result'],'failed')
        self.assertIn('elapsed_seconds',record)
        self.assertNotIn('SYNTHETIC_SECRET',json.dumps(record))
    def test_repair_inventory_cannot_hide_ignored_or_missing_case(self):
        for name in native.REPAIR_CASES:
            text=f'test {native.TARGET_PREFIX}{name} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'.encode()
            native.verify(text,0,(name,),prefix=native.TARGET_PREFIX)
            with self.assertRaises(ValueError):native.verify(text.replace(b' ... ok',b' ... ignored'),0,(name,),prefix=native.TARGET_PREFIX)

if __name__=='__main__':unittest.main()
