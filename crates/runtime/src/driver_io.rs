//! Private, bounded in-memory endpoints. Not operating-system pipes or process handles.
#![forbid(unsafe_code)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, TryRecvError, TrySendError},
    Arc,
};
use zeroize::Zeroizing;

pub(super) const CHUNK_BYTES: usize = 4096;
pub(super) const QUEUE_CHUNKS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stream {
    Input,
    Output,
    Error,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IoError {
    Closed,
    ChunkLimit,
    Binding,
}
pub(super) struct Chunk {
    owner: Arc<()>,
    stream: Stream,
    pub(super) bytes: Zeroizing<Vec<u8>>,
}
impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Chunk([REDACTED])")
    }
}
pub(super) enum ReadStep {
    Data(Chunk),
    Pending,
    Eof,
}
pub(super) struct ReadEnd {
    owner: Arc<()>,
    stream: Stream,
    receiver: Option<Receiver<Chunk>>,
    wake: SyncSender<()>,
}
pub(super) struct WriteEnd {
    owner: Arc<()>,
    stream: Stream,
    sender: Option<SyncSender<Chunk>>,
    wake: SyncSender<()>,
}
impl ReadEnd {
    pub(super) fn bound_to(&self, owner: &Arc<()>, stream: Stream) -> bool {
        Arc::ptr_eq(&self.owner, owner) && self.stream == stream
    }
    pub(super) fn try_read(&self) -> Result<ReadStep, IoError> {
        let Some(receiver) = &self.receiver else {
            return Err(IoError::Closed);
        };
        match receiver.try_recv() {
            Ok(chunk) => {
                // Releasing a queue slot can unblock the other endpoint's writer.
                let _ = self.wake.try_send(());
                if !Arc::ptr_eq(&self.owner, &chunk.owner) || self.stream != chunk.stream {
                    return Err(IoError::Binding);
                }
                Ok(ReadStep::Data(chunk))
            }
            Err(TryRecvError::Empty) => Ok(ReadStep::Pending),
            Err(TryRecvError::Disconnected) => Ok(ReadStep::Eof),
        }
    }
    pub(super) fn close(&mut self) {
        // Dropping the receiver also drops/zeroizes every queued chunk.
        self.receiver.take();
        let _ = self.wake.try_send(());
    }
}
impl Drop for ReadEnd {
    fn drop(&mut self) {
        self.close();
    }
}
impl WriteEnd {
    pub(super) fn bound_to(&self, owner: &Arc<()>, stream: Stream) -> bool {
        Arc::ptr_eq(&self.owner, owner) && self.stream == stream
    }
    /// False means backpressure, not successful delivery or dropped data.
    /// Never blocks, including when the consumer has stopped reading.
    pub(super) fn try_write(&self, bytes: &[u8]) -> Result<bool, IoError> {
        if bytes.is_empty() || bytes.len() > CHUNK_BYTES {
            return Err(IoError::ChunkLimit);
        }
        let Some(sender) = &self.sender else {
            return Err(IoError::Closed);
        };
        let chunk = Chunk {
            owner: self.owner.clone(),
            stream: self.stream,
            bytes: Zeroizing::new(bytes.to_vec()),
        };
        match sender.try_send(chunk) {
            Ok(()) => {
                let _ = self.wake.try_send(());
                Ok(true)
            }
            Err(TrySendError::Full(_)) => Ok(false),
            Err(TrySendError::Disconnected(_)) => Err(IoError::Closed),
        }
        // Rejected chunks are zeroized on drop, not included in errors.
    }
    pub(super) fn close(&mut self) {
        // There are no cloned data senders. Disconnection follows queued data.
        self.sender.take();
        let _ = self.wake.try_send(());
    }
}
impl Drop for WriteEnd {
    fn drop(&mut self) {
        self.close();
    }
}

pub(super) struct CancelHandle {
    pub(super) requested: Arc<AtomicBool>,
    wake: SyncSender<()>,
}
impl CancelHandle {
    pub(super) fn request(&self) {
        self.requested.store(true, Ordering::Release);
        // One coalesced wake is enough: intent is an independent sticky bit.
        // A full stdout/stderr queue therefore cannot hide cancellation.
        let _ = self.wake.try_send(());
    }
}
impl Drop for CancelHandle {
    fn drop(&mut self) {
        // Dropping a control handle does not itself cancel the operation.
        let _ = self.wake.try_send(());
    }
}

#[cfg(test)]
pub(super) fn pair(owner: &Arc<()>, stream: Stream, wake: &SyncSender<()>) -> (ReadEnd, WriteEnd) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(QUEUE_CHUNKS);
    (
        ReadEnd {
            owner: owner.clone(),
            stream,
            receiver: Some(receiver),
            wake: wake.clone(),
        },
        WriteEnd {
            owner: owner.clone(),
            stream,
            sender: Some(sender),
            wake: wake.clone(),
        },
    )
}
#[cfg(test)]
pub(super) fn cancellation(wake: &SyncSender<()>) -> CancelHandle {
    CancelHandle {
        requested: Arc::new(AtomicBool::new(false)),
        wake: wake.clone(),
    }
}
