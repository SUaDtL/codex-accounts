//! CA-05D internal normalized login events, not an official schema or IO adapter.
#![forbid(unsafe_code)]
use super::*;
use zeroize::Zeroizing;

/// Exact bounded opaque ID bytes from a future qualified decoder, with reader/session
/// ownership. No public constructor, wire-format assumption or diagnostic disclosure.
/// ```compile_fail
/// use codex_accounts_runtime::LoginId;
/// let id = LoginId { owner: std::sync::Arc::new(()), bytes: Default::default() };
/// ```
#[derive(Clone)]
pub struct LoginId {
    owner: Arc<()>,
    bytes: Zeroizing<Vec<u8>>,
}
impl PartialEq for LoginId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.bytes == other.bytes
    }
}
impl Eq for LoginId {}
impl fmt::Debug for LoginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoginId([REDACTED])")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginCompletion {
    Success,
    Failure,
}
/// Protocol cancellation evidence only. Acknowledged never means helper exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationState {
    NotRequested,
    Unavailable,
    WaitingForId,
    Ready,
    WaitingForAck,
    Acknowledged,
    Failed,
    TimedOut,
}

pub(super) struct LoginFlow {
    deadline: Option<Instant>,
    id: Option<LoginId>,
    early: Vec<(LoginId, LoginCompletion)>,
    report: Option<LoginCompletion>,
    completion_seen: bool,
    cancel: CancellationState,
    cancel_deadline: Option<Instant>,
    cancel_error: Option<SessionError>,
}
impl ProtocolSession {
    pub const LOGIN_BUDGET: Duration = Duration::from_secs(600);
    /// Conservative protocol-cleanup budget, not extra time to complete login.
    /// Driver must still stop/reap/capture regardless of cancellation outcome.
    pub const CANCELLATION_BUDGET: Duration = Duration::from_secs(5);
    #[cfg(test)]
    fn for_login_test(now: Instant) -> Result<Self, SessionError> {
        let mut session = Self::for_test(now)?;
        session.login = Some(LoginFlow {
            deadline: None,
            id: None,
            early: Vec::new(),
            report: None,
            completion_seen: false,
            cancel: CancellationState::NotRequested,
            cancel_deadline: None,
            cancel_error: None,
        });
        Ok(session)
    }
    #[cfg(test)]
    fn login_id_for_test(&self, bytes: &[u8]) -> Result<LoginId, SessionError> {
        if bytes.is_empty() || bytes.len() > 256 {
            return Err(SessionError::LoginIdInvalid);
        }
        Ok(LoginId {
            owner: self.owner.clone(),
            bytes: Zeroizing::new(bytes.to_vec()),
        })
    }
    fn validate_login_id(&mut self, id: &LoginId) -> Result<(), SessionError> {
        if !Arc::ptr_eq(&self.owner, &id.owner) {
            return self.stop(SessionError::Correlation);
        }
        if id.bytes.is_empty() || id.bytes.len() > 256 {
            return self.stop(SessionError::LoginIdInvalid);
        }
        Ok(())
    }
    pub(super) fn login_started(&self) -> bool {
        self.login_deadline().is_some()
    }
    pub(super) fn login_deadline(&self) -> Option<Instant> {
        self.login.as_ref().and_then(|l| l.deadline)
    }
    pub(super) fn login_budget_expired(&self, now: Instant) -> bool {
        self.login_deadline().is_some_and(|d| now >= d)
    }
    pub(super) fn early_login_notifications(&self) -> usize {
        self.login.as_ref().map_or(0, |l| l.early.len())
    }
    pub(super) fn stop_login(&mut self, error: SessionError) {
        if let Some(l) = &mut self.login {
            l.early.clear();
            l.report = None;
            if matches!(
                l.cancel,
                CancellationState::WaitingForId
                    | CancellationState::Ready
                    | CancellationState::WaitingForAck
            ) {
                l.cancel_error.get_or_insert(error);
                l.cancel = if error == SessionError::CancellationTimeout {
                    CancellationState::TimedOut
                } else {
                    CancellationState::Failed
                };
            }
        }
    }
    pub fn login_report(&self) -> Option<LoginCompletion> {
        self.login.as_ref().and_then(|l| l.report)
    }
    pub fn cancellation_state(&self) -> CancellationState {
        self.login
            .as_ref()
            .map_or(CancellationState::NotRequested, |l| l.cancel)
    }
    pub fn cancellation_error(&self) -> Option<SessionError> {
        self.login.as_ref().and_then(|l| l.cancel_error)
    }
    /// Borrowed target only for the future private cancellation encoder. No raw-byte
    /// getter or URL is exposed, and this reference is not permission to send.
    pub fn cancellation_target(&self) -> Option<&LoginId> {
        self.login
            .as_ref()
            .filter(|l| {
                matches!(
                    l.cancel,
                    CancellationState::Ready | CancellationState::WaitingForAck
                )
            })
            .and_then(|l| l.id.as_ref())
    }
    /// Only the privately selected login lane may reserve one managed login. The
    /// unchanged generic reserve() refuses login enum values; no caller flag opts in.
    /// ```compile_fail
    /// use codex_accounts_runtime::ProtocolSession;
    /// let login = ProtocolSession::for_login_test(std::time::Instant::now());
    /// ```
    pub fn start_login(&mut self, now: Instant) -> Result<IssuedRequest, SessionError> {
        self.check(now)?;
        if self.login.is_none() {
            return self.stop(SessionError::MethodUnavailable);
        }
        if self.phase != SessionPhase::Ready
            || !self.pending.is_empty()
            || self.notifications != 0
            || self.login_started()
        {
            return self.stop(SessionError::Sequence);
        }
        let Some(deadline) = now.checked_add(Self::LOGIN_BUDGET) else {
            return self.stop(SessionError::ClockRange);
        };
        let Some(response_deadline) = now.checked_add(REQUEST) else {
            return self.stop(SessionError::ClockRange);
        };
        let ticket = self.issue(
            Method::LoginStart,
            response_deadline.min(self.total_deadline),
        )?;
        if let Some(l) = &mut self.login {
            l.deadline = Some(deadline);
        }
        self.phase = SessionPhase::AwaitLoginStart;
        Ok(ticket)
    }
    fn check_login_event(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.phase == SessionPhase::LoginCancelling {
            self.check_cancellation(now)
        } else {
            self.check(now)
        }
    }
    /// The correlated start response supplies the authoritative login ID. An early
    /// completion is buffered until here; mismatched or duplicate IDs fail closed.
    pub fn login_start_response(
        &mut self,
        request: &RequestId,
        id: &LoginId,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_login_event(now)?;
        self.validate_login_id(id)?;
        if !matches!(
            self.phase,
            SessionPhase::AwaitLoginStart | SessionPhase::LoginCancelling
        ) {
            return self.stop(SessionError::Sequence);
        }
        let index = self.matching_sent(request)?;
        if self.pending[index].request.method != Method::LoginStart {
            return self.stop(SessionError::Sequence);
        }
        if self
            .login
            .as_ref()
            .is_none_or(|l| l.id.is_some() || l.early.iter().any(|(early, _)| early != id))
        {
            return self.stop(SessionError::Correlation);
        }
        self.pending.remove(index);
        let cancelling = self.phase == SessionPhase::LoginCancelling;
        let early = if let Some(l) = &mut self.login {
            l.id = Some(id.clone());
            let early = l.early.pop().map(|(_, result)| result);
            l.early.clear();
            if cancelling {
                l.cancel = CancellationState::Ready;
            }
            early
        } else {
            return self.stop(SessionError::Sequence);
        };
        if !cancelling {
            self.phase = SessionPhase::AwaitLoginCompletion;
        }
        if let Some(result) = early {
            self.record_completion(result, cancelling)?;
        }
        Ok(())
    }
    /// Normalized start error cannot be misinterpreted as a successful login.
    pub fn login_start_error(
        &mut self,
        request: &RequestId,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_login_event(now)?;
        let index = self.matching_sent(request)?;
        if self.pending[index].request.method != Method::LoginStart {
            return self.stop(SessionError::Sequence);
        }
        self.stop(SessionError::RemoteError)
    }
    pub fn login_completed(
        &mut self,
        id: &LoginId,
        result: LoginCompletion,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_login_event(now)?;
        self.validate_login_id(id)?;
        if !matches!(
            self.phase,
            SessionPhase::AwaitLoginStart
                | SessionPhase::AwaitLoginCompletion
                | SessionPhase::LoginCancelling
        ) {
            return self.stop(SessionError::Sequence);
        }
        let Some(l) = &self.login else {
            return self.stop(SessionError::Sequence);
        };
        if let Some(expected) = &l.id {
            if expected != id || l.completion_seen {
                return self.stop(SessionError::Correlation);
            }
            self.record_completion(result, self.phase == SessionPhase::LoginCancelling)
        } else {
            if !self
                .pending
                .iter()
                .any(|p| p.request.method == Method::LoginStart && p.sent)
            {
                return self.stop(SessionError::UnsentResponse);
            }
            if l.early.iter().any(|(early, _)| early == id) {
                return self.stop(SessionError::Correlation);
            }
            if self.notifications + l.early.len() >= MAX_NOTIFICATIONS {
                return self.stop(SessionError::NotificationLimit);
            }
            if let Some(l) = &mut self.login {
                l.early.push((id.clone(), result));
            }
            Ok(())
        }
    }
    fn record_completion(
        &mut self,
        result: LoginCompletion,
        cancelling: bool,
    ) -> Result<(), SessionError> {
        if let Some(l) = &mut self.login {
            l.completion_seen = true;
            if cancelling {
                return Ok(());
            } // Never reverse a cancellation intent.
            if result == LoginCompletion::Success {
                l.report = Some(result);
                self.phase = SessionPhase::LoginReported;
                return Ok(());
            }
        }
        self.stop(SessionError::LoginFailed)
    }
    pub(super) fn begin_cancellation(
        &mut self,
        reason: SessionError,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.failure.get_or_insert(reason);
        self.notifications = 0;
        let awaiting_start = self
            .pending
            .iter()
            .find(|p| p.request.method == Method::LoginStart && p.sent)
            .map(|p| p.request.deadline);
        let Some(l) = &mut self.login else {
            return self.stop(reason);
        };
        l.early.clear();
        if l.report.is_some() || (l.id.is_none() && awaiting_start.is_none()) {
            l.cancel = CancellationState::Unavailable;
            return self.stop(reason);
        }
        let Some(deadline) = now.checked_add(Self::CANCELLATION_BUDGET) else {
            return self.stop(SessionError::ClockRange);
        };
        l.cancel_deadline = Some(awaiting_start.map_or(deadline, |d| d.min(deadline)));
        l.cancel = if l.id.is_some() {
            CancellationState::Ready
        } else {
            CancellationState::WaitingForId
        };
        self.phase = SessionPhase::LoginCancelling;
        Err(reason)
    }
    pub(super) fn check_cancellation(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.phase != SessionPhase::LoginCancelling {
            return Err(self.failure.unwrap_or(SessionError::Sequence));
        }
        if now.checked_duration_since(self.last).is_none() {
            return self.stop(SessionError::ClockRegression);
        }
        self.last = now;
        let Some(deadline) = self.login.as_ref().and_then(|l| l.cancel_deadline) else {
            return self.stop(SessionError::Sequence);
        };
        if now >= deadline {
            return self.stop(SessionError::CancellationTimeout);
        }
        Ok(())
    }
    /// Once cancel/timeout has made ordinary poll return the primary error, use
    /// this separate bounded shutdown lane. It never returns permission to login.
    pub fn poll_cancellation(&mut self, now: Instant) -> Result<Instant, SessionError> {
        self.check_cancellation(now)?;
        self.login
            .as_ref()
            .and_then(|l| l.cancel_deadline)
            .ok_or(SessionError::Sequence)
    }
    pub fn reserve_login_cancel(
        &mut self,
        now: Instant,
    ) -> Result<Option<IssuedRequest>, SessionError> {
        self.check_cancellation(now)?;
        match self.cancellation_state() {
            CancellationState::WaitingForId => Ok(None),
            CancellationState::Ready => {
                let deadline = self.poll_cancellation(now)?;
                let ticket = self.issue(Method::LoginCancel, deadline)?;
                if let Some(l) = &mut self.login {
                    l.cancel = CancellationState::WaitingForAck;
                }
                Ok(Some(ticket))
            }
            _ => self.stop(SessionError::DuplicateSend),
        }
    }
    pub fn login_cancel_response(
        &mut self,
        request: &RequestId,
        outcome: ResponseOutcome,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.check_cancellation(now)?;
        let index = self.matching_sent(request)?;
        if self.pending[index].request.method != Method::LoginCancel {
            return self.stop(SessionError::Sequence);
        }
        if outcome == ResponseOutcome::Error {
            return self.stop(SessionError::RemoteError);
        }
        self.pending.clear();
        self.notifications = 0;
        if let Some(l) = &mut self.login {
            l.early.clear();
            l.cancel = CancellationState::Acknowledged;
        }
        self.phase = SessionPhase::Stopped;
        Err(self.failure.unwrap_or(SessionError::Sequence))
    }
}

#[cfg(test)]
#[path = "session_login_tests.rs"]
mod tests;
