//! Bounded host drainage and independent diagnostic loss accounting.
use super::{DiagnosticEvent, DiagnosticKey, DiagnosticStats, correlation::Correlation};
use crate::{Error, Result};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc::{self, Receiver, Sender, error::TrySendError};

/// Opt-in diagnostic configuration. Host session uniqueness is a host responsibility.
pub struct DiagnosticOptions {
    key: DiagnosticKey,
    session: [u8; 16],
    capacity: usize,
}
impl DiagnosticOptions {
    /// Supply a protected key and explicit nonzero identity for this host stream.
    pub fn new(key: DiagnosticKey, session: [u8; 16]) -> Self {
        Self {
            key,
            session,
            capacity: 512,
        }
    }
    /// Bound pending records to 1..=4096. Invalid values fail before intake opens.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }
}

pub struct Diagnostics;
impl Diagnostics {
    pub fn bounded(options: DiagnosticOptions) -> Result<(DiagnosticSink, DiagnosticReader)> {
        if !(1..=4096).contains(&options.capacity) || options.session == [0; 16] {
            return Err(Error::invalid("diagnostics", "capacity/session"));
        }
        let correlation = Correlation::new(options.key)?;
        let (sender, receiver) = mpsc::channel(options.capacity);
        let counters = Arc::new(Counters::default());
        let inner = Arc::new(Inner {
            sender,
            correlation,
            session: options.session,
            sequence: AtomicU64::new(0),
            counters: counters.clone(),
        });
        Ok((
            DiagnosticSink { inner },
            DiagnosticReader { receiver, counters },
        ))
    }
}

/// Internal producers hold no Runtime or database. There is no callback or exporter here.
#[derive(Clone)]
pub struct DiagnosticSink {
    pub(super) inner: Arc<Inner>,
}
/// Host-owned drain handle. Dropping it closes the diagnostic stream only.
pub struct DiagnosticReader {
    receiver: Receiver<DiagnosticEvent>,
    counters: Arc<Counters>,
}
impl DiagnosticReader {
    pub async fn recv(&mut self) -> Option<DiagnosticEvent> {
        self.receiver.recv().await
    }
    pub fn try_recv(&mut self) -> Option<DiagnosticEvent> {
        self.receiver.try_recv().ok()
    }
    /// Stop future records. Already queued records can still be drained.
    pub fn close(&mut self) {
        self.receiver.close();
    }
    pub fn stats(&self) -> DiagnosticStats {
        self.counters.snapshot()
    }
}

pub(super) struct Inner {
    pub(super) sender: Sender<DiagnosticEvent>,
    pub(super) correlation: Correlation,
    pub(super) session: [u8; 16],
    sequence: AtomicU64,
    pub(super) counters: Arc<Counters>,
}
impl Inner {
    pub(super) fn publish(&self, mut record: DiagnosticEvent) {
        if self.sender.is_closed() {
            increment(&self.counters.closed);
            return;
        }
        let Ok(sequence) = self
            .sequence
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        else {
            increment(&self.counters.sequence);
            return;
        };
        record.sequence = sequence;
        match self.sender.try_send(record) {
            Ok(()) => increment(&self.counters.enqueued),
            Err(TrySendError::Full(_)) => increment(&self.counters.full),
            Err(TrySendError::Closed(_)) => increment(&self.counters.closed),
        }
    }
}

#[derive(Default)]
pub(super) struct Counters {
    enqueued: AtomicU64,
    full: AtomicU64,
    pub(super) closed: AtomicU64,
    pub(super) collector: AtomicU64,
    sequence: AtomicU64,
}
impl Counters {
    fn snapshot(&self) -> DiagnosticStats {
        DiagnosticStats {
            enqueued: self.enqueued.load(Ordering::Relaxed),
            dropped_full: self.full.load(Ordering::Relaxed),
            dropped_closed: self.closed.load(Ordering::Relaxed),
            dropped_collector: self.collector.load(Ordering::Relaxed),
            dropped_sequence: self.sequence.load(Ordering::Relaxed),
        }
    }
}
pub(super) fn increment(counter: &AtomicU64) {
    let _ = counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
        Some(n.saturating_add(1))
    });
}

#[cfg(test)]
#[path = "channel_tests.rs"]
mod tests;
