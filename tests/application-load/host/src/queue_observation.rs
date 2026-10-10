//! In-process confirmation timing only. No clock is reconstructed across restart.
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
pub struct QueueObservation {
    capacity: usize,
    seen: BTreeSet<String>,
    pending: BTreeMap<String, Option<Instant>>,
    samples: Vec<u64>,
    raced_or_unknown: usize,
    rejected: usize,
}
impl QueueObservation {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            seen: BTreeSet::new(),
            pending: BTreeMap::new(),
            samples: vec![],
            raced_or_unknown: 0,
            rejected: 0,
        }
    }
    /// Bookkeeping lock must be released before the adapter commit begins.
    pub fn offer(&mut self, identity: &str) -> rom::Result<()> {
        if self.seen.contains(identity) {
            return Ok(());
        }
        if self.seen.len() == self.capacity || identity.len() > 4096 {
            return Err(rom::Error::TooLarge);
        }
        self.seen.insert(identity.to_owned());
        self.pending.insert(identity.to_owned(), None);
        Ok(())
    }
    /// A claim which already removed the entry must not become a synthetic sample.
    pub fn committed(&mut self, identity: &str, confirmed: Instant) {
        if let Some(slot) = self.pending.get_mut(identity)
            && slot.is_none()
        {
            *slot = Some(confirmed);
        }
    }
    pub fn failed(&mut self, identity: &str) {
        self.pending.remove(identity);
    }
    /// Bookkeeping runs after actual adapter claim confirmation, outside its native lock.
    pub fn claimed(&mut self, identity: &str, confirmed: Instant) {
        let Some(Some(enqueued)) = self.pending.remove(identity) else {
            self.raced_or_unknown += 1;
            return;
        };
        let Some(duration) = confirmed.checked_duration_since(enqueued) else {
            self.raced_or_unknown += 1;
            return;
        };
        if let Ok(nanos) = u64::try_from(duration.as_nanos()) {
            self.samples.push(nanos);
        } else {
            self.raced_or_unknown += 1;
        }
    }
    pub fn mark_rejected(&mut self) {
        self.rejected += 1;
    }
    pub fn rejected(&self) -> usize {
        self.rejected
    }
    pub fn samples(&self) -> &[u64] {
        &self.samples
    }
    pub fn missing(&self) -> usize {
        self.raced_or_unknown
    }
}
#[cfg(test)]
#[path = "queue_observation_tests.rs"]
mod tests;
