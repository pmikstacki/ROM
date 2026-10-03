//! Retry epoch integrity and whole-root maintenance selection.
use super::*;

fn completed(record: &WorkRecord) -> bool {
    record.state == WorkState::Done
        && match record.pending.payload {
            WorkPayload::Notification { .. } => record.delivery == Some(DeliveryOutcome::Accepted),
            _ => record.delivery.is_none(),
        }
}

impl WorkLedger {
    /// Every member of a causal root belongs to one immutable retry epoch.
    pub(super) fn root_epochs(&self) -> Result<BTreeMap<&str, u64>> {
        let mut roots = BTreeMap::new();
        for record in self.work.values() {
            let cause = &record.pending.cause;
            if roots
                .insert(cause.root.as_str(), cause.retry_epoch)
                .is_some_and(|prior| prior != cause.retry_epoch)
            {
                return Err(Error::Storage);
            }
        }
        Ok(roots)
    }

    pub(crate) fn validate_retry_epochs(&self, epochs: RetryEpochs) -> Result<()> {
        epochs.validate()?;
        self.root_epochs()?;
        for record in self.work.values() {
            let epoch = record.pending.cause.retry_epoch;
            if epoch > epochs.current || (epoch < epochs.replay_floor && !completed(record)) {
                return Err(Error::Storage);
            }
            if let WorkPayload::Action(value) = &record.pending.payload {
                let invocation: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                if invocation.retry_epoch != epoch {
                    return Err(Error::Storage);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn claim_retry_epoch(&self, claim: &ClaimKey, now: u64) -> Result<u64> {
        let record = self.validate_claim(claim, now)?;
        if matches!(
            record.state,
            WorkState::Leased {
                resolution_only: Some(_),
                ..
            }
        ) {
            return Err(Error::Conflict);
        }
        Ok(record.pending.cause.retry_epoch)
    }

    /// Called only on a private candidate state, after archive integrity validation.
    pub(crate) fn retire_completed_roots(&mut self, replay_floor: u64) -> Result<(usize, usize)> {
        let retired: BTreeSet<String> = self
            .root_epochs()?
            .into_iter()
            .filter(|(_, epoch)| *epoch < replay_floor)
            .map(|(root, _)| root.to_owned())
            .collect();
        if self
            .work
            .values()
            .any(|record| retired.contains(&record.pending.cause.root) && !completed(record))
        {
            return Err(Error::Conflict);
        }
        let prior_records = self.work.len();
        self.work
            .retain(|_, record| !retired.contains(&record.pending.cause.root));
        self.roots.retain(|root, _| !retired.contains(root));
        self.validate_archive()?;
        Ok((prior_records - self.work.len(), retired.len()))
    }
}
