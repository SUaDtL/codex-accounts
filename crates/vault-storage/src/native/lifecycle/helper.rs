//! An owned, noninherited, unnamed job is distinct from observed user processes.
//! CA-04B has no production constructor: only controlled suspended test children
//! can create this owner. Official-runtime creation/stdio remains a later packet.
use super::super::*;
use super::process::ObservedProcess;
use crate::lifecycle_model::{
    owned_exit_observed, owned_member_survey_pending, Fault, ProcessKey, MAX_FAMILY,
};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use windows_sys::Win32::System::JobObjects::*;

pub(super) struct OwnedFamily {
    job: Token,
    root: ObservedProcess,
    retained: BTreeMap<ProcessKey, ObservedProcess>,
    member_observation_incomplete: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ExitObservation {
    pub forced: bool,
    pub observed_members: usize,
    pub member_observation_incomplete: bool,
}
impl std::fmt::Debug for OwnedFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedFamily([REDACTED])")
    }
}
impl OwnedFamily {
    #[cfg(test)]
    pub(super) fn for_suspended_test_child(handle: HANDLE, pid: u32) -> Result<Self, Fault> {
        // This constructor is absent from production. The test owns CreateProcessW's
        // original process/thread handles and assigns before resuming its child.
        let job = unsafe { CreateJobObjectW(null(), null()) };
        if job.is_null() {
            return Err(Fault::Incomplete);
        }
        let job = Token(job);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        limits.BasicLimitInformation.ActiveProcessLimit = MAX_FAMILY as u32;
        // SAFETY: owned new job, exact SDK input. Neither breakaway flag is enabled.
        if unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(Fault::Incomplete);
        }
        let root = ObservedProcess::open(pid)?;
        // SAFETY: exclusively created suspended synthetic child, not a discovered PID.
        if unsafe { AssignProcessToJobObject(job.0, handle) } == 0 {
            return Err(Fault::Incomplete);
        }
        let family = Self {
            job,
            root,
            retained: BTreeMap::new(),
            member_observation_incomplete: false,
        };
        family.check_limits()?;
        Ok(family)
    }
    fn check_limits(&self) -> Result<(), Fault> {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        // SAFETY: owned job and exact output buffer. Unknown/broadened limits refuse.
        if unsafe {
            QueryInformationJobObject(
                self.job.0,
                JobObjectExtendedLimitInformation,
                (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
                null_mut(),
            )
        } == 0
        {
            return Err(Fault::Incomplete);
        }
        if limits.BasicLimitInformation.LimitFlags
            != (JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS)
            || limits.BasicLimitInformation.ActiveProcessLimit != MAX_FAMILY as u32
        {
            return Err(Fault::Changed);
        }
        Ok(())
    }
    fn active(&self) -> Result<u32, Fault> {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: exact SDK output and owned job.
        if unsafe {
            QueryInformationJobObject(
                self.job.0,
                JobObjectBasicAccountingInformation,
                (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
                null_mut(),
            )
        } == 0
        {
            return Err(Fault::Incomplete);
        }
        if info.ActiveProcesses as usize > MAX_FAMILY {
            return Err(Fault::Bound);
        }
        Ok(info.ActiveProcesses)
    }
    fn retain_members(&mut self) -> Result<(), Fault> {
        // The x64 SDK list has two u32 header fields followed by usize PIDs.
        let size = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList)
            + MAX_FAMILY * std::mem::size_of::<usize>();
        let mut buffer = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
        let list = buffer
            .as_mut_ptr()
            .cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>();
        // SAFETY: aligned, initialized bounded variable-length SDK output.
        if unsafe {
            QueryInformationJobObject(
                self.job.0,
                JobObjectBasicProcessIdList,
                list.cast(),
                size as u32,
                null_mut(),
            )
        } == 0
        {
            return Err(Fault::Incomplete);
        }
        let count = unsafe { (*list).NumberOfProcessIdsInList } as usize;
        if count > MAX_FAMILY || unsafe { (*list).NumberOfAssignedProcesses } as usize != count {
            return Err(Fault::Incomplete);
        }
        // SAFETY: header count is bounded against the exact allocated tail capacity.
        let ids = unsafe { std::slice::from_raw_parts((*list).ProcessIdList.as_ptr(), count) };
        for raw in ids {
            let pid = u32::try_from(*raw).map_err(|_| Fault::Incomplete)?;
            let mut already_retained = false;
            for process in self.retained.values().filter(|p| p.key().pid == pid) {
                if !process.signalled()? {
                    already_retained = true;
                    break;
                }
            }
            if already_retained {
                continue;
            }
            let process = match ObservedProcess::open(pid) {
                Ok(p) => p,
                // Disappearance is not exit evidence. The later empty job check
                // independently proves no member remains, including a missed child.
                Err(Fault::Disappeared) => continue,
                Err(e) => return Err(e),
            };
            let mut belongs = 0;
            // SAFETY: both handles are retained. A reused unrelated PID is not adopted.
            if unsafe { IsProcessInJob(process.raw(), self.job.0, &mut belongs) } == 0
                || belongs == 0
            {
                return Err(Fault::Changed);
            }
            if !self.retained.contains_key(&process.key()) && self.retained.len() >= MAX_FAMILY {
                return Err(Fault::Bound);
            }
            self.retained.entry(process.key()).or_insert(process);
        }
        Ok(())
    }
    pub(super) fn poll_exit(&mut self) -> Result<bool, Fault> {
        self.check_limits()?;
        if self.exit_observed()? {
            return Ok(true);
        }
        if let Err(error) = self.retain_members() {
            if !owned_member_survey_pending(error) {
                return Err(error);
            }
            // Inspection may race with exit or remain denied. Preserve incomplete
            // survey evidence; do not relabel denial as absence or restart a timer.
            // False remains pending. Only independent signals AND empty job count
            // can become true, even after an incomplete or missed member survey.
            self.member_observation_incomplete = true;
        }
        self.exit_observed()
    }
    fn exit_observed(&self) -> Result<bool, Fault> {
        let root = self.root.signalled()?;
        let active = self.active()?;
        let mut retained = true;
        for process in self.retained.values() {
            retained &= process.signalled()?;
        }
        owned_exit_observed(root, active, retained)
    }
    fn wait(&mut self, limit: Duration) -> Result<bool, Fault> {
        let start = Instant::now();
        loop {
            if self.poll_exit()? {
                return Ok(true);
            }
            if start.elapsed() >= limit {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(5).min(limit.saturating_sub(start.elapsed())));
        }
    }
    /// Later stdio integration must close/cancel the owned runtime's input first.
    /// These fixed budgets are not renderer-configurable and never apply to user apps.
    pub(super) fn shutdown_after_input_closed(&mut self) -> Result<ExitObservation, Fault> {
        self.shutdown(Duration::from_secs(5), Duration::from_secs(5))
    }
    fn shutdown(&mut self, grace: Duration, after: Duration) -> Result<ExitObservation, Fault> {
        if self.wait(grace)? {
            return Ok(ExitObservation {
                forced: false,
                observed_members: self.retained.len(),
                member_observation_incomplete: self.member_observation_incomplete,
            });
        }
        self.check_limits()?;
        // SAFETY: only the owner of the private, newly created job can reach this
        // call, and ONLY after its grace period actually expired. Never a PID kill.
        if unsafe { TerminateJobObject(self.job.0, 1) } == 0 {
            return Err(Fault::HelperStuck);
        }
        if !self.wait(after).map_err(|_| Fault::HelperStuck)? {
            return Err(Fault::HelperStuck);
        }
        Ok(ExitObservation {
            forced: true,
            observed_members: self.retained.len(),
            member_observation_incomplete: self.member_observation_incomplete,
        })
    }
    #[cfg(test)]
    pub(super) fn short_test_shutdown(&mut self) -> Result<ExitObservation, Fault> {
        self.shutdown(Duration::from_millis(40), Duration::from_secs(5))
    }
}
