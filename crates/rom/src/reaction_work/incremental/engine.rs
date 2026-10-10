//! Touched-record overlay, accounting and one-time revision publication.
use super::*;
use crate::reaction_work::retention::completed;
pub(super) struct Engine<'a, R: WorkRead> {
    read: &'a R,
    coherence: ReadFence,
    before: WorkHeader,
    pub(super) header: WorkHeader,
    pub(super) records: BTreeMap<String, RecordChange>,
    record_facts: BTreeMap<String, Option<WorkRecord>>,
    versioned: BTreeSet<String>,
    roots: BTreeMap<String, RootChange>,
}
impl<'a, R: WorkRead> Engine<'a, R> {
    pub(super) fn new(read: &'a R) -> Result<Self> {
        let coherence = read.coherence();
        let header = read.header()?;
        if !coherence.same_context(&read.coherence()) {
            return Err(Error::Conflict);
        }
        header.validate()?;
        Ok(Self {
            read,
            coherence,
            before: header.clone(),
            header,
            records: BTreeMap::new(),
            record_facts: BTreeMap::new(),
            versioned: BTreeSet::new(),
            roots: BTreeMap::new(),
        })
    }
    pub(super) fn compatible_capacity(&self) -> Result<()> {
        match self.check_bounds() {
            Err(Error::Overloaded) => Err(Error::Unsupported("persisted work ledger lacks lifecycle capacity; automatic migration is unavailable".into())),
            other => other,
        }
    }
    pub(super) fn check_bounds(&self) -> Result<()> {
        match &self.header.limits {
            Some(limits) => self.header.accounting.check_bounds(limits),
            None => Ok(()),
        }
    }
    pub(super) fn record(&mut self, id: &str) -> Result<Option<WorkRecord>> {
        if let Some(change) = self.records.get(id) {
            return Ok(Some(change.after.clone()));
        }
        if let Some(prior) = self.record_facts.get(id) {
            return Ok(prior.clone());
        }
        let record = self.read.record(id)?;
        self.record_facts.insert(id.into(), record.clone());
        if let Some(record) = &record {
            if record.pending.id != id {
                return Err(Error::Storage);
            }
            self.records.insert(
                id.into(),
                RecordChange {
                    before: Some(record.clone()),
                    after: record.clone(),
                },
            );
        }
        Ok(record)
    }
    pub(super) fn root(&mut self, id: &str, epoch: u64) -> Result<RootAccount> {
        if let Some(change) = self.roots.get(id) {
            return Ok(change.after);
        }
        let before = self.read.root(id)?;
        if let Some(root) = before {
            root.validate()?;
        }
        let after = before.unwrap_or(RootAccount {
            used: 0,
            epoch,
            members: 0,
            incomplete: 0,
        });
        self.roots.insert(id.into(), RootChange { before, after });
        Ok(after)
    }
    pub(super) fn put(&mut self, record: WorkRecord) -> Result<()> {
        let id = record.pending.id.clone();
        let prior = self.record(&id)?;
        if prior
            .as_ref()
            .is_some_and(|before| before.pending != record.pending)
        {
            return Err(Error::IdentityMismatch);
        }
        let root_id = record.pending.cause.root.clone();
        let mut root = self.root(&root_id, record.pending.cause.retry_epoch)?;
        let old_root = root;
        if let Some(before) = &prior {
            root.used = root
                .used
                .checked_sub(before.attempts)
                .ok_or(Error::Storage)?;
            root.incomplete = root
                .incomplete
                .checked_sub(usize::from(!completed(before)))
                .ok_or(Error::Storage)?;
        } else {
            root.members = root.members.checked_add(1).ok_or(Error::TooLarge)?;
        }
        root.used = root
            .used
            .checked_add(record.attempts)
            .ok_or(Error::TooLarge)?;
        root.incomplete = root
            .incomplete
            .checked_add(usize::from(!completed(&record)))
            .ok_or(Error::TooLarge)?;
        let root_change = self.roots.get(&root_id).ok_or(Error::Storage)?;
        // A new root contributes bytes only once, alongside its first member.
        let old_bytes = (old_root.members > 0 || root_change.before.is_some())
            .then(|| entry_for_root(&root_id, old_root.used))
            .transpose()?;
        self.header
            .accounting
            .replace_root(old_bytes, Some(entry_for_root(&root_id, root.used)?))?;
        self.header.accounting.replace_work(
            prior.as_ref().map(|r| entry_for_work(&id, r)).transpose()?,
            Some(entry_for_work(&id, &record)?),
        )?;
        self.roots.get_mut(&root_id).ok_or(Error::Storage)?.after = root;
        match self.records.get_mut(&id) {
            Some(change) => change.after = record,
            None => {
                self.records.insert(
                    id,
                    RecordChange {
                        before: None,
                        after: record,
                    },
                );
            }
        }
        Ok(())
    }
    pub(super) fn root_epochs(&self) -> Result<()> {
        for change in self.records.values() {
            let cause = &change.after.pending.cause;
            if self
                .roots
                .get(&cause.root)
                .is_some_and(|root| root.after.epoch != cause.retry_epoch)
            {
                return Err(Error::Storage);
            }
        }
        Ok(())
    }
    fn validate_epochs(&self) -> Result<()> {
        self.root_epochs()?;
        for change in self.records.values() {
            let record = &change.after;
            let epoch = record.pending.cause.retry_epoch;
            if epoch > self.header.retry_epochs.current
                || (epoch < self.header.retry_epochs.replay_floor && !completed(record))
            {
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
    pub(super) fn checkpoint(&mut self) -> Result<()> {
        for (id, change) in &mut self.records {
            if let Some(before) = &change.before
                && change.after != *before
                && !self.versioned.contains(id)
            {
                let old = entry_for_work(id, &change.after)?;
                change.after.revision = before.revision.checked_add(1).ok_or(Error::TooLarge)?;
                self.versioned.insert(id.clone());
                self.header
                    .accounting
                    .replace_work(Some(old), Some(entry_for_work(id, &change.after)?))?;
            }
        }
        self.check_bounds()
    }
    pub(super) fn finish(mut self, mut result: WorkResult) -> Result<WorkDelta> {
        self.checkpoint()?;
        self.validate_epochs()?;
        if let WorkResult::Claimed(claim) = &mut result {
            claim.work = self
                .records
                .get(&claim.work.pending.id)
                .ok_or(Error::Storage)?
                .after
                .clone();
        }
        if !self.coherence.same_context(&self.read.coherence()) {
            return Err(Error::Conflict);
        }
        let record_preconditions = self.record_facts;
        let root_preconditions = self
            .roots
            .iter()
            .map(|(id, change)| (id.clone(), change.before))
            .collect();
        self.records
            .retain(|_, change| change.before.as_ref() != Some(&change.after));
        self.roots
            .retain(|_, change| change.before.as_ref() != Some(&change.after));
        Ok(WorkDelta {
            coherence: self.coherence,
            record_preconditions,
            root_preconditions,
            before: self.before,
            after: self.header,
            records: self.records,
            roots: self.roots,
            result,
        })
    }
    pub(super) fn candidate(
        &mut self,
        now: u64,
        after: Option<&str>,
    ) -> Result<Option<WorkRecord>> {
        let record = self.read.next_candidate(now, after)?;
        if let Some(record) = &record {
            if after.is_some_and(|id| record.pending.id.as_str() <= id) {
                return Err(Error::Storage);
            }
            if let Some(prior) = self.record_facts.get(&record.pending.id) {
                if prior.as_ref() != Some(record) {
                    return Err(Error::Storage);
                }
            } else {
                self.record_facts
                    .insert(record.pending.id.clone(), Some(record.clone()));
            }
            self.records
                .entry(record.pending.id.clone())
                .or_insert_with(|| RecordChange {
                    before: Some(record.clone()),
                    after: record.clone(),
                });
        }
        Ok(record)
    }
}
