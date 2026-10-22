//! Out-of-band cancellation for one switch. No credentials or runtime authority.
#![forbid(unsafe_code)]

use crate::LoginCancellation;
#[cfg(any(windows, test))]
use codex_accounts_runtime::PipeCancellation;

/// Clone before moving into the worker; `cancel` wakes its owned protocol wait.
/// Cancellation is a request, not a completion receipt. Keep the operation busy
/// until it reports its cleanup/reconciliation result.
#[derive(Clone, Default)]
pub struct SwitchCancellation(LoginCancellation);

impl SwitchCancellation {
    #[cfg(any(windows, test))]
    pub(crate) fn from_login(control: LoginCancellation) -> Self {
        Self(control)
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.cancel();
    }
    #[cfg(any(windows, test))]
    pub(crate) fn cancelled(&self) -> bool {
        self.0.cancelled()
    }
    #[cfg(any(windows, test))]
    pub(crate) fn attach(&self, pipe: PipeCancellation) -> Result<(), crate::OperationError> {
        self.0.attach(pipe)
    }
    #[cfg(any(windows, test))]
    pub(crate) fn detach(&self) {
        self.0.detach();
    }
}
impl std::fmt::Debug for SwitchCancellation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SwitchCancellation([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};

    struct Pending;
    impl Read for Pending {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::WouldBlock.into())
        }
    }
    #[test]
    fn cloned_control_cancels_attached_wait_and_later_attachment() {
        let control = SwitchCancellation::from_login(LoginCancellation::new());
        let clone = control.clone();
        for before_attach in [false, true] {
            let (mut session, pipe) = codex_accounts_runtime::PipeSession::from_nonblocking_io(
                Box::new(io::sink()),
                Box::new(Pending),
                Box::new(Pending),
                false,
            )
            .unwrap();
            session.initialize().unwrap();
            if before_attach {
                clone.cancel();
            }
            control.attach(pipe).unwrap();
            clone.cancel();
            assert!(control.cancelled());
            assert_eq!(
                session.next_event().unwrap_err().primary,
                codex_accounts_runtime::SessionError::Cancelled
            );
            control.detach();
        }
    }
}
