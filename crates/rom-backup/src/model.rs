use rom::{Descriptor, Error, Intent, Receipt, ReferenceEdge, Result, Row, StorageState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const ARCHIVE_VERSION: u32 = 4;
/// Current native format. Older readers must reject persisted retry epochs.
pub const STORAGE_FORMAT: u32 = 6;

#[derive(Clone, Copy, Debug)]
pub struct BackupLimits {
    pub max_bytes: usize,
    pub max_records: usize,
}
impl Default for BackupLimits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_records: 400_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    Sqlite,
    Redb,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub archive_version: u32,
    pub storage_format: u32,
    pub backend: Backend,
    pub rows: usize,
    pub receipts: usize,
    pub events: usize,
    pub effects: usize,
    pub descriptors: usize,
    pub references: usize,
    pub work: usize,
    pub external_blobs_included: bool,
    pub external_deliveries_included: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEffect {
    pub identity: String,
    pub ordinal: u64,
    pub intent: Intent,
}
/// Full private data for adapter implementations. Deliberately has no Debug implementation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub state: StorageState,
    pub rows: Vec<Row>,
    pub receipts: Vec<Receipt>,
    pub events: Vec<(String, Row)>,
    pub effects: Vec<StoredEffect>,
    pub descriptors: Vec<Descriptor>,
    pub references: Vec<ReferenceEdge>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let mut keys = BTreeMap::new();
        for row in &self.rows {
            if row.key.kind.is_empty()
                || row.key.id.is_empty()
                || row.revision == 0
                || row.revision > i64::MAX as u64
                || keys.insert((&row.key.kind, &row.key.id), row).is_some()
            {
                return Err(Error::Storage);
            }
        }
        for descriptor in &self.descriptors {
            if descriptor.canonical().map_err(|_| Error::Storage)? != *descriptor {
                return Err(Error::Storage);
            }
        }
        let catalog = rom::validate_descriptors(&self.descriptors).map_err(|_| Error::Storage)?;
        let catalog: BTreeMap<_, _> = catalog.iter().map(|d| (d.kind.as_str(), d)).collect();
        let mut expected_edges = BTreeSet::new();
        for row in &self.rows {
            let descriptor = catalog.get(row.key.kind.as_str()).ok_or(Error::Storage)?;
            for target in descriptor
                .reference_targets(row.value.as_ref())
                .map_err(|_| Error::Storage)?
            {
                if keys
                    .get(&(&target.kind, &target.id))
                    .is_none_or(|r| r.value.is_none())
                {
                    return Err(Error::Storage);
                }
                expected_edges.insert(ReferenceEdge {
                    source: row.key.clone(),
                    target,
                });
            }
        }
        let actual_edges: BTreeSet<_> = self.references.iter().cloned().collect();
        if actual_edges.len() != self.references.len() || actual_edges != expected_edges {
            return Err(Error::Storage);
        }
        let mut receipts = BTreeMap::new();
        let mut current_receipts = BTreeSet::new();
        for receipt in &self.receipts {
            let row = &receipt.row;
            let current = keys
                .get(&(&row.key.kind, &row.key.id))
                .ok_or(Error::Storage)?;
            if receipt.identity.is_empty()
                || receipt.retry_epoch > self.state.retry_epochs().current
                || receipt.replay_version.is_some_and(|v| {
                    v == 0
                        || catalog
                            .get(row.key.kind.as_str())
                            .is_none_or(|d| v > d.version)
                })
                || row.revision == 0
                || row.revision > current.revision
                || (row.revision == current.revision && row != *current)
                || receipts.insert(&receipt.identity, receipt).is_some()
            {
                return Err(Error::Storage);
            }
            if row.revision == current.revision {
                current_receipts.insert((&row.key.kind, &row.key.id));
            }
        }
        if current_receipts.len() != keys.len() {
            return Err(Error::Storage);
        }
        let mut events = BTreeSet::new();
        for (id, row) in &self.events {
            if !events.insert(id) || &receipts.get(id).ok_or(Error::Storage)?.row != row {
                return Err(Error::Storage);
            }
        }
        let mut effects = BTreeMap::<&str, BTreeSet<u64>>::new();
        for effect in &self.effects {
            if !receipts.contains_key(&effect.identity)
                || !effects
                    .entry(&effect.identity)
                    .or_default()
                    .insert(effect.ordinal)
            {
                return Err(Error::Storage);
            }
        }
        for ordinals in effects.values() {
            if ordinals.iter().copied().ne(0..ordinals.len() as u64) {
                return Err(Error::Storage);
            }
        }
        self.state
            .validate_archive(self.receipts.len(), self.effects.len(), &self.events)
    }
    pub(crate) fn manifest(&self, backend: Backend) -> Manifest {
        Manifest {
            archive_version: ARCHIVE_VERSION,
            storage_format: STORAGE_FORMAT,
            backend,
            rows: self.rows.len(),
            receipts: self.receipts.len(),
            events: self.events.len(),
            effects: self.effects.len(),
            descriptors: self.descriptors.len(),
            references: self.references.len(),
            work: self.state.work.records().len(),
            external_blobs_included: false,
            external_deliveries_included: false,
        }
    }
}
