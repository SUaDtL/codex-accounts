//! Bounded transport primitives, not an executable runtime adapter.
//! Strict JSON validation and sealed session sequencing; no qualified schema or IO.
//! LF delimits raw UTF-8 payloads; CR is preserved, not normalized.
//! Successful EOF is terminal. Classification below grants no invocation authority.
//! No caller may treat a decoded byte frame as a valid JSON-RPC response.
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

mod json_object;
mod session;
pub use json_object::{JsonFrameError, JsonObjectDecoder, JsonObjectFrame};
pub use session::{
    CancellationState, IssuedRequest, LoginCompletion, LoginId, ProtocolSession, RequestId,
    ResponseOutcome, SessionError, SessionPhase,
};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_PENDING_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_QUEUED_FRAMES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientMessageKind {
    Request,
    Notification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Initialize,
    Initialized,
    AccountRead,
    LoginStart,
    LoginCancel,
    RateLimitsRead,
}

impl Method {
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Initialized => "initialized",
            Self::AccountRead => "account/read",
            Self::LoginStart => "account/login/start",
            Self::LoginCancel => "account/login/cancel",
            Self::RateLimitsRead => "account/rateLimits/read",
        }
    }

    /// Outbound message classification, not a schema validator or permission to
    /// perform IO. Qualified schema, handshake, IDs and coordinator policy still
    /// gate every use. This is not a server-message dispatcher.
    pub fn classify_client(name: &str, kind: ClientMessageKind) -> Option<Self> {
        let method = Self::from_allowlist(name)?;
        let expected = if method == Self::Initialized {
            ClientMessageKind::Notification
        } else {
            ClientMessageKind::Request
        };
        (kind == expected).then_some(method)
    }

    /// Policy classification, not permission to invoke a method.
    /// Q3 must additionally authorize managed-login parameters and network effects.
    pub fn from_allowlist(name: &str) -> Option<Self> {
        match name {
            "initialize" => Some(Self::Initialize),
            "initialized" => Some(Self::Initialized),
            "account/read" => Some(Self::AccountRead),
            "account/login/start" => Some(Self::LoginStart),
            "account/login/cancel" => Some(Self::LoginCancel),
            "account/rateLimits/read" => Some(Self::RateLimitsRead),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    FrameTooLarge,
    PendingLimit,
    QueueLimit,
    InvalidUtf8,
    EmptyFrame,
    Truncated,
    Closed,
    Poisoned,
}

/// Raw, untrusted frame. Debug output never contains its body.
pub struct UntrustedFrame(Zeroizing<Vec<u8>>);

impl UntrustedFrame {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for UntrustedFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UntrustedFrame([REDACTED])")
    }
}

#[derive(Default)]
pub struct FrameDecoder {
    partial: Zeroizing<Vec<u8>>,
    ready: VecDeque<UntrustedFrame>,
    queued_bytes: usize,
    poisoned: bool,
    finished: bool,
}

impl fmt::Debug for FrameDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FrameDecoder([REDACTED])")
    }
}

impl FrameDecoder {
    pub fn feed(&mut self, input: &[u8]) -> Result<(), FrameError> {
        if self.poisoned {
            return Err(FrameError::Poisoned);
        }
        if self.finished {
            return self.fail(FrameError::Closed);
        }
        for &byte in input {
            if byte == b'\n' {
                if self.partial.is_empty() {
                    return self.fail(FrameError::EmptyFrame);
                }
                if self.ready.len() == MAX_QUEUED_FRAMES {
                    return self.fail(FrameError::QueueLimit);
                }
                if std::str::from_utf8(&self.partial).is_err() {
                    return self.fail(FrameError::InvalidUtf8);
                }
                self.queued_bytes += self.partial.len();
                self.ready
                    .push_back(UntrustedFrame(std::mem::take(&mut self.partial)));
            } else {
                if self.partial.len() == MAX_FRAME_BYTES {
                    return self.fail(FrameError::FrameTooLarge);
                }
                if self.queued_bytes + self.partial.len() == MAX_PENDING_BYTES {
                    return self.fail(FrameError::PendingLimit);
                }
                self.partial.push(byte);
            }
        }
        Ok(())
    }

    pub fn next_frame(&mut self) -> Option<UntrustedFrame> {
        if self.poisoned {
            return None;
        }
        let frame = self.ready.pop_front()?;
        self.queued_bytes -= frame.0.len();
        Some(frame)
    }

    pub fn finish(&mut self) -> Result<(), FrameError> {
        if self.poisoned {
            return Err(FrameError::Poisoned);
        }
        if !self.partial.is_empty() {
            return self.fail(FrameError::Truncated);
        }
        self.finished = true;
        Ok(())
    }

    fn fail(&mut self, error: FrameError) -> Result<(), FrameError> {
        self.poisoned = true;
        self.partial.zeroize();
        for frame in &mut self.ready {
            frame.0.zeroize();
        }
        self.ready.clear();
        self.queued_bytes = 0;
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_privileged_methods() {
        for name in ["turn/start", "thread/resume", "account/logout", "exec", "x"] {
            assert!(Method::from_allowlist(name).is_none());
        }
    }

    #[test]
    fn allowed_method_roundtrip() {
        assert_eq!(
            Method::from_allowlist(Method::AccountRead.wire_name()),
            Some(Method::AccountRead)
        );
    }

    #[test]
    fn split_frames_are_reassembled() {
        let mut decoder = FrameDecoder::default();
        decoder.feed(b"{\"id\":").unwrap();
        assert!(decoder.next_frame().is_none());
        decoder.feed(b"1}\n{}\n").unwrap();
        assert_eq!(decoder.next_frame().unwrap().as_bytes(), b"{\"id\":1}");
        assert_eq!(decoder.next_frame().unwrap().as_bytes(), b"{}");
        assert!(decoder.finish().is_ok());
    }

    #[test]
    fn accepts_exact_limit_and_rejects_one_more() {
        let mut decoder = FrameDecoder::default();
        decoder.feed(&vec![b'x'; MAX_FRAME_BYTES]).unwrap();
        decoder.feed(b"\n").unwrap();
        assert_eq!(
            decoder.next_frame().unwrap().as_bytes().len(),
            MAX_FRAME_BYTES
        );
        decoder.feed(&vec![b'x'; MAX_FRAME_BYTES]).unwrap();
        assert_eq!(decoder.feed(b"x"), Err(FrameError::FrameTooLarge));
    }

    #[test]
    fn aggregate_limit_is_not_per_frame_only() {
        let mut decoder = FrameDecoder::default();
        for _ in 0..4 {
            decoder.feed(&vec![b'x'; MAX_FRAME_BYTES]).unwrap();
            decoder.feed(b"\n").unwrap();
        }
        assert_eq!(decoder.feed(b"x"), Err(FrameError::PendingLimit));
    }

    #[test]
    fn queue_cap_is_enforced() {
        let mut decoder = FrameDecoder::default();
        for _ in 0..MAX_QUEUED_FRAMES {
            decoder.feed(b"{}\n").unwrap();
        }
        assert_eq!(decoder.feed(b"{}\n"), Err(FrameError::QueueLimit));
        assert!(decoder.next_frame().is_none());
    }

    #[test]
    fn invalid_utf8_poisons_session() {
        let mut decoder = FrameDecoder::default();
        assert_eq!(decoder.feed(&[0xff, b'\n']), Err(FrameError::InvalidUtf8));
        assert_eq!(decoder.feed(b"{}\n"), Err(FrameError::Poisoned));
    }

    #[test]
    fn truncated_and_empty_frames_fail() {
        let mut decoder = FrameDecoder::default();
        decoder.feed(b"{").unwrap();
        assert_eq!(decoder.finish(), Err(FrameError::Truncated));
        let mut other = FrameDecoder::default();
        assert_eq!(other.feed(b"\n"), Err(FrameError::EmptyFrame));
    }

    #[test]
    fn debug_never_emits_frame_canary() {
        let mut decoder = FrameDecoder::default();
        decoder.feed(b"SYNTHETIC_SECRET_CANARY\n").unwrap();
        assert!(!format!("{decoder:?}").contains("SYNTHETIC"));
        assert!(!format!("{:?}", decoder.next_frame().unwrap()).contains("SYNTHETIC"));
    }
}
