//! Conditional Resource bundles and their atomic reference-index updates.
use crate::{Redb, format::*, references};
use redb::{Durability, ReadableTable};
use rom::{Bundle, Error, Receipt, Result, Row, StorageState};
use std::sync::atomic::Ordering;

impl Redb {
    pub(super) fn commit_bundle(&self, b: &Bundle) -> Result<Receipt> {
        // Keep the uncertain-state update serialized with other writers, including
        // the interval after redb releases its internal transaction lock.
        let _writer = self.commit_gate.lock().map_err(|_| Error::Storage)?;
        self.available()?;
        let mut tx = self.db.begin_write().map_err(|_| Error::Storage)?;
        // Another writer may have made an uncertain commit while this one waited.
        self.available()?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        {
            let mut receipts = tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
            if let Some(prior) = receipts
                .get(b.receipt.identity.as_str())
                .map_err(|_| Error::Storage)?
            {
                let prior: Receipt =
                    serde_json::from_str(prior.value()).map_err(|_| Error::Storage)?;
                return if prior.fingerprint == b.receipt.fingerprint {
                    Ok(prior)
                } else {
                    Err(Error::IdentityMismatch)
                };
            }
            let mut state_table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
            let mut state: StorageState = serde_json::from_str(
                state_table
                    .get("state")
                    .map_err(|_| Error::Storage)?
                    .ok_or(Error::Storage)?
                    .value(),
            )
            .map_err(|_| Error::Storage)?;
            let mut rows = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
            let key = (
                b.receipt.row.key.kind.as_str(),
                b.receipt.row.key.id.as_str(),
            );
            let existing: Option<Row> = rows
                .get(key)
                .map_err(|_| Error::Storage)?
                .map(|v| serde_json::from_str(v.value()).map_err(|_| Error::Storage))
                .transpose()?;
            if existing.as_ref().map(|r| r.revision) != b.expected {
                return Err(Error::Conflict);
            }
            let revision = b
                .expected
                .unwrap_or(0)
                .checked_add(u64::from(b.changed))
                .ok_or(Error::TooLarge)?;
            if b.receipt.row.revision != revision
                || (!b.changed
                    && (!b.effects.is_empty() || existing.as_ref() != Some(&b.receipt.row)))
            {
                return Err(Error::NotCommitted);
            }
            // Common profile matches SQLite's representable revision range.
            i64::try_from(revision).map_err(|_| Error::TooLarge)?;
            let (old_references, new_references) =
                references::prepare(&tx, &rows, &b.receipt.row, existing.as_ref())?;
            let retired = state.bundle(b)?;
            let row = serde_json::to_string(&b.receipt.row).map_err(|_| Error::NotCommitted)?;
            let receipt = serde_json::to_string(&b.receipt).map_err(|_| Error::NotCommitted)?;
            let mut ordinal = 0;
            if b.changed {
                self.replace_references(
                    &tx,
                    &b.receipt.row.key,
                    &old_references,
                    &new_references,
                    &mut ordinal,
                )?;
                rows.insert(key, row.as_str())
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
                tx.open_table(EVENTS)
                    .map_err(|_| Error::NotCommitted)?
                    .insert(b.receipt.identity.as_str(), row.as_str())
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            }
            receipts
                .insert(b.receipt.identity.as_str(), receipt.as_str())
                .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            for id in retired {
                tx.open_table(EVENTS)
                    .map_err(|_| Error::Storage)?
                    .remove(id.as_str())
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            }
            state_table
                .insert(
                    "state",
                    serde_json::to_string(&state)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            let mut effects = tx.open_table(EFFECTS).map_err(|_| Error::NotCommitted)?;
            for (i, intent) in b.effects.iter().enumerate() {
                let payload = serde_json::to_string(intent).map_err(|_| Error::NotCommitted)?;
                effects
                    .insert(
                        (
                            b.receipt.identity.as_str(),
                            u64::try_from(i).map_err(|_| Error::TooLarge)?,
                        ),
                        payload.as_str(),
                    )
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            }
        }
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        if tx.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        Ok(b.receipt.clone())
    }
}
