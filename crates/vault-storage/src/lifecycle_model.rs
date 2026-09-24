//! Private observation rules. None of these values is a compatibility receipt.
#![forbid(unsafe_code)]

pub(crate) const MAX_PROCESSES: usize = 4096;
pub(crate) const MAX_FAMILY: usize = 128;
pub(crate) const MAX_WINDOWS: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProcessKey {
    pub pid: u32,
    pub created: u64,
}
impl std::fmt::Debug for ProcessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProcessKey([REDACTED])")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fault {
    AccessDenied,
    Disappeared,
    Incomplete,
    Changed,
    Bound,
    QuitUnavailable,
    Timeout,
    HelperStuck,
    QualificationMissing,
}
#[derive(Clone)]
pub(crate) struct Life {
    pub key: ProcessKey,
    pub parent: u32,
    pub owner: [u8; 32],
    // Only a signalled process handle permits reading the OS exit time.
    pub exited: Option<u64>,
}
impl Life {
    pub fn validate(&self) -> Result<(), Fault> {
        if self.key.pid == 0
            || self.key.created == 0
            || self.owner == [0; 32]
            || self.exited.is_some_and(|t| t < self.key.created)
        {
            return Err(Fault::Incomplete);
        }
        Ok(())
    }
}
/// PPID alone is not association. The child must have been born during this
/// retained parent's lifetime. This is still not proof of a shared Codex home.
pub(crate) fn child_of(child: &Life, parent: &Life) -> Result<bool, Fault> {
    child.validate()?;
    parent.validate()?;
    if child.parent != parent.key.pid
        || child.key == parent.key
        || child.key.created < parent.key.created
        || parent.exited.is_some_and(|t| child.key.created > t)
    {
        return Ok(false);
    }
    if child.owner != parent.owner {
        return Err(Fault::Incomplete);
    }
    Ok(true)
}
/// Polling can miss an intermediate parent and cannot prove home association.
/// Even a complete process snapshot never qualifies a Desktop writer set here.
pub(crate) fn shared_home_readiness(complete_snapshot: bool) -> Result<(), Fault> {
    if !complete_snapshot {
        Err(Fault::Incomplete)
    } else {
        Err(Fault::QualificationMissing)
    }
}
/// Native callers supply observations from an owned, non-breakaway job and
/// retained handles. A successful quit/terminate call is deliberately no input.
pub(crate) fn owned_exit_observed(
    root_signalled: bool,
    active_members: u32,
    retained_signalled: bool,
) -> Result<bool, Fault> {
    if active_members as usize > MAX_FAMILY {
        return Err(Fault::Bound);
    }
    Ok(root_signalled && active_members == 0 && retained_signalled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn life(pid: u32, created: u64, parent: u32) -> Life {
        Life {
            key: ProcessKey { pid, created },
            parent,
            owner: [7; 32],
            exited: None,
        }
    }
    #[test]
    fn reused_parent_pid_does_not_adopt_an_older_child() {
        assert!(!child_of(&life(20, 10, 10), &life(10, 30, 1)).unwrap());
    }
    #[test]
    fn late_descendant_is_retained_after_parent_exit() {
        let mut parent = life(10, 10, 1);
        parent.exited = Some(30);
        assert!(child_of(&life(20, 25, 10), &parent).unwrap());
        assert!(!child_of(&life(21, 31, 10), &parent).unwrap());
    }
    #[test]
    fn different_owner_and_invalid_lifetime_remain_unknown() {
        let parent = life(10, 10, 1);
        let mut child = life(20, 20, 10);
        child.owner = [8; 32];
        assert_eq!(child_of(&child, &parent), Err(Fault::Incomplete));
        child.owner = parent.owner;
        child.exited = Some(1);
        assert_eq!(child_of(&child, &parent), Err(Fault::Incomplete));
    }
    #[test]
    fn incomplete_and_empty_snapshots_never_authorize_shared_home_writes() {
        assert_eq!(shared_home_readiness(false), Err(Fault::Incomplete));
        assert_eq!(
            shared_home_readiness(true),
            Err(Fault::QualificationMissing)
        );
    }
    #[test]
    fn root_exit_signal_receipt_and_zero_count_are_not_interchangeable() {
        for root in [false, true] {
            for active in [0, 1] {
                for retained in [false, true] {
                    assert_eq!(
                        owned_exit_observed(root, active, retained).unwrap(),
                        root && active == 0 && retained
                    );
                }
            }
        }
        assert_eq!(owned_exit_observed(true, 129, true), Err(Fault::Bound));
    }
    #[test]
    fn invalid_zero_identity_and_self_parent_do_not_form_a_tree() {
        let parent = life(10, 10, 10);
        assert!(!child_of(&parent, &parent).unwrap());
        assert_eq!(child_of(&life(0, 10, 10), &parent), Err(Fault::Incomplete));
        assert_eq!(child_of(&life(20, 0, 10), &parent), Err(Fault::Incomplete));
    }
    #[test]
    fn process_keys_do_not_expose_process_identifiers_in_debug() {
        assert_eq!(
            format!("{:?}", life(123456, 987654, 1).key),
            "ProcessKey([REDACTED])"
        );
    }
}
