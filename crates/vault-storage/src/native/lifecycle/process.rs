//! Read-only, retained-handle process observations. No terminate right is requested.
use super::super::*;
use crate::lifecycle_model::{child_of, shared_home_readiness, Fault, Life, ProcessKey};
use crate::lifecycle_model::{MAX_FAMILY, MAX_PROCESSES, MAX_WINDOWS};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn failed() -> Fault {
    match unsafe { GetLastError() } {
        ERROR_ACCESS_DENIED => Fault::AccessDenied,
        ERROR_INVALID_PARAMETER | ERROR_NOT_FOUND => Fault::Disappeared,
        _ => Fault::Incomplete,
    }
}
fn filetime(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}
fn owner(handle: HANDLE) -> Result<[u8; 32], Fault> {
    let mut token = null_mut();
    // SAFETY: retained process handle; least-privilege token query, never impersonation.
    if unsafe { OpenProcessToken(handle, TOKEN_QUERY, &mut token) } == 0 {
        return Err(failed());
    }
    let token = Token(token);
    let mut buffer = [0u64; 128];
    let mut length = 0;
    // SAFETY: aligned bounded output. A larger or inaccessible token refuses.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
            &mut length,
        )
    } == 0
    {
        return Err(failed());
    }
    if length as usize > std::mem::size_of_val(&buffer)
        || (length as usize) < std::mem::size_of::<TOKEN_USER>()
    {
        return Err(Fault::Incomplete);
    }
    // SAFETY: successful TOKEN_USER output owns its SID until this function returns.
    let sid = sid_copy(unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid })
        .map_err(|_| Fault::Incomplete)?;
    Ok(Sha256::digest(&sid).into())
}
fn image(handle: HANDLE) -> Result<Vec<u16>, Fault> {
    let mut value = vec![0; 1024];
    let mut count = value.len() as u32;
    // SAFETY: writable initialized capacity, non-mutating OS query.
    if unsafe { QueryFullProcessImageNameW(handle, 0, value.as_mut_ptr(), &mut count) } == 0 {
        return Err(failed());
    }
    if count == 0 || count as usize >= value.len() {
        return Err(Fault::Bound);
    }
    value.truncate(count as usize);
    Ok(value)
}

pub(super) struct ObservedProcess {
    handle: Token,
    key: ProcessKey,
    owner: [u8; 32],
    image: Vec<u16>,
}
impl std::fmt::Debug for ObservedProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ObservedProcess([REDACTED])")
    }
}
impl ObservedProcess {
    pub(super) fn open(pid: u32) -> Result<Self, Fault> {
        if pid == 0 {
            return Err(Fault::Incomplete);
        }
        // SAFETY: observation rights only. PID is not retained as later signal authority.
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return Err(failed());
        }
        let handle = Token(handle);
        let mut created = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: exact SDK output structures and owned query handle.
        if unsafe { GetProcessTimes(handle.0, &mut created, &mut exit, &mut kernel, &mut user) }
            == 0
        {
            return Err(failed());
        }
        let key = ProcessKey {
            pid,
            created: filetime(created),
        };
        if key.created == 0 {
            return Err(Fault::Incomplete);
        }
        let observed = (|| Ok::<_, Fault>((owner(handle.0)?, image(handle.0)?)))();
        let (owner, image) = match observed {
            Ok(value) => value,
            Err(error) => {
                // An exited retained handle establishes disappearance, not an
                // inaccessible process being silently treated as an empty slot.
                if unsafe { WaitForSingleObject(handle.0, 0) } == WAIT_OBJECT_0 {
                    return Err(Fault::Disappeared);
                }
                return Err(error);
            }
        };
        let process = Self {
            handle,
            key,
            owner,
            image,
        };
        process.ensure_live(key)?;
        Ok(process)
    }
    pub(super) fn key(&self) -> ProcessKey {
        self.key
    }
    pub(super) fn raw(&self) -> HANDLE {
        self.handle.0
    }
    pub(super) fn signalled(&self) -> Result<bool, Fault> {
        // SAFETY: retained SYNCHRONIZE handle. No wait on a reused PID.
        match unsafe { WaitForSingleObject(self.handle.0, 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(Fault::Incomplete),
        }
    }
    pub(super) fn life(&self, parent: u32) -> Result<Life, Fault> {
        let exited = self.signalled()?;
        let mut created = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: handle retained even after exit; exit output used ONLY if signalled.
        if unsafe {
            GetProcessTimes(
                self.handle.0,
                &mut created,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        } == 0
        {
            return Err(failed());
        }
        if filetime(created) != self.key.created {
            return Err(Fault::Changed);
        }
        let value = Life {
            key: self.key,
            parent,
            owner: self.owner,
            exited: exited.then(|| filetime(exit)),
        };
        value.validate()?;
        Ok(value)
    }
    pub(super) fn ensure_live(&self, expected: ProcessKey) -> Result<(), Fault> {
        if expected != self.key {
            return Err(Fault::Changed);
        }
        if self.signalled()? {
            return Err(Fault::Disappeared);
        }
        if self.life(0)?.key != expected
            || owner(self.handle.0)? != self.owner
            || image(self.handle.0)? != self.image
        {
            return Err(Fault::Changed);
        }
        Ok(())
    }
    pub(super) fn wait_exit(&self, timeout: Duration) -> Result<(), Fault> {
        if timeout > Duration::from_secs(30) {
            return Err(Fault::Bound);
        }
        // SAFETY: finite wait on a retained process handle, never signal receipt.
        match unsafe { WaitForSingleObject(self.handle.0, timeout.as_millis() as u32) } {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT => Err(Fault::Timeout),
            _ => Err(Fault::Incomplete),
        }
    }
    /// Normal OS close only. Delivery is not exit, descendant or home evidence.
    /// No production caller exists until exact binding and native consent are implemented.
    pub(super) fn request_normal_quit(&self, expected: ProcessKey) -> Result<usize, Fault> {
        self.ensure_live(expected)?;
        if owner(unsafe { GetCurrentProcess() })? != self.owner {
            return Err(Fault::AccessDenied);
        }
        let mut state = Windows {
            pid: self.key.pid,
            handles: Vec::new(),
            overflow: false,
            seen: 0,
            started: Instant::now(),
        };
        // SAFETY: callback is synchronous; stack state lives through EnumWindows.
        if unsafe { EnumWindows(Some(collect_window), (&mut state as *mut Windows) as isize) } == 0
        {
            return Err(if state.overflow {
                Fault::Bound
            } else {
                Fault::Incomplete
            });
        }
        if state.handles.is_empty() {
            return Err(Fault::QuitUnavailable);
        }
        for (window, thread) in &state.handles {
            self.ensure_live(expected)?;
            let mut pid = 0;
            // SAFETY: reobserve window ownership immediately before normal close.
            let observed_thread = unsafe { GetWindowThreadProcessId(*window, &mut pid) };
            if observed_thread != *thread || pid != self.key.pid {
                return Err(Fault::Changed);
            }
            // SAFETY: WM_CLOSE has no pointers. It can be refused by the recipient.
            if unsafe { PostMessageW(*window, WM_CLOSE, 0, 0) } == 0 {
                return Err(failed());
            }
        }
        Ok(state.handles.len())
    }
}
struct Windows {
    pid: u32,
    handles: Vec<(HWND, u32)>,
    overflow: bool,
    seen: usize,
    started: Instant,
}
unsafe extern "system" fn collect_window(
    window: HWND,
    argument: LPARAM,
) -> windows_sys::core::BOOL {
    // SAFETY: only EnumWindows above supplies this live uniquely borrowed pointer.
    let state = unsafe { &mut *(argument as *mut Windows) };
    state.seen += 1;
    if state.seen > MAX_PROCESSES || state.started.elapsed() >= Duration::from_secs(2) {
        state.overflow = true;
        return 0;
    }
    let mut pid = 0;
    let thread = unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid == state.pid {
        if thread == 0 || state.handles.len() >= MAX_WINDOWS {
            state.overflow = true;
            return 0;
        }
        state.handles.push((window, thread));
    }
    1
}

pub(super) struct Inventory {
    pub(super) entries: Vec<(u32, ObservedProcess)>,
    pub(super) complete: bool,
}
impl Inventory {
    pub(super) fn capture() -> Result<Self, Fault> {
        // SAFETY: read-only process snapshot, no inherited snapshot handle.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(failed());
        }
        let snapshot = Token(snapshot);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut inventory = Self {
            entries: Vec::new(),
            complete: true,
        };
        let mut seen = BTreeSet::new();
        let start = Instant::now();
        // SAFETY: correct structure size set before the documented iteration API.
        let mut found = unsafe { Process32FirstW(snapshot.0, &mut entry) };
        while found != 0 {
            if seen.len() >= MAX_PROCESSES || start.elapsed() >= Duration::from_secs(2) {
                return Err(Fault::Bound);
            }
            if !seen.insert(entry.th32ProcessID) {
                return Err(Fault::Incomplete);
            }
            match ObservedProcess::open(entry.th32ProcessID) {
                Ok(process) => inventory.entries.push((entry.th32ParentProcessID, process)),
                Err(_) => inventory.complete = false,
            }
            found = unsafe { Process32NextW(snapshot.0, &mut entry) };
        }
        if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
            return Err(Fault::Incomplete);
        }
        Ok(inventory)
    }
    pub(super) fn shared_home_readiness(&self) -> Result<(), Fault> {
        shared_home_readiness(self.complete)
    }
}
/// Keeps every observed descendant handle, including after its parent exits.
/// Snapshot polling cannot establish complete birth history or Codex home binding.
pub(super) struct ObservedTree {
    members: BTreeMap<ProcessKey, (u32, ObservedProcess)>,
    incomplete: bool,
}
impl ObservedTree {
    pub(super) fn new(root: ObservedProcess) -> Self {
        Self {
            members: BTreeMap::from([(root.key(), (0, root))]),
            incomplete: false,
        }
    }
    pub(super) fn refresh(&mut self, inventory: Inventory) -> Result<(), Fault> {
        let result = self.refresh_inner(inventory);
        if result.is_err() {
            self.incomplete = true;
        }
        result
    }
    fn refresh_inner(&mut self, inventory: Inventory) -> Result<(), Fault> {
        self.incomplete |= !inventory.complete;
        let mut pending = inventory.entries;
        if pending.len() > MAX_PROCESSES {
            return Err(Fault::Bound);
        }
        let started = Instant::now();
        loop {
            let mut remaining = Vec::new();
            let mut changed = false;
            for (parent, process) in pending {
                if started.elapsed() >= Duration::from_secs(2) {
                    return Err(Fault::Bound);
                }
                if self.members.contains_key(&process.key()) {
                    continue;
                }
                if !self.members.keys().any(|key| key.pid == parent) {
                    remaining.push((parent, process));
                    continue;
                }
                let life = process.life(parent)?;
                let mut related = false;
                for (parent_id, observed) in self.members.values() {
                    if observed.key().pid != parent {
                        continue;
                    }
                    match child_of(&life, &observed.life(*parent_id)?) {
                        Ok(true) => {
                            related = true;
                            break;
                        }
                        Ok(false) => {}
                        Err(_) => self.incomplete = true,
                    }
                }
                if related {
                    if self.members.len() >= MAX_FAMILY {
                        return Err(Fault::Bound);
                    }
                    self.members.insert(process.key(), (parent, process));
                    changed = true;
                } else {
                    remaining.push((parent, process));
                }
            }
            if !changed {
                break;
            }
            pending = remaining;
        }
        Ok(())
    }
    pub(super) fn contains(&self, key: ProcessKey) -> bool {
        self.members.contains_key(&key)
    }
    pub(super) fn observed_members_exited(&self) -> Result<bool, Fault> {
        if self.incomplete {
            return Err(Fault::Incomplete);
        }
        for (_, process) in self.members.values() {
            if !process.signalled()? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn shared_home_readiness(&self) -> Result<(), Fault> {
        shared_home_readiness(!self.incomplete)
    }
}
