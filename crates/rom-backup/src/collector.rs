use crate::codec::decode;
use crate::{BackupLimits, Snapshot, StoredEffect};
use rom::{Descriptor, Error, Receipt, ReferenceEdge, Result, Row};

/// Bounds raw native bytes before decoding and appending to a logical snapshot.
pub struct Collector {
    pub snapshot: Snapshot,
    limits: BackupLimits,
    bytes: usize,
    records: usize,
}
impl Collector {
    pub fn new(state: &str, limits: BackupLimits) -> Result<Self> {
        if state.len() > limits.max_bytes || limits.max_records == 0 {
            return Err(Error::TooLarge);
        }
        let parsed: rom::StorageState = decode(state)?;
        let records = parsed.work.records().len();
        if records > limits.max_records {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            snapshot: Snapshot {
                state: parsed,
                rows: vec![],
                receipts: vec![],
                events: vec![],
                effects: vec![],
                descriptors: vec![],
                references: vec![],
            },
            limits,
            bytes: state.len(),
            records,
        })
    }
    fn charge(&mut self, data: &str, key_bytes: usize) -> Result<()> {
        self.physical(data.len().checked_add(key_bytes).ok_or(Error::TooLarge)?)
    }
    /// Charge one adapter-owned physical record to the same complete collection budget.
    /// Include encoded keys and values in `bytes`; no logical snapshot data is added.
    pub fn physical(&mut self, bytes: usize) -> Result<()> {
        let records = self.records.checked_add(1).ok_or(Error::TooLarge)?;
        let total = self.bytes.checked_add(bytes).ok_or(Error::TooLarge)?;
        if records > self.limits.max_records || total > self.limits.max_bytes {
            return Err(Error::TooLarge);
        }
        self.records = records;
        self.bytes = total;
        Ok(())
    }
    pub fn row(&mut self, kind: &str, id: &str, revision: Option<u64>, data: &str) -> Result<()> {
        self.charge(
            data,
            kind.len().checked_add(id.len()).ok_or(Error::TooLarge)?,
        )?;
        let row: Row = decode(data)?;
        if row.key.kind != kind || row.key.id != id || revision.is_some_and(|r| r != row.revision) {
            return Err(Error::Storage);
        }
        self.snapshot.rows.push(row);
        Ok(())
    }
    pub fn descriptor(&mut self, kind: &str, data: &str) -> Result<()> {
        self.charge(data, kind.len())?;
        let descriptor: Descriptor = decode(data)?;
        if descriptor.kind != kind {
            return Err(Error::Storage);
        }
        self.snapshot.descriptors.push(descriptor);
        Ok(())
    }
    pub fn reference(&mut self, edge: ReferenceEdge) -> Result<()> {
        let bytes = [
            &edge.source.kind,
            &edge.source.id,
            &edge.target.kind,
            &edge.target.id,
        ]
        .into_iter()
        .try_fold(0usize, |n, value| {
            n.checked_add(value.len()).ok_or(Error::TooLarge)
        })?;
        self.charge("", bytes)?;
        self.snapshot.references.push(edge);
        Ok(())
    }
    pub fn receipt(&mut self, id: &str, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        let receipt: Receipt = decode(data)?;
        if receipt.identity != id {
            return Err(Error::Storage);
        }
        self.snapshot.receipts.push(receipt);
        Ok(())
    }
    pub fn event(&mut self, id: &str, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        self.snapshot.events.push((id.into(), decode(data)?));
        Ok(())
    }
    pub fn effect(&mut self, id: &str, ordinal: u64, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        self.snapshot.effects.push(StoredEffect {
            identity: id.into(),
            ordinal,
            intent: decode(data)?,
        });
        Ok(())
    }
}
