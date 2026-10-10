//! Fixed collectors publish explicitly after core and persistence gates are released.
use super::{
    DiagnosticEvent, DiagnosticOutcome, DiagnosticSink, DiagnosticStage, channel::increment,
    correlation::Domain,
};
use crate::{Cause, ClaimKey, WorkClaim};
use std::time::Instant;

const RECORD_LIMIT: usize = 32;

pub(crate) struct OperationRecord {
    sink: DiagnosticSink,
    base: Option<DiagnosticEvent>,
    records: [Option<DiagnosticEvent>; RECORD_LIMIT],
    length: usize,
    started: Option<Instant>,
}
impl DiagnosticSink {
    pub(crate) fn operation(
        &self,
        identity: &str,
        cause: Option<&Cause>,
        claim: Option<&ClaimKey>,
        attempt: u32,
    ) -> OperationRecord {
        self.record(
            identity,
            cause,
            claim.map(|claim| (claim.id.as_str(), claim.generation)),
            attempt,
            Domain::Operation,
        )
    }
    pub(crate) fn work(&self, claim: &WorkClaim) -> OperationRecord {
        self.record(
            &claim.work.pending.id,
            Some(&claim.work.pending.cause),
            Some((&claim.work.pending.id, claim.work.generation)),
            claim.work.attempts,
            Domain::Operation,
        )
    }
    pub(crate) fn shutdown_record(&self) -> OperationRecord {
        self.record("", None, None, 0, Domain::Lifecycle)
    }
    fn record(
        &self,
        identity: &str,
        cause: Option<&Cause>,
        claim: Option<(&str, u64)>,
        attempt: u32,
        domain: Domain,
    ) -> OperationRecord {
        let base = if self.inner.sender.is_closed() {
            None
        } else {
            let correlation = &self.inner.correlation;
            let operation = if matches!(domain, Domain::Lifecycle) {
                correlation.token(domain, &[&self.inner.session])
            } else {
                correlation.token(domain, &[identity.as_bytes()])
            };
            let root = if matches!(domain, Domain::Lifecycle) {
                operation
            } else {
                correlation.token(
                    Domain::Root,
                    &[cause
                        .map_or(identity, |cause| cause.root.as_str())
                        .as_bytes()],
                )
            };
            let parent = cause
                .and_then(|cause| cause.parent.as_ref())
                .map(|parent| correlation.token(Domain::Work, &[parent.as_bytes()]));
            let work = claim.map(|(id, _)| correlation.token(Domain::Work, &[id.as_bytes()]));
            let claim_token = claim.map(|(id, generation)| {
                correlation.token(Domain::Claim, &[id.as_bytes(), &generation.to_be_bytes()])
            });
            Some(DiagnosticEvent {
                version: 1,
                session: self.inner.session,
                sequence: 0,
                operation,
                root,
                parent,
                work,
                claim: claim_token,
                stage: DiagnosticStage::Admission,
                outcome: DiagnosticOutcome::Started,
                attempt,
                depth: cause.map_or(0, |cause| cause.depth),
                elapsed_ns: None,
                stage_elapsed_ns: None,
                event_count: 0,
            })
        };
        OperationRecord {
            sink: self.clone(),
            started: base.map(|_| Instant::now()),
            base,
            records: [None; RECORD_LIMIT],
            length: 0,
        }
    }
}
impl OperationRecord {
    pub(crate) fn last_stage(&self) -> Option<DiagnosticStage> {
        self.length
            .checked_sub(1)
            .and_then(|index| self.records[index])
            .map(|record| record.stage)
    }
    pub(crate) fn last_outcome(&self) -> Option<DiagnosticOutcome> {
        self.length
            .checked_sub(1)
            .and_then(|index| self.records[index])
            .map(|record| record.outcome)
    }
    pub(crate) fn stage_since(
        &mut self,
        stage: DiagnosticStage,
        outcome: DiagnosticOutcome,
        started: Option<Instant>,
        event_count: u32,
    ) {
        let previous_length = self.length;
        self.stage(stage, outcome, self.elapsed_ns(), event_count);
        if self.length > previous_length
            && let Some(record) = &mut self.records[previous_length]
        {
            record.stage_elapsed_ns = started.and_then(elapsed_since);
        }
    }
    pub(crate) fn stage(
        &mut self,
        stage: DiagnosticStage,
        outcome: DiagnosticOutcome,
        elapsed_ns: Option<u64>,
        event_count: u32,
    ) {
        let Some(mut record) = self.base else {
            increment(&self.sink.inner.counters.closed);
            return;
        };
        if self.length == RECORD_LIMIT {
            increment(&self.sink.inner.counters.collector);
            return;
        }
        record.stage = stage;
        record.outcome = outcome;
        record.elapsed_ns = elapsed_ns;
        record.event_count = event_count;
        self.records[self.length] = Some(record);
        self.length += 1;
    }
    /// A timer exists only while diagnostics is active. Disabled callers never construct a recorder.
    pub(crate) fn timer(&self) -> Option<Instant> {
        self.base
            .filter(|_| !self.sink.inner.sender.is_closed())
            .map(|_| Instant::now())
    }
    pub(crate) fn elapsed_ns(&self) -> Option<u64> {
        self.started
            .filter(|_| !self.sink.inner.sender.is_closed())
            .and_then(elapsed_since)
    }
    /// Explicit publication only. Dropping a collector never invokes code or emits a record.
    pub(crate) fn publish(self) {
        for record in self.records.into_iter().take(self.length).flatten() {
            self.sink.inner.publish(record);
        }
    }
}
/// A missing or backwards local clock observation yields unavailable timing, not a fabricated zero.
pub(crate) fn elapsed_since(started: Instant) -> Option<u64> {
    checked_elapsed(started, Instant::now())
}
fn checked_elapsed(started: Instant, ended: Instant) -> Option<u64> {
    ended
        .checked_duration_since(started)
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
}

#[cfg(test)]
#[path = "timing_tests.rs"]
mod tests;
