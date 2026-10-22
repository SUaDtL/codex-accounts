//! Shared metadata/cancellation only; never contains a URL, account or path.
use codex_accounts_runtime::PipeCancellation;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LoginStage {
    Preparing,
    WaitingForBrowser,
    Saving,
    Cleaning,
    Finished,
}
impl LoginStage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::WaitingForBrowser => "waiting_for_browser",
            Self::Saving => "saving",
            Self::Cleaning => "cleaning",
            Self::Finished => "finished",
        }
    }
}
struct Shared {
    cancelled: AtomicBool,
    stage: AtomicU8,
    pipe: Mutex<Option<PipeCancellation>>,
}
#[derive(Clone)]
pub struct LoginCancellation(Arc<Shared>);
impl std::fmt::Debug for LoginCancellation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginCancellation")
            .field("stage", &self.state())
            .finish_non_exhaustive()
    }
}
impl Default for LoginCancellation {
    fn default() -> Self {
        Self::new()
    }
}
impl LoginCancellation {
    pub fn new() -> Self {
        Self(Arc::new(Shared {
            cancelled: AtomicBool::new(false),
            stage: AtomicU8::new(0),
            pipe: Mutex::new(None),
        }))
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        if let Ok(pipe) = self.0.pipe.lock() {
            if let Some(pipe) = pipe.as_ref() {
                pipe.request();
            }
        }
    }
    pub fn request(&self) {
        self.cancel();
    }
    pub fn state(&self) -> LoginStage {
        match self.0.stage.load(Ordering::Acquire) {
            0 => LoginStage::Preparing,
            1 => LoginStage::WaitingForBrowser,
            2 => LoginStage::Saving,
            3 => LoginStage::Cleaning,
            _ => LoginStage::Finished,
        }
    }
    pub(crate) fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn stage(&self, stage: LoginStage) {
        self.0.stage.store(stage as u8, Ordering::Release);
    }
    pub(crate) fn attach(&self, pipe: PipeCancellation) -> Result<(), crate::OperationError> {
        let mut guard = self
            .0
            .pipe
            .lock()
            .map_err(|_| crate::OperationError("E_CANCELLED"))?;
        if self.cancelled() {
            pipe.request();
        }
        *guard = Some(pipe);
        Ok(())
    }
    pub(crate) fn detach(&self) {
        if let Ok(mut pipe) = self.0.pipe.lock() {
            *pipe = None;
        }
    }
}
