"""Bounded Windows CI process ownership. No product qualification authority.

The caller owns the shared binary output file, including on failure. Only the
new suspended process joins the anonymous job; the calling host never does.
"""
from __future__ import annotations

import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import time

REAP_SECONDS = 5
_launch_blocked = False
_state_lock = threading.RLock()


class ProcessError(OSError):
    """Fixed refusal category; never raw OS text, arguments, paths or output."""


def _refuse_cleanup() -> None:
    global _launch_blocked
    with _state_lock:
        _launch_blocked = True
    raise ProcessError('owned_process_cleanup_failed')


def _cleanup(process) -> None:
    deadline = time.monotonic() + REAP_SECONDS
    try:
        # Also kill a lingering descendant after an otherwise successful root.
        process.terminate()
        while True:
            if process.poll() is not None and process.empty():
                return
            if time.monotonic() >= deadline:
                _refuse_cleanup()
            time.sleep(0.005)
    except Exception:
        _refuse_cleanup()


def run(command, *, cwd, stdout, stderr, timeout, check=False):
    """Run exact argv in an owned job; timeout includes startup, never cleanup.

    Reap has its own fixed five-second failure bound. Cleanup failure poisons
    future launches in this interpreter, even when a caller catches OSError.
    No shell, alternate stdio, environment override, retry or platform fallback.
    """
    if (not isinstance(command, (list, tuple)) or not command
            or any(not isinstance(arg, str) or '\0' in arg for arg in command)
            or not command[0] or not isinstance(check, bool)
            or isinstance(timeout, bool) or not isinstance(timeout, (int, float))
            or not math.isfinite(timeout) or timeout <= 0 or stdout is not stderr):
        raise ProcessError('owned_process_invalid_invocation')
    try:
        if stdout.closed or not stdout.writable() or 'b' not in stdout.mode:
            raise ValueError
        stdout.fileno()
        directory = Path(cwd)
        if not directory.is_absolute() or not directory.is_dir():
            raise ValueError
    except (OSError, AttributeError, TypeError, ValueError):
        raise ProcessError('owned_process_invalid_invocation') from None
    argv = list(command)
    started = time.monotonic()
    process = None
    try:
        with _state_lock:
            if _launch_blocked:
                raise ProcessError('owned_process_launch_blocked')
            process = _Process(argv, directory, stdout)
            process.assign()
            if time.monotonic() - started >= timeout:
                raise subprocess.TimeoutExpired(argv, timeout)
            process.resume()
        while True:
            remaining = timeout - (time.monotonic() - started)
            if remaining <= 0:
                raise subprocess.TimeoutExpired(argv, timeout)
            code = process.poll()
            if code is not None:
                break
            time.sleep(min(0.005, remaining))
    finally:
        if process is not None:
            try:
                _cleanup(process)
            finally:
                process.close()
    if check and code:
        raise subprocess.CalledProcessError(code, argv)
    return subprocess.CompletedProcess(argv, code)


if sys.platform == 'win32':
    import ctypes as C
    from ctypes import wintypes as W
    import msvcrt

    class _Security(C.Structure):
        _fields_ = [('size', W.DWORD), ('descriptor', W.LPVOID), ('inherit', W.BOOL)]

    class _Startup(C.Structure):
        _fields_ = [('cb', W.DWORD), ('reserved', W.LPWSTR), ('desktop', W.LPWSTR),
                    ('title', W.LPWSTR), ('x', W.DWORD), ('y', W.DWORD),
                    ('xsize', W.DWORD), ('ysize', W.DWORD), ('xchars', W.DWORD),
                    ('ychars', W.DWORD), ('fill', W.DWORD), ('flags', W.DWORD),
                    ('show', W.WORD), ('reserved_size', W.WORD),
                    ('reserved_bytes', C.POINTER(W.BYTE)), ('stdin', W.HANDLE),
                    ('stdout', W.HANDLE), ('stderr', W.HANDLE)]

    class _StartupEx(C.Structure):
        _fields_ = [('startup', _Startup), ('attributes', W.LPVOID)]

    class _Info(C.Structure):
        _fields_ = [('process', W.HANDLE), ('thread', W.HANDLE),
                    ('pid', W.DWORD), ('tid', W.DWORD)]

    class _BasicLimits(C.Structure):
        _fields_ = [('process_time', C.c_int64), ('job_time', C.c_int64),
                    ('flags', W.DWORD), ('minimum', C.c_size_t),
                    ('maximum', C.c_size_t), ('active', W.DWORD),
                    ('affinity', C.c_size_t), ('priority', W.DWORD),
                    ('scheduling', W.DWORD)]

    class _Io(C.Structure):
        _fields_ = [(name, C.c_uint64) for name in
                    ('read_ops', 'write_ops', 'other_ops', 'read_bytes', 'write_bytes', 'other_bytes')]

    class _Limits(C.Structure):
        _fields_ = [('basic', _BasicLimits), ('io', _Io),
                    ('process_memory', C.c_size_t), ('job_memory', C.c_size_t),
                    ('peak_process', C.c_size_t), ('peak_job', C.c_size_t)]

    class _Accounting(C.Structure):
        _fields_ = [('user', C.c_int64), ('kernel', C.c_int64),
                    ('period_user', C.c_int64), ('period_kernel', C.c_int64),
                    ('faults', W.DWORD), ('total', W.DWORD),
                    ('active', W.DWORD), ('terminated', W.DWORD)]

    _k = C.WinDLL('kernel32', use_last_error=True)
    def _api(name, args, result=W.BOOL):
        function = getattr(_k, name)
        function.argtypes = args
        function.restype = result
        return function

    _CreateJob = _api('CreateJobObjectW', [W.LPVOID, W.LPCWSTR], W.HANDLE)
    _SetJob = _api('SetInformationJobObject', [W.HANDLE, C.c_int, W.LPVOID, W.DWORD])
    _QueryJob = _api('QueryInformationJobObject', [W.HANDLE, C.c_int, W.LPVOID, W.DWORD, W.LPVOID])
    _Assign = _api('AssignProcessToJobObject', [W.HANDLE, W.HANDLE])
    _TerminateJob = _api('TerminateJobObject', [W.HANDLE, W.UINT])
    _TerminateProcess = _api('TerminateProcess', [W.HANDLE, W.UINT])
    _Wait = _api('WaitForSingleObject', [W.HANDLE, W.DWORD], W.DWORD)
    _ExitCode = _api('GetExitCodeProcess', [W.HANDLE, C.POINTER(W.DWORD)])
    _OpenProcess = _api('OpenProcess', [W.DWORD, W.BOOL, W.DWORD], W.HANDLE)
    _InJob = _api('IsProcessInJob', [W.HANDLE, W.HANDLE, C.POINTER(W.BOOL)])
    _Close = _api('CloseHandle', [W.HANDLE])
    _Current = _api('GetCurrentProcess', [], W.HANDLE)
    _Duplicate = _api('DuplicateHandle', [W.HANDLE, W.HANDLE, W.HANDLE,
                                        C.POINTER(W.HANDLE), W.DWORD, W.BOOL, W.DWORD])
    _CreateFile = _api('CreateFileW', [W.LPCWSTR, W.DWORD, W.DWORD,
                                     C.POINTER(_Security), W.DWORD, W.DWORD, W.HANDLE], W.HANDLE)
    _InitializeAttributes = _api('InitializeProcThreadAttributeList',
                                [W.LPVOID, W.DWORD, W.DWORD, C.POINTER(C.c_size_t)])
    _UpdateAttribute = _api('UpdateProcThreadAttribute',
                           [W.LPVOID, W.DWORD, C.c_size_t, W.LPVOID, C.c_size_t, W.LPVOID, W.LPVOID])
    _DeleteAttributes = _api('DeleteProcThreadAttributeList', [W.LPVOID], None)
    _CreateProcess = _api('CreateProcessW', [W.LPCWSTR, W.LPWSTR, W.LPVOID, W.LPVOID,
                                            W.BOOL, W.DWORD, W.LPVOID, W.LPCWSTR,
                                            C.POINTER(_StartupEx), C.POINTER(_Info)])
    _Resume = _api('ResumeThread', [W.HANDLE], W.DWORD)


class _Process:
    """Own every successful native allocation, including a failed startup."""
    def __init__(self, argv, cwd, output):
        self.job = self.process = self.thread = None
        self.assigned = False
        self.closed = False
        self._handles = []
        self._members = []
        self._attributes = None
        if sys.platform != 'win32':
            raise ProcessError('owned_process_windows_required')
        try:
            executable = shutil.which(argv[0])
            if (executable is None or not Path(executable).is_absolute()
                    or Path(executable).suffix.lower() != '.exe'):
                raise ProcessError('owned_process_executable_unavailable')
            output.flush()
            self.job = _CreateJob(None, None)
            if not self.job:
                raise ProcessError('owned_process_job_unavailable')
            limits = _Limits()
            limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            if not _SetJob(self.job, 9, C.byref(limits), C.sizeof(limits)):
                raise ProcessError('owned_process_job_unavailable')
            duplicate = W.HANDLE()
            if not _Duplicate(_Current(), msvcrt.get_osfhandle(output.fileno()), _Current(),
                              C.byref(duplicate), 0, True, 2):  # DUPLICATE_SAME_ACCESS
                raise ProcessError('owned_process_stdio_unavailable')
            self._handles.append(duplicate.value)
            attributes = _Security(C.sizeof(_Security), None, True)
            null_input = _CreateFile('NUL', 0x80000000, 3, C.byref(attributes), 3, 0, None)
            if null_input in (None, C.c_void_p(-1).value):
                raise ProcessError('owned_process_stdio_unavailable')
            self._handles.append(null_input)
            length = C.c_size_t()
            _InitializeAttributes(None, 1, 0, C.byref(length))
            if not 0 < length.value <= 65536:
                raise ProcessError('owned_process_handle_list_unavailable')
            buffer = C.create_string_buffer(length.value)
            if not _InitializeAttributes(buffer, 1, 0, C.byref(length)):
                raise ProcessError('owned_process_handle_list_unavailable')
            self._attributes = buffer
            inherited = (W.HANDLE * 2)(duplicate.value, null_input)
            if not _UpdateAttribute(buffer, 0, 0x20002, inherited, C.sizeof(inherited), None, None):
                raise ProcessError('owned_process_handle_list_unavailable')
            startup = _StartupEx()
            startup.startup.cb = C.sizeof(startup)
            startup.startup.flags = 0x100  # STARTF_USESTDHANDLES
            startup.startup.stdin = null_input
            startup.startup.stdout = startup.startup.stderr = duplicate.value
            startup.attributes = C.cast(buffer, W.LPVOID)
            command = C.create_unicode_buffer(subprocess.list2cmdline(argv))
            info = _Info()
            # Explicit native image, exact argument array, no shell. Environment
            # is the current reviewed CI environment; cwd is caller-selected.
            if not _CreateProcess(executable, command, None, None, True,
                                  0x4 | 0x80000 | 0x8000000, None, str(cwd),
                                  C.byref(startup), C.byref(info)):
                raise ProcessError('owned_process_creation_failed')
            self.process, self.thread = info.process, info.thread
            # Duplicates/attribute list are needed only during CreateProcess.
            self._close_startup()
        except Exception as error:
            # Constructor failures do not leak an unassigned suspended process.
            try:
                if self.process:
                    _cleanup(self)
            finally:
                self.close()
            if isinstance(error, ProcessError):
                raise
            raise ProcessError('owned_process_creation_failed') from None

    def _close_startup(self):
        if self._attributes is not None:
            _DeleteAttributes(self._attributes)
            self._attributes = None
        failed = False
        handles, self._handles = self._handles, []
        for handle in handles:
            failed |= not bool(_Close(handle))
        if failed:
            _refuse_cleanup()

    def assign(self):
        if not _Assign(self.job, self.process):
            raise ProcessError('owned_process_assignment_failed')
        self.assigned = True

    def resume(self):
        if not self.assigned or _Resume(self.thread) != 1:
            raise ProcessError('owned_process_resume_failed')

    def poll(self):
        status = _Wait(self.process, 0)
        if status == 258:  # WAIT_TIMEOUT
            return None
        if status != 0:
            raise ProcessError('owned_process_wait_failed')
        code = W.DWORD()
        if not _ExitCode(self.process, C.byref(code)):
            raise ProcessError('owned_process_wait_failed')
        return code.value

    def active(self):
        info = _Accounting()
        if not _QueryJob(self.job, 1, C.byref(info), C.sizeof(info), None):
            raise ProcessError('owned_process_job_query_failed')
        return info.active

    def empty(self):
        if self.active() != 0:
            return False
        for handle in self._members:
            status = _Wait(handle, 0)
            if status == 258:
                return False
            if status != 0:
                raise ProcessError('owned_process_wait_failed')
        return True

    def _retain_members(self):
        # A bounded survey pins existing members before termination. The job's
        # empty accounting additionally covers members born after this survey.
        capacity = 4096
        class Members(C.Structure):
            _fields_ = [('assigned', W.DWORD), ('listed', W.DWORD),
                        ('pids', C.c_size_t * capacity)]
        members = Members()
        if not _QueryJob(self.job, 3, C.byref(members), C.sizeof(members), None):
            raise ProcessError('owned_process_job_query_failed')
        if members.assigned != members.listed or members.listed > capacity:
            raise ProcessError('owned_process_job_query_failed')
        for pid in members.pids[:members.listed]:
            handle = _OpenProcess(0x100000 | 0x1000, False, pid)
            if not handle:
                # An exiting process may vanish between enumeration and open;
                # job emptiness remains mandatory before returning success.
                if C.get_last_error() == 87:
                    continue
                raise ProcessError('owned_process_member_query_failed')
            belongs = W.BOOL()
            if not _InJob(handle, self.job, C.byref(belongs)) or not belongs.value:
                _Close(handle)
                raise ProcessError('owned_process_member_query_failed')
            self._members.append(handle)

    def terminate(self):
        if self.assigned:
            self._retain_members()
            if not _TerminateJob(self.job, 1):
                raise ProcessError('owned_process_termination_failed')
        elif self.process and self.poll() is None:
            if not _TerminateProcess(self.process, 1):
                raise ProcessError('owned_process_termination_failed')

    def close(self):
        if self.closed:
            return
        failed = False
        try:
            self._close_startup()
        except ProcessError:
            failed = True
        for handle in self._members:
            failed |= not bool(_Close(handle))
        self._members = []
        for name in ('thread', 'process', 'job'):
            handle = getattr(self, name)
            if handle:
                failed |= not bool(_Close(handle))
                setattr(self, name, None)
        self.closed = True
        if failed:
            _refuse_cleanup()
