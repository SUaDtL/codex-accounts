//! CA-05E private transport pump over owned synthetic endpoints.
//! No official codec, OS pipe, process, credential operation or public constructor.
#![forbid(unsafe_code)]
// This entire private boundary has no production constructor until reviewed binding.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;
use crate::{JsonFrameError, JsonObjectDecoder, JsonObjectFrame, MAX_FRAME_BYTES, MAX_PENDING_BYTES};
use std::sync::{atomic::Ordering, mpsc::RecvTimeoutError, Weak};
use zeroize::Zeroize;
#[path = "driver_io.rs"]
mod io;
use io::{Chunk, ReadEnd, ReadStep, Stream, WriteEnd, CHUNK_BYTES};

const STDERR_TOTAL_LIMIT: usize = MAX_PENDING_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DriverFault {
    Session,
    Binding,
    OutputBusy,
    InvalidOutput,
    InputClosed,
    InvalidFrame(JsonFrameError),
    StderrLimit,
    Endpoint,
    OutstandingFrame,
    PartialWriteCancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DriverError {
    fault: DriverFault,
    primary: SessionError,
    cancellation: CancellationState,
    cancellation_error: Option<SessionError>,
}

/// A single leased syntax-checked frame. It is NOT a normalized RPC message.
/// The next driver read refuses until this lease is dropped; no caller-side queue.
struct ReaderFrame {
    owner: Arc<()>,
    reader: Arc<()>,
    lease: Arc<()>,
    frame: JsonObjectFrame,
}
impl ReaderFrame {
    fn bytes<'a>(
        &'a self,
        driver: &TransportDriver,
        session: &ProtocolSession,
    ) -> Result<&'a [u8], DriverFault> {
        if !Arc::ptr_eq(&self.owner, &session.owner)
            || !Arc::ptr_eq(&self.reader, &driver.reader)
            || !Arc::ptr_eq(&self.owner, &driver.owner)
            || driver.first_error.is_some()
        {
            return Err(DriverFault::Binding);
        }
        Ok(self.frame.as_bytes())
    }
}
impl fmt::Debug for ReaderFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Keep the lease alive along with the bytes, without printing either.
        let _ = &self.lease;
        f.write_str("ReaderFrame([REDACTED])")
    }
}
#[derive(Debug)]
enum DriverEvent {
    Written(Method),
    Frame(ReaderFrame),
    CancellationRequired,
    Closed,
}

enum Receipt {
    Request(RequestId, Method),
    Initialized,
}
impl Receipt {
    fn method(&self) -> Method {
        match self {
            Self::Request(_, method) => *method,
            Self::Initialized => Method::Initialized,
        }
    }
}
struct PendingWrite {
    bytes: Zeroizing<Vec<u8>>,
    offset: usize,
    receipt: Receipt,
}

/// This internal transport cannot be constructed or invoked by external callers.
/// ```compile_fail
/// use codex_accounts_runtime::TransportDriver;
/// ```
struct TransportDriver {
    owner: Arc<()>,
    reader: Arc<()>,
    stdout: ReadEnd,
    stderr: ReadEnd,
    stdin: WriteEnd,
    wake: std::sync::mpsc::Receiver<()>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
    decoder: JsonObjectDecoder,
    chunk: Option<Chunk>,
    cursor: usize,
    ready: Option<JsonObjectFrame>,
    lease: Weak<()>,
    write: Option<PendingWrite>,
    stderr_total: usize,
    stdout_eof: bool,
    stderr_eof: bool,
    cancel_seen: bool,
    cancel_notified: bool,
    closed: bool,
    first_error: Option<DriverError>,
}
impl fmt::Debug for TransportDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TransportDriver([REDACTED])")
    }
}
impl TransportDriver {
    fn fail(&mut self, session: &mut ProtocolSession, fault: DriverFault) -> DriverError {
        if let Some(error) = self.first_error {
            return error;
        }
        match fault {
            DriverFault::Session => {}
            DriverFault::InvalidFrame(_) | DriverFault::InvalidOutput => {
                let _ = session.invalid_message(Instant::now());
            }
            _ => {
                let _ = session.transport_failed(Instant::now());
            }
        }
        let error = DriverError {
            fault,
            primary: session.failure().unwrap_or(SessionError::TransportFailure),
            cancellation: session.cancellation_state(),
            cancellation_error: session.cancellation_error(),
        };
        self.first_error = Some(error);
        self.write = None;
        self.chunk = None;
        self.ready = None;
        self.decoder = JsonObjectDecoder::default();
        self.stdin.close();
        self.stdout.close();
        self.stderr.close();
        error
    }

    fn gate(&mut self, session: &mut ProtocolSession) -> Result<Instant, DriverError> {
        if let Some(error) = self.first_error {
            return Err(error);
        }
        if !Arc::ptr_eq(&self.owner, &session.owner)
            || !self.stdout.bound_to(&self.reader, Stream::Output)
            || !self.stderr.bound_to(&self.reader, Stream::Error)
            || !self.stdin.bound_to(&self.reader, Stream::Input)
        {
            return Err(self.fail(session, DriverFault::Binding));
        }
        let now = Instant::now();
        // Expiry wins over simultaneous cancellation or data. Never move deadlines.
        let polled = session.poll(now);
        if polled.is_ok() && !self.cancel_seen && self.cancel.load(Ordering::Acquire) {
            self.cancel_seen = true;
            let _ = session.cancel(now);
        }
        if session.phase() == SessionPhase::LoginCancelling {
            if let Some(write) = &self.write {
                if write.receipt.method() != Method::LoginCancel {
                    if write.offset != 0 {
                        return Err(self.fail(session, DriverFault::PartialWriteCancelled));
                    }
                    self.write = None;
                }
            }
            return session
                .poll_cancellation(Instant::now())
                .map_err(|_| self.fail(session, DriverFault::Session));
        }
        session
            .poll(Instant::now())
            .map_err(|_| self.fail(session, DriverFault::Session))
    }

    fn queue_request(
        &mut self,
        session: &mut ProtocolSession,
        request: &IssuedRequest,
        bytes: Zeroizing<Vec<u8>>,
    ) -> Result<(), DriverError> {
        self.gate(session)?;
        let pending = session.pending.iter().any(|p| {
            p.request.id == *request.id() && p.request.method == request.method() && !p.sent
        });
        if !pending
            || (session.phase() == SessionPhase::LoginCancelling
                && request.method() != Method::LoginCancel)
        {
            return Err(self.fail(session, DriverFault::Binding));
        }
        self.queue(
            session,
            Receipt::Request(request.id().clone(), request.method()),
            bytes,
        )
    }
    fn queue_initialized(
        &mut self,
        session: &mut ProtocolSession,
        bytes: Zeroizing<Vec<u8>>,
    ) -> Result<(), DriverError> {
        self.gate(session)?;
        if session.phase() != SessionPhase::AwaitInitialized {
            return Err(self.fail(session, DriverFault::Binding));
        }
        self.queue(session, Receipt::Initialized, bytes)
    }
    fn queue(
        &mut self,
        session: &mut ProtocolSession,
        receipt: Receipt,
        bytes: Zeroizing<Vec<u8>>,
    ) -> Result<(), DriverError> {
        if self.write.is_some() {
            return Err(self.fail(session, DriverFault::OutputBusy));
        }
        // Syntax/framing only. An eventual private codec must bind actual methods
        // and parameter schemas; these bytes cannot be supplied by a renderer.
        if bytes.len() < 3
            || bytes.len() > MAX_FRAME_BYTES + 1
            || bytes.last() != Some(&b'\n')
            || bytes[..bytes.len() - 1].contains(&b'\n')
            || codex_accounts_vault::validate_json_object(&bytes[..bytes.len() - 1]).is_err()
        {
            return Err(self.fail(session, DriverFault::InvalidOutput));
        }
        self.write = Some(PendingWrite {
            bytes,
            offset: 0,
            receipt,
        });
        Ok(())
    }
    fn close_input(&mut self, session: &mut ProtocolSession) -> Result<(), DriverError> {
        self.gate(session)?;
        if self.write.is_some() {
            return Err(self.fail(session, DriverFault::OutputBusy));
        }
        self.stdin.close();
        Ok(())
    }
    fn write_step(
        &mut self,
        session: &mut ProtocolSession,
    ) -> Result<(bool, Option<Method>), DriverError> {
        let Some(write) = &mut self.write else {
            return Ok((false, None));
        };
        let end = (write.offset + CHUNK_BYTES).min(write.bytes.len());
        match self.stdin.try_write(&write.bytes[write.offset..end]) {
            Ok(false) => return Ok((false, None)),
            Err(_) => return Err(self.fail(session, DriverFault::InputClosed)),
            Ok(true) => {}
        }
        write.bytes[write.offset..end].zeroize();
        write.offset = end;
        if end != write.bytes.len() {
            return Ok((true, None));
        }
        let write = self.write.take().expect("pending write remains owned");
        let method = write.receipt.method();
        // Queue reservation and partial writes are NOT a sent receipt.
        let result = match write.receipt {
            Receipt::Request(ref id, _) => session.sent(id, Instant::now()),
            Receipt::Initialized => session.initialized_sent(Instant::now()),
        };
        result.map_err(|_| self.fail(session, DriverFault::Session))?;
        Ok((true, Some(method)))
    }
    fn stderr_step(&mut self, session: &mut ProtocolSession) -> Result<bool, DriverError> {
        if self.stderr_eof {
            return Ok(false);
        }
        match self.stderr.try_read() {
            Ok(ReadStep::Data(chunk)) => {
                if chunk.bytes.len() > STDERR_TOTAL_LIMIT - self.stderr_total {
                    return Err(self.fail(session, DriverFault::StderrLimit));
                }
                self.stderr_total += chunk.bytes.len();
                // No UTF-8, JSON or keyword interpretation. Drop zeroizes raw stderr.
                Ok(true)
            }
            Ok(ReadStep::Pending) => Ok(false),
            Ok(ReadStep::Eof) => {
                self.stderr_eof = true;
                Ok(true)
            }
            Err(_) => Err(self.fail(session, DriverFault::Endpoint)),
        }
    }
    fn stdout_step(&mut self, session: &mut ProtocolSession) -> Result<bool, DriverError> {
        if self.stdout_eof || self.ready.is_some() {
            return Ok(false);
        }
        if self.chunk.is_none() {
            match self.stdout.try_read() {
                Ok(ReadStep::Data(chunk)) => {
                    self.chunk = Some(chunk);
                    self.cursor = 0;
                }
                Ok(ReadStep::Pending) => return Ok(false),
                Ok(ReadStep::Eof) => {
                    self.decoder
                        .finish()
                        .map_err(|e| self.fail(session, DriverFault::InvalidFrame(e)))?;
                    self.stdout_eof = true;
                    return Ok(true);
                }
                Err(_) => return Err(self.fail(session, DriverFault::Endpoint)),
            }
        }
        let chunk = self.chunk.as_mut().expect("bounded chunk remains owned");
        let rest = &chunk.bytes[self.cursor..];
        let count = rest
            .iter()
            .position(|b| *b == b'\n')
            .map_or(rest.len(), |n| n + 1);
        // At most one complete frame enters the decoder. Subsequent frames stay in
        // the bounded chunk/channel, instead of forming a second 4 MiB queue.
        let result = self.decoder.feed(&rest[..count]);
        chunk.bytes[self.cursor..self.cursor + count].zeroize();
        self.cursor += count;
        if self.cursor == chunk.bytes.len() {
            self.chunk = None;
        }
        result.map_err(|e| self.fail(session, DriverFault::InvalidFrame(e)))?;
        self.ready = self
            .decoder
            .next_object()
            .map_err(|e| self.fail(session, DriverFault::InvalidFrame(e)))?;
        Ok(true)
    }

    /// Pumps at most one chunk of each stream per iteration. The only idle wait is
    /// bounded by the sequencer's earliest absolute deadline. No reader/writer
    /// worker can remain detached or blocked after this object is dropped.
    fn next(&mut self, session: &mut ProtocolSession) -> Result<DriverEvent, DriverError> {
        if let Some(error) = self.first_error {
            return Err(error);
        }
        if self.closed && Arc::ptr_eq(&self.owner, &session.owner) {
            return Ok(DriverEvent::Closed);
        }
        if self.lease.upgrade().is_some() {
            return Err(self.fail(session, DriverFault::OutstandingFrame));
        }
        loop {
            let deadline = self.gate(session)?;
            if session.phase() == SessionPhase::LoginCancelling && !self.cancel_notified {
                self.cancel_notified = true;
                return Ok(DriverEvent::CancellationRequired);
            }
            if let Some(frame) = self.ready.take() {
                let lease = Arc::new(());
                self.lease = Arc::downgrade(&lease);
                return Ok(DriverEvent::Frame(ReaderFrame {
                    owner: self.owner.clone(),
                    reader: self.reader.clone(),
                    lease,
                    frame,
                }));
            }
            let stderr_progress = self.stderr_step(session)?;
            self.gate(session)?; // Cancellation is checked again before a write.
            let (write_progress, written) = self.write_step(session)?;
            let stdout_progress = self.stdout_step(session)?;
            if let Some(method) = written {
                self.gate(session)?;
                return Ok(DriverEvent::Written(method));
            }
            if self.ready.is_some() {
                continue; // Recheck actual time/cancel before exposing the frame.
            }
            if self.stdout_eof && self.stderr_eof {
                if self.write.is_some() {
                    return Err(self.fail(session, DriverFault::InputClosed));
                }
                session
                    .finish(Instant::now())
                    .map_err(|_| self.fail(session, DriverFault::Session))?;
                self.stdin.close();
                self.closed = true;
                return Ok(DriverEvent::Closed);
            }
            if stderr_progress || write_progress || stdout_progress {
                continue;
            }
            // Producer commits data/intent before a coalesced wake. A wake between
            // the scans and this wait is retained; stale wakes only cause a rescan.
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.wake.recv_timeout(remaining) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(self.fail(session, DriverFault::Endpoint));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "driver_tests.rs"]
mod tests;
