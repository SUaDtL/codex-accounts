//! Syntax-only transport adapter. No RPC schema, ID, method or identity authority.
#![forbid(unsafe_code)]
use super::{FrameDecoder, FrameError, UntrustedFrame};
use codex_accounts_vault::{validate_json_object, DataError};
use std::fmt;

/// Fixed categories only; parser messages and rejected payloads are never exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonFrameError {
    Transport(FrameError),
    InvalidObject,
    DuplicateKey,
    DepthLimit,
    InputLimit,
}

/// Exact original bytes with object syntax checked, still untrusted protocol data.
/// This does NOT assert a valid response, allowed method, identity or qualification.
/// No public constructor can bypass validation.
///
/// ```compile_fail
/// use codex_accounts_runtime::JsonObjectFrame;
/// let forged = JsonObjectFrame(Vec::new());
/// ```
pub struct JsonObjectFrame(UntrustedFrame);
impl JsonObjectFrame {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}
impl fmt::Debug for JsonObjectFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JsonObjectFrame([REDACTED])")
    }
}

/// Uses the same 1 MiB frame / 4 MiB pending / 32-frame queue as raw transport.
/// `feed` frames bytes; `next_object` validates before returning each object.
/// On ANY syntax or framing error, all undelivered frames are discarded and this
/// decoder is permanently poisoned. Already returned objects cannot be revoked;
/// consumers must independently validate schema and correlate IDs before use.
#[derive(Default)]
pub struct JsonObjectDecoder {
    decoder: FrameDecoder,
}
impl fmt::Debug for JsonObjectDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JsonObjectDecoder([REDACTED])")
    }
}
impl JsonObjectDecoder {
    pub fn feed(&mut self, input: &[u8]) -> Result<(), JsonFrameError> {
        self.decoder.feed(input).map_err(JsonFrameError::Transport)
    }

    /// Returns None only when no complete frame is queued, not after an error.
    /// Reuses the existing strict parser: duplicate decoded keys at every level,
    /// trailing input, non-object roots, malformed UTF-8/JSON and depth >64 refuse.
    pub fn next_object(&mut self) -> Result<Option<JsonObjectFrame>, JsonFrameError> {
        if self.decoder.poisoned {
            return Err(JsonFrameError::Transport(FrameError::Poisoned));
        }
        let Some(frame) = self.decoder.next_frame() else {
            return Ok(None);
        };
        match validate_json_object(frame.as_bytes()) {
            Ok(()) => Ok(Some(JsonObjectFrame(frame))),
            Err(error) => {
                // The failing frame drops its zeroizing owner. Invalidate the
                // remaining queue, including otherwise-valid trailing objects.
                let _ = self.decoder.fail(FrameError::Poisoned);
                Err(match error {
                    DataError::DuplicateKey => JsonFrameError::DuplicateKey,
                    DataError::DepthLimit => JsonFrameError::DepthLimit,
                    DataError::InputLimit => JsonFrameError::InputLimit,
                    _ => JsonFrameError::InvalidObject,
                })
            }
        }
    }

    /// End transport input. Complete queued frames still require next_object().
    /// EOF does not turn unparsed frames into valid objects or successful RPCs.
    pub fn finish(&mut self) -> Result<(), JsonFrameError> {
        self.decoder.finish().map_err(JsonFrameError::Transport)
    }
}
