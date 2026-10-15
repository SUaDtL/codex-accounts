"""Owned synthetic processes only; no Desktop/runtime execution or authority."""
import ctypes
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from tools import windows_process as owned


class FakeProcess:
    events = []
    code = 0
    fails = None
    def __init__(self, *args):
        self.events.append('create_suspended')
    def assign(self):
        self.events.append('assign')
        if self.fails == 'assign':
            raise owned.ProcessError('assignment_failed')
    def resume(self):
        self.events.append('resume')
        if self.fails == 'resume':
            raise owned.ProcessError('resume_failed')
    def poll(self):
        return self.code
    def terminate(self):
        self.events.append('terminate')
        if self.fails == 'cleanup':
            raise owned.ProcessError('cleanup_failed')
        self.code = 1
    def empty(self):
        self.events.append('empty')
        return True
    def close(self):
        self.events.append('close')


class SupervisionTests(unittest.TestCase):
    def setUp(self):
        FakeProcess.events = []
        FakeProcess.code = 0
        FakeProcess.fails = None
        poison = patch.object(owned, '_launch_blocked', False)
        poison.start()
        self.addCleanup(poison.stop)
        self.output = tempfile.TemporaryFile()
        self.addCleanup(self.output.close)
    def run_owned(self, timeout=1, check=False):
        with patch.object(owned, '_Process', FakeProcess):
            return owned.run([sys.executable, '-c', 'pass'], cwd=Path.cwd(),
                             stdout=self.output, stderr=self.output,
                             timeout=timeout, check=check)
    def test_success_assigns_before_resume_and_proves_empty_before_closing(self):
        result = self.run_owned()
        self.assertIsInstance(result, subprocess.CompletedProcess)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(FakeProcess.events,
                         ['create_suspended', 'assign', 'resume', 'terminate', 'empty', 'close'])
        self.assertFalse(self.output.closed)
    def test_assignment_and_resume_refusals_cleanup_before_return(self):
        for stage in ('assign', 'resume'):
            with self.subTest(stage=stage):
                FakeProcess.events = []
                FakeProcess.fails = stage
                with self.assertRaises(owned.ProcessError):
                    self.run_owned()
                self.assertEqual(FakeProcess.events[-3:], ['terminate', 'empty', 'close'])
                if stage == 'assign':
                    self.assertNotIn('resume', FakeProcess.events)
    def test_timeout_is_failure_after_owned_cleanup_and_output_is_readable(self):
        FakeProcess.code = None
        self.output.write(b'SYNTHETIC_PARTIAL_OUTPUT')
        with self.assertRaises(subprocess.TimeoutExpired):
            self.run_owned(timeout=0.01)
        self.assertEqual(FakeProcess.events[-3:], ['terminate', 'empty', 'close'])
        self.output.seek(0)
        self.assertEqual(self.output.read(), b'SYNTHETIC_PARTIAL_OUTPUT')
    def test_startup_counts_against_deadline_and_expiry_never_resumes(self):
        with patch.object(owned.time, 'monotonic', side_effect=[0, 2, 2, 2]):
            with self.assertRaises(subprocess.TimeoutExpired):
                self.run_owned(timeout=1)
        self.assertNotIn('resume', FakeProcess.events)
        self.assertEqual(FakeProcess.events[-3:], ['terminate', 'empty', 'close'])
    def test_cleanup_failure_blocks_every_later_launch(self):
        FakeProcess.fails = 'cleanup'
        with self.assertRaisesRegex(owned.ProcessError, '^owned_process_cleanup_failed$'):
            self.run_owned()
        before = list(FakeProcess.events)
        with self.assertRaisesRegex(owned.ProcessError, '^owned_process_launch_blocked$'):
            self.run_owned()
        self.assertEqual(FakeProcess.events, before)
        self.assertEqual(FakeProcess.events[-1], 'close')
    def test_nonempty_job_exhausts_cleanup_bound_and_blocks_launch(self):
        with patch.object(FakeProcess, 'empty', return_value=False), \
             patch.object(owned, 'REAP_SECONDS', 0.01):
            with self.assertRaisesRegex(owned.ProcessError, '^owned_process_cleanup_failed$'):
                self.run_owned()
        self.assertTrue(owned._launch_blocked)
    def test_check_failure_is_raised_only_after_cleanup(self):
        FakeProcess.code = 7
        with self.assertRaises(subprocess.CalledProcessError) as result:
            self.run_owned(check=True)
        self.assertEqual(result.exception.returncode, 7)
        self.assertEqual(FakeProcess.events[-1], 'close')
    def test_invalid_bounds_and_stdio_never_create_process(self):
        for timeout in (0, -1, float('inf'), float('nan')):
            with self.assertRaises(owned.ProcessError):
                self.run_owned(timeout)
        with self.assertRaises(owned.ProcessError):
            owned.run([sys.executable], cwd=Path.cwd(), stdout=self.output,
                      stderr=subprocess.PIPE, timeout=1)
        self.assertEqual(FakeProcess.events, [])


@unittest.skipUnless(sys.platform == 'win32', 'Windows native supervisor fixture')
class NativeProcessTests(unittest.TestCase):
    def setUp(self):
        self.assertFalse(owned._launch_blocked, 'owned_process_cleanup_poisoned')
    def invoke(self, script, *, timeout=10, observer=None):
        with tempfile.TemporaryFile() as output:
            native = owned._Process
            class Observed(native):
                def resume(self):
                    if observer is not None:
                        observer(self)
                    super().resume()
            with patch.object(owned, '_Process', Observed):
                result = owned.run([sys.executable, '-c', script], cwd=Path.cwd(),
                                   stdout=output, stderr=output, timeout=timeout)
            output.seek(0)
            return result, output.read()
    def test_actual_suspended_assignment_normal_output_and_closed_handles(self):
        seen = []
        def before_resume(process):
            self.assertEqual(process.active(), 1)
            self.assertIsNone(process.poll())
            seen.append((process, process.process, process.thread))
        result, output = self.invoke('print("SYNTHETIC_ONLY")', observer=before_resume)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output.strip(), b'SYNTHETIC_ONLY')
        self.assertEqual(len(seen), 1)
        self.assertTrue(seen[0][0].closed)
        for handle in seen[0][1:]:
            self.assertEqual(owned._Wait(handle, 0), 0xffffffff)
    def test_actual_assignment_failure_never_resumes_and_reaps_suspended_root(self):
        native = owned._Process
        retained = []
        class Observed(native):
            def close(self):
                if self.process:
                    retained.append((self.poll(), self.active()))
                super().close()
        with tempfile.TemporaryFile() as output, \
             patch.object(owned, '_Process', Observed), \
             patch.object(owned, '_Assign', return_value=0), \
             patch.object(owned, '_Resume') as resume:
            with self.assertRaisesRegex(owned.ProcessError, '^owned_process_assignment_failed$'):
                owned.run([sys.executable, '-c', 'print("SHOULD_NOT_RUN")'], cwd=Path.cwd(),
                          stdout=output, stderr=output, timeout=10)
            resume.assert_not_called()
            output.seek(0)
            self.assertEqual(output.read(), b'')
        self.assertEqual(retained[-1][1], 0)
        self.assertIsNotNone(retained[-1][0])
    def test_timeout_and_lingering_descendant_are_killed_with_empty_job(self):
        for root_waits in (True, False):
            with self.subTest(root_waits=root_waits), tempfile.TemporaryDirectory() as directory:
                marker = Path(directory) / 'synthetic-child-ready'
                child = 'import pathlib,time; pathlib.Path(' + repr(str(marker)) + ').touch(); time.sleep(60)'
                script = ('import subprocess,sys,time,pathlib\nsubprocess.Popen([sys.executable,"-c",'
                          + repr(child) + '])\n'
                          + ('time.sleep(60)' if root_waits else
                             'while not pathlib.Path(' + repr(str(marker)) + ').exists(): time.sleep(.01)'))
                retained = []
                native = owned._Process
                class Observed(native):
                    def close(self):
                        retained.append((self.poll(), self.active()))
                        super().close()
                with tempfile.TemporaryFile() as output, patch.object(owned, '_Process', Observed):
                    if root_waits:
                        with self.assertRaises(subprocess.TimeoutExpired):
                            owned.run([sys.executable, '-c', script], cwd=directory,
                                      stdout=output, stderr=output, timeout=3)
                    else:
                        result = owned.run([sys.executable, '-c', script], cwd=directory,
                                           stdout=output, stderr=output, timeout=10)
                        self.assertEqual(result.returncode, 0)
                self.assertTrue(marker.exists())
                self.assertEqual(retained[-1][1], 0)
                self.assertIsNotNone(retained[-1][0])
    def test_unrelated_process_and_ambient_inheritable_handle_are_preserved(self):
        read_fd, write_fd = os.pipe()
        os.set_inheritable(write_fd, True)
        try:
            import msvcrt
            handle = msvcrt.get_osfhandle(write_fd)
            script = ('import ctypes; from ctypes import wintypes; k=ctypes.WinDLL("kernel32",use_last_error=True); '
                      'k.GetHandleInformation.argtypes=[wintypes.HANDLE,ctypes.POINTER(wintypes.DWORD)]; '
                      'k.CreateEventW.argtypes=[wintypes.LPVOID,wintypes.BOOL,wintypes.BOOL,wintypes.LPCWSTR]; '
                      'k.CreateEventW.restype=wintypes.HANDLE; '
                      'k.WriteFile.argtypes=[wintypes.HANDLE,wintypes.LPCVOID,wintypes.DWORD,ctypes.POINTER(wintypes.DWORD),wintypes.LPVOID]; '
                      'flags=wintypes.DWORD(); events=[]\n'
                      'for _ in range(4096):\n'
                      ' if k.GetHandleInformation(' + str(handle) + ',ctypes.byref(flags)): break\n'
                      ' event=k.CreateEventW(None,True,False,None)\n'
                      ' if not event: raise RuntimeError("synthetic event allocation failed")\n'
                      ' events.append(event)\n'
                      'assert k.GetHandleInformation(' + str(handle) + ',ctypes.byref(flags))\n'
                      'count=wintypes.DWORD()\n'
                      'k.WriteFile(' + str(handle) + ',b"P",1,ctypes.byref(count),None)\n'
                      'print("SYNTHETIC_HANDLE_VALID")')
            unrelated = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
            try:
                result, output = self.invoke(script)
                self.assertEqual(result.returncode, 0)
                self.assertIn(b'SYNTHETIC_HANDLE_VALID', output)
                self.assertIsNone(unrelated.poll())
                # A valid child handle number may denote a different object.
                # Test the actual pipe effect, never numerical handle absence.
                os.write(write_fd, b'X')
                self.assertEqual(os.read(read_fd, 1), b'X')

                # Sensitivity control: deliberately add the ambient pipe to the
                # real owned child's HANDLE_LIST, retaining its ctypes storage
                # through CreateProcess. The semantic assertion MUST catch P.
                real_update = owned._UpdateAttribute
                retained_lists = []
                def leak_only_fixture_pipe(attributes, flags, attribute, value, size, previous, returned):
                    self.assertEqual(attribute, 0x20002)
                    self.assertEqual(size, ctypes.sizeof(owned.W.HANDLE) * 2)
                    original = ctypes.cast(value, ctypes.POINTER(owned.W.HANDLE))
                    handles = (owned.W.HANDLE * 3)(original[0], original[1], handle)
                    retained_lists.append(handles)
                    return real_update(attributes, flags, attribute, handles,
                                       ctypes.sizeof(handles), previous, returned)
                with patch.object(owned, '_UpdateAttribute', leak_only_fixture_pipe):
                    result, output = self.invoke(script)
                self.assertEqual(result.returncode, 0)
                self.assertIn(b'SYNTHETIC_HANDLE_VALID', output)
                self.assertEqual(len(retained_lists), 1)
                os.write(write_fd, b'X')
                with self.assertRaises(AssertionError):
                    self.assertEqual(os.read(read_fd, 1), b'X')
                self.assertEqual(os.read(read_fd, 1), b'X')
                self.assertIsNone(unrelated.poll())
            finally:
                unrelated.kill()
                unrelated.wait(timeout=5)
        finally:
            os.close(write_fd)
            os.close(read_fd)


if __name__ == '__main__':
    unittest.main()
