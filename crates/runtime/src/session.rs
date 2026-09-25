//! CA-05C normalized-event sequencing, NOT wire decoding or execution authority.
#![forbid(unsafe_code)]
use crate::Method;
use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

const REQUEST: Duration = Duration::from_secs(15);
const MAX_PENDING: usize = 8;
const MAX_ISSUED: u64 = 256;
const MAX_NOTIFICATIONS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionPhase {
    AwaitInitialize,
    AwaitInitializeResponse,
    AwaitInitialized,
    Ready,
    Closed,
    Stopped,
}

/// Only fixed local categories. None means credentials are invalid or a helper exited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    Sequence,
    MethodUnavailable,
    Correlation,
    UnsentResponse,
    DuplicateSend,
    InitializationTimeout,
    RequestTimeout,
    TotalTimeout,
    ClockRegression,
    ClockRange,
    PendingLimit,
    IssuedLimit,
    NotificationLimit,
    RemoteError,
    UnexpectedServerRequest,
    InvalidMessage,
    TransportFailure,
    Cancelled,
    IncompleteEof,
    Closed,
}

/// Local request identity, not a chosen wire-ID representation or native receipt.
/// Owner lifetime prevents an old session's local ticket from matching a new one.
#[derive(Clone)]
pub struct RequestId {
    owner: Arc<()>,
    sequence: u64,
}
impl PartialEq for RequestId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.sequence == other.sequence
    }
}
impl Eq for RequestId {}
impl fmt::Debug for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RequestId([REDACTED])")
    }
}

/// Bookkeeping only; no bytes, URL, credentials, executable or native consent.
/// The future driver must register before writing and acknowledge the actual write
/// before delivering responses. A ticket never grants permission to send anything.
pub struct IssuedRequest {
    id: RequestId,
    method: Method,
    deadline: Instant,
}
impl IssuedRequest {
    pub fn id(&self) -> &RequestId {
        &self.id
    }
    pub fn method(&self) -> Method {
        self.method
    }
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
}
impl fmt::Debug for IssuedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IssuedRequest([REDACTED])")
    }
}

/// An input from a future independently schema-validated adapter. Not a JSON-RPC
/// envelope, login success, authenticated identity or caller qualification flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseOutcome {
    Success,
    Error,
}

struct Pending {
    request: IssuedRequest,
    sent: bool,
}

/// Bounded non-login state machine. Production construction is deliberately absent
/// until the version-bound decoder/driver, policy and native lifecycle are reviewed.
/// Tests execute these exact transitions without inventing an official wire schema.
///
/// ```compile_fail
/// use codex_accounts_runtime::ProtocolSession;
/// let session = ProtocolSession::for_test(std::time::Instant::now());
/// ```
///
/// There is no Default, Clone, deserializer, reset, IO, timer task or public factory.
/// A future driver must call poll at its returned deadline AND on every event;
/// a synchronous state machine cannot wake itself or observe a blocked reader.
/// Clock suspend behavior and safe stop/reap/capture remain integration obligations.
pub struct ProtocolSession {
    owner: Arc<()>,
    phase: SessionPhase,
    failure: Option<SessionError>,
    last: Instant,
    initialize_deadline: Instant,
    total_deadline: Instant,
    next: u64,
    pending: Vec<Pending>,
    notifications: usize,
}
impl fmt::Debug for ProtocolSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtocolSession([REDACTED])")
    }
}
impl ProtocolSession {
    pub const INITIALIZATION_BUDGET: Duration = Duration::from_secs(10);
    pub const TOTAL_BUDGET: Duration = Duration::from_secs(30);
    #[cfg(test)]
    fn for_test(start: Instant) -> Result<Self, SessionError> {
        Ok(Self {
            owner: Arc::new(()),
            phase: SessionPhase::AwaitInitialize,
            failure: None,
            last: start,
            initialize_deadline: start
                .checked_add(Self::INITIALIZATION_BUDGET)
                .ok_or(SessionError::ClockRange)?,
            total_deadline: start
                .checked_add(Self::TOTAL_BUDGET)
                .ok_or(SessionError::ClockRange)?,
            next: 1,
            pending: Vec::new(),
            notifications: 0,
        })
    }
    /// Snapshot only, not a current deadline check or permission to run operations.
    pub fn phase(&self) -> SessionPhase {
        self.phase
    }
    pub fn failure(&self) -> Option<SessionError> {
        self.failure
    }
    pub fn pending_requests(&self) -> usize {
        self.pending.len()
    }
    pub fn pending_notifications(&self) -> usize {
        self.notifications
    }

    fn stop<T>(&mut self, error: SessionError) -> Result<T, SessionError> {
        let first = *self.failure.get_or_insert(error);
        self.phase = SessionPhase::Stopped;
        self.pending.clear();
        self.notifications = 0;
        Err(first)
    }
    fn check(&mut self, now: Instant) -> Result<(), SessionError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.phase == SessionPhase::Closed {
            return Err(SessionError::Closed);
        }
        if now.checked_duration_since(self.last).is_none() {
            return self.stop(SessionError::ClockRegression);
        }
        self.last = now;
        // Fixed precedence; all boundaries expire at equality, before an event.
        if now >= self.total_deadline {
            return self.stop(SessionError::TotalTimeout);
        }
        if self.phase != SessionPhase::Ready && now >= self.initialize_deadline {
            return self.stop(SessionError::InitializationTimeout);
        }
        if self.pending.iter().any(|p| now >= p.request.deadline) {
            return self.stop(SessionError::RequestTimeout);
        }
        Ok(())
    }
    /// Earliest absolute deadline. Progress, queue draining and new requests never
    /// extend the initialization or total budget (30s INCLUDING initialization).
    pub fn poll(&mut self, now: Instant) -> Result<Instant, SessionError> {
        self.check(now)?;
        let mut next = self.total_deadline;
        if self.phase != SessionPhase::Ready {
            next = next.min(self.initialize_deadline);
        }
        for pending in &self.pending {
            next = next.min(pending.request.deadline);
        }
        Ok(next)
    }
    pub fn reserve(&mut self, method: Method, now: Instant) -> Result<IssuedRequest, SessionError> {
        self.check(now)?;
        // Login needs its own ID/cancel/reorder and ten-minute lifecycle contract.
        // The allowlist's enum alone cannot opt it into this non-login sequencer.
        if matches!(
            method,
            Method::LoginStart | Method::LoginCancel | Method::Initialized
        ) {
            return self.stop(SessionError::MethodUnavailable);
        }
        if !matches!(
            (self.phase, method),
            (SessionPhase::AwaitInitialize, Method::Initialize)
                | (
                    SessionPhase::Ready,
                    Method::AccountRead | Method::RateLimitsRead
                )
        ) {
            return self.stop(SessionError::Sequence);
        }
        if self.pending.len() == MAX_PENDING {
            return self.stop(SessionError::PendingLimit);
        }
        if self.next > MAX_ISSUED {
            return self.stop(SessionError::IssuedLimit);
        }
        let deadline = if method == Method::Initialize {
            self.initialize_deadline
        } else {
            let Some(deadline) = now.checked_add(REQUEST) else {
                return self.stop(SessionError::ClockRange);
            };
            deadline.min(self.total_deadline)
        };
        let id = RequestId {
            owner: self.owner.clone(),
            sequence: self.next,
        };
        self.next += 1; // MAX_ISSUED checked first; no wrap, reset or re-use.
        self.pending.push(Pending {
            request: IssuedRequest {
                id: id.clone(),
                method,
                deadline,
            },
            sent: false,
        });
        if method == Method::Initialize {
            self.phase = SessionPhase::AwaitInitializeResponse;
        }
        Ok(IssuedRequest {
            id,
            method,
            deadline,
        })
    }
    pub fn sent(&mut self, id: &RequestId, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        let Some(index) = self.pending.iter().position(|p| &p.request.id == id) else {
            return self.stop(SessionError::Correlation);
        };
        if self.pending[index].sent {
            return self.stop(SessionError::DuplicateSend);
        }
        self.pending[index].sent = true;
        Ok(())
    }
    /// Correlates a NORMALIZED response. Wire IDs, schema and pipe-origin mapping
    /// are deliberately not parsed here. Unknown/duplicate/stale replies fail closed.
    pub fn response(
        &mut self,
        id: &RequestId,
        outcome: ResponseOutcome,
        now: Instant,
    ) -> Result<Method, SessionError> {
        self.check(now)?;
        let Some(index) = self.pending.iter().position(|p| &p.request.id == id) else {
            return self.stop(SessionError::Correlation);
        };
        if !self.pending[index].sent {
            return self.stop(SessionError::UnsentResponse);
        }
        if outcome == ResponseOutcome::Error {
            return self.stop(SessionError::RemoteError);
        }
        let method = self.pending.remove(index).request.method;
        if method == Method::Initialize {
            self.phase = SessionPhase::AwaitInitialized;
        }
        Ok(method)
    }
    /// Called only AFTER the future driver successfully writes initialized.
    /// A successful initialize response alone does not allow account requests.
    pub fn initialized_sent(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        if self.phase != SessionPhase::AwaitInitialized {
            return self.stop(SessionError::Sequence);
        }
        self.phase = SessionPhase::Ready;
        Ok(())
    }
    /// Record only an account-change signal from a future qualified decoder, never
    /// payload/identity data. Early signals after initialize is sent are bounded and
    /// may be drained without completing a request or extending any deadline.
    pub fn account_notification(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        let initializing = self.phase == SessionPhase::AwaitInitializeResponse
            && self
                .pending
                .iter()
                .any(|p| p.request.method == Method::Initialize && p.sent);
        if !initializing
            && !matches!(
                self.phase,
                SessionPhase::AwaitInitialized | SessionPhase::Ready
            )
        {
            return self.stop(SessionError::Sequence);
        }
        if self.notifications == MAX_NOTIFICATIONS {
            return self.stop(SessionError::NotificationLimit);
        }
        self.notifications += 1;
        Ok(())
    }
    pub fn take_notification(&mut self, now: Instant) -> Result<bool, SessionError> {
        self.check(now)?;
        let present = self.notifications != 0;
        if present {
            self.notifications -= 1;
        }
        Ok(present)
    }
    pub fn unexpected_server_request(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        self.stop(SessionError::UnexpectedServerRequest)
    }
    pub fn invalid_message(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        self.stop(SessionError::InvalidMessage)
    }
    pub fn transport_failed(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        self.stop(SessionError::TransportFailure)
    }
    pub fn cancel(&mut self, now: Instant) -> Result<(), SessionError> {
        self.check(now)?;
        self.stop(SessionError::Cancelled)
    }
    /// Clean protocol EOF is not a helper-exit, credential or Desktop observation.
    /// Require handshake, all replies and all buffered signals drained first.
    pub fn finish(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.phase == SessionPhase::Closed {
            return Ok(());
        }
        self.check(now)?;
        if self.phase != SessionPhase::Ready || !self.pending.is_empty() || self.notifications != 0
        {
            return self.stop(SessionError::IncompleteEof);
        }
        self.phase = SessionPhase::Closed;
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
