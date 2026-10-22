//! Production protocol access through backend-owned nonblocking pipe handles.
//! This layer never selects, launches, trusts or reaps a process, or reads auth files.
#![forbid(unsafe_code)]
use super::*;
pub use codec::config::{
    AllowedLoginMethods, ChatGptEndpoint, ConfigObservation, ConfigSettings, CredentialStore,
    LoginMethod, ObjectSummary, Observed, PolicySettings, Provider, RequirementsObservation,
    StringOverride, WorkspaceRestriction,
};
use std::io::{Read, Write};
use std::sync::mpsc::sync_channel;

/// Potentially sensitive runtime declarations. Explicit access; redacted Debug.
pub struct RuntimeText(Zeroizing<String>);
impl RuntimeText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn from_secret(value: codec::Secret) -> Self {
        Self(Zeroizing::new(value.as_str().to_owned()))
    }
}
impl fmt::Debug for RuntimeText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RuntimeText([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipeSessionError {
    pub primary: SessionError,
    pub cancellation: CancellationState,
    pub cancellation_error: Option<SessionError>,
}
impl From<DriverError> for PipeSessionError {
    fn from(value: DriverError) -> Self {
        Self {
            primary: value.primary,
            cancellation: value.cancellation,
            cancellation_error: value.cancellation_error,
        }
    }
}
impl From<SessionError> for PipeSessionError {
    fn from(primary: SessionError) -> Self {
        Self {
            primary,
            cancellation: CancellationState::NotRequested,
            cancellation_error: None,
        }
    }
}

/// Wire presence only: missing or null does not prove credential absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountPresence {
    Missing,
    Null,
    Managed,
}

/// Numeric wire metadata only. Percentages retain the schema's signed width;
/// presentation may clamp a progress bar but must not invent an absent quota.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimitWindow {
    pub used_percent: i32,
    pub duration_mins: Observed<i64>,
    /// Unix timestamp in seconds, when supplied by the helper.
    pub resets_at: Observed<i64>,
}

/// The response's default rateLimits snapshot. Account IDs, bucket IDs, names,
/// model text and credit details never cross this metadata boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimitSummary {
    pub primary: Observed<RateLimitWindow>,
    pub secondary: Observed<RateLimitWindow>,
    pub plan: Observed<&'static str>,
}
fn rate_presence<T, U>(value: codec::Presence<T>, project: impl FnOnce(T) -> U) -> Observed<U> {
    match value {
        codec::Presence::Missing => Observed::Missing,
        codec::Presence::Null => Observed::Null,
        codec::Presence::Present(value) => Observed::Value(project(value)),
    }
}
impl From<codec::Snapshot> for RateLimitSummary {
    fn from(value: codec::Snapshot) -> Self {
        Self {
            primary: rate_presence(value.primary, |window| RateLimitWindow {
                used_percent: window.used_percent,
                duration_mins: rate_presence(window.duration_mins, |value| value),
                resets_at: rate_presence(window.resets_at, |value| value),
            }),
            secondary: rate_presence(value.secondary, |window| RateLimitWindow {
                used_percent: window.used_percent,
                duration_mins: rate_presence(window.duration_mins, |value| value),
                resets_at: rate_presence(window.resets_at, |value| value),
            }),
            plan: rate_presence(value.plan, |plan| plan.0),
        }
    }
}

/// Metadata observations, not authenticated identity or Desktop confirmation.
pub enum PipeEvent {
    /// Bounded unsolicited metadata with no request/account acceptance meaning.
    IgnoredNotification,
    Config(ConfigObservation),
    Requirements(RequirementsObservation),
    Written(Method),
    Initialized {
        home: RuntimeText,
        platform_family: RuntimeText,
        platform_os: RuntimeText,
        user_agent: RuntimeText,
    },
    Account {
        email: Option<RuntimeText>,
        plan: Option<&'static str>,
        presence: AccountPresence,
        requires_openai_auth: bool,
    },
    LoginStarted {
        auth_url: RuntimeText,
    },
    LoginCompleted(LoginCompletion),
    LoginCompletionBuffered,
    AccountUpdated,
    /// A response matched by the codec to this session's outstanding request.
    RateLimits(RateLimitSummary),
    /// Unsolicited metadata; never completes a requested account check.
    RateLimitsUpdated(RateLimitSummary),
    CancellationProgress,
    CancellationAcknowledged,
    Closed,
}
impl fmt::Debug for PipeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PipeEvent([REDACTED])")
    }
}

#[derive(Clone)]
pub struct PipeCancellation(io::CancelHandle);
impl PipeCancellation {
    pub fn request(&self) {
        self.0.request();
    }
}

/// Owns only pipe endpoints, sequencer and codec. Drop closes all local handles.
/// The native owner must stop/reap its contained helper before credential effects.
pub struct PipeSession {
    session: ProtocolSession,
    driver: TransportDriver,
    codec: codec::Codec,
}
impl fmt::Debug for PipeSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PipeSession([REDACTED])")
    }
}
impl PipeSession {
    /// Accept only backend-validated nonblocking pipe adapters. Read must return
    /// WouldBlock for an empty open pipe, and Ok(0) only for actual EOF. Windows
    /// File loses this distinction for PIPE_NOWAIT and must not be used
    /// directly. Write must acknowledge only the bytes actually written, with
    /// zero/WouldBlock for backpressure. The native owner validates pipe mode,
    /// maps native errors, and owns process/job cleanup. No renderer-provided IO.
    /// No worker threads are spawned; all IO is nonblocking with 10ms maximum
    /// idle polling and existing absolute initialization/request/login deadlines.
    pub fn from_nonblocking_io(
        stdin: Box<dyn Write + Send>,
        stdout: Box<dyn Read + Send>,
        stderr: Box<dyn Read + Send>,
        login: bool,
    ) -> Result<(Self, PipeCancellation), PipeSessionError> {
        let session = if login {
            ProtocolSession::new_login(Instant::now())?
        } else {
            ProtocolSession::new(Instant::now())?
        };
        let reader = Arc::new(());
        let (wake_tx, wake) = sync_channel(1);
        let cancel = io::cancellation(&wake_tx);
        let driver = TransportDriver {
            owner: session.owner.clone(),
            stdout: io::file_reader(stdout, &reader, Stream::Output, &wake_tx),
            stderr: io::file_reader(stderr, &reader, Stream::Error, &wake_tx),
            stdin: io::file_writer(stdin, &reader, &wake_tx),
            reader,
            wake,
            cancel: cancel.requested.clone(),
            decoder: JsonObjectDecoder::default(),
            chunk: None,
            cursor: 0,
            ready: None,
            lease: Weak::new(),
            write: None,
            stderr_total: 0,
            stdout_eof: false,
            stderr_eof: false,
            cancel_seen: false,
            cancel_notified: false,
            closed: false,
            first_error: None,
        };
        let codec = codec::Codec::new(&driver, &session);
        Ok((
            Self {
                session,
                driver,
                codec,
            },
            PipeCancellation(cancel),
        ))
    }

    pub fn initialize(&mut self) -> Result<(), PipeSessionError> {
        self.queue(Method::Initialize)
    }
    pub fn initialized(&mut self) -> Result<(), PipeSessionError> {
        self.codec
            .initialized(&mut self.session, &mut self.driver)
            .map_err(Into::into)
    }
    pub fn read_account(&mut self) -> Result<(), PipeSessionError> {
        self.queue(Method::AccountRead)
    }
    pub fn read_config(&mut self) -> Result<(), PipeSessionError> {
        self.queue(Method::ConfigRead)
    }
    pub fn read_config_requirements(&mut self) -> Result<(), PipeSessionError> {
        self.queue(Method::ConfigRequirementsRead)
    }
    pub fn read_rate_limits(&mut self) -> Result<(), PipeSessionError> {
        self.queue(Method::RateLimitsRead)
    }
    pub fn start_login(&mut self) -> Result<(), PipeSessionError> {
        self.codec
            .start_login(&mut self.session, &mut self.driver)
            .map(|_| ())
            .map_err(Into::into)
    }
    pub fn close_input(&mut self) -> Result<(), PipeSessionError> {
        self.driver
            .close_input(&mut self.session)
            .map_err(Into::into)
    }
    pub fn phase(&self) -> SessionPhase {
        self.session.phase()
    }
    fn queue(&mut self, method: Method) -> Result<(), PipeSessionError> {
        self.codec
            .queue(&mut self.session, &mut self.driver, method)
            .map(|_| ())
            .map_err(Into::into)
    }

    fn delivered_notification(&mut self) -> Result<(), PipeSessionError> {
        // The codec has already checked the notification's syntax, binding,
        // phase and deadline, including the cancellation-specific deadline.
        // take_notification() re-enters the normal session check, which returns
        // the sticky cancellation reason during LoginCancelling. Consume only
        // this validated receipt directly; ignored notifications use their own
        // codec counter and must never pass through this method.
        let Some(remaining) = self.session.notifications.checked_sub(1) else {
            let _ = self.session.stop::<()>(SessionError::Sequence);
            return Err(self
                .driver
                .fail(&mut self.session, DriverFault::Session)
                .into());
        };
        self.session.notifications = remaining;
        Ok(())
    }

    /// Returns one bounded event. Login cancellation is serialized through the
    /// existing cancellation lane and never counts as proof the helper exited.
    pub fn next_event(&mut self) -> Result<PipeEvent, PipeSessionError> {
        match self.driver.next(&mut self.session)? {
            DriverEvent::Written(method) => Ok(PipeEvent::Written(method)),
            DriverEvent::Closed => Ok(PipeEvent::Closed),
            DriverEvent::CancellationRequired => {
                self.codec
                    .cancel_login(&mut self.session, &mut self.driver)?;
                Ok(PipeEvent::CancellationProgress)
            }
            DriverEvent::Frame(frame) => {
                let event = self
                    .codec
                    .consume(&mut self.session, &mut self.driver, frame)?;
                match event {
                    codec::CodecEvent::IgnoredNotification => Ok(PipeEvent::IgnoredNotification),
                    codec::CodecEvent::Config(observation) => Ok(PipeEvent::Config(observation)),
                    codec::CodecEvent::Requirements(observation) => {
                        Ok(PipeEvent::Requirements(observation))
                    }
                    codec::CodecEvent::Initialized(observation) => Ok(PipeEvent::Initialized {
                        home: RuntimeText::from_secret(observation.codex_home),
                        platform_family: RuntimeText::from_secret(observation.platform_family),
                        platform_os: RuntimeText::from_secret(observation.platform_os),
                        user_agent: RuntimeText::from_secret(observation.user_agent),
                    }),
                    codec::CodecEvent::Account(observation) => {
                        let (email, plan, presence) = match observation.account {
                            codec::AccountPresence::Managed(account) => (
                                account.email.map(RuntimeText::from_secret),
                                Some(account.plan),
                                AccountPresence::Managed,
                            ),
                            codec::AccountPresence::Missing => {
                                (None, None, AccountPresence::Missing)
                            }
                            codec::AccountPresence::Null => (None, None, AccountPresence::Null),
                        };
                        Ok(PipeEvent::Account {
                            email,
                            plan,
                            presence,
                            requires_openai_auth: observation.requires_openai_auth,
                        })
                    }
                    codec::CodecEvent::LoginStarted { auth_url } => Ok(PipeEvent::LoginStarted {
                        auth_url: RuntimeText::from_secret(auth_url),
                    }),
                    codec::CodecEvent::LoginCompletion { outcome, .. } => {
                        Ok(PipeEvent::LoginCompleted(outcome))
                    }
                    codec::CodecEvent::AccountUpdated(_) => {
                        self.delivered_notification()?;
                        Ok(PipeEvent::AccountUpdated)
                    }
                    codec::CodecEvent::RateLimits(value) => {
                        Ok(PipeEvent::RateLimits(value.rate_limits.into()))
                    }
                    codec::CodecEvent::RateLimitsUpdated(value) => {
                        self.delivered_notification()?;
                        Ok(PipeEvent::RateLimitsUpdated(value.into()))
                    }
                    codec::CodecEvent::CancellationReady => {
                        self.codec
                            .cancel_login(&mut self.session, &mut self.driver)?;
                        Ok(PipeEvent::CancellationProgress)
                    }
                    codec::CodecEvent::CancellationAcknowledged { .. } => {
                        Ok(PipeEvent::CancellationAcknowledged)
                    }
                    codec::CodecEvent::LoginCompletionBuffered => {
                        Ok(PipeEvent::LoginCompletionBuffered)
                    }
                    codec::CodecEvent::CancellationProgress => Ok(PipeEvent::CancellationProgress),
                }
            }
        }
    }
}
