use crate::common::*;
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use rom_persistence_core::*;
use std::{path::Path, sync::Arc};

const RESOURCES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("resources");
const RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("receipts");
const EVENTS: TableDefinition<u64, &[u8]> = TableDefinition::new("events");
const META: TableDefinition<&str, &[u8]> = TableDefinition::new("metadata");
fn unavailable<E>(_: E) -> Error {
    Error::Unavailable
}

#[derive(Clone)]
pub struct RedbStore {
    database: Arc<Database>,
    #[cfg(feature = "fault-injection")]
    probe: crate::probe::Probe,
}
impl RedbStore {
    #[cfg(feature = "fault-injection")]
    pub fn with_probe(mut self, probe: crate::probe::Probe) -> Self {
        self.probe = probe;
        self
    }
    pub fn open(path: &Path) -> Result<Self, Error> {
        let database = Database::create(path).map_err(unavailable)?;
        let mut tx = database.begin_write().map_err(unavailable)?;
        tx.set_durability(Durability::Immediate)
            .map_err(unavailable)?;
        tx.open_table(RESOURCES).map_err(unavailable)?;
        tx.open_table(RECEIPTS).map_err(unavailable)?;
        tx.open_table(EVENTS).map_err(unavailable)?;
        {
            let mut meta = tx.open_table(META).map_err(unavailable)?;
            let absent = meta.get("meta").map_err(unavailable)?.is_none();
            if absent {
                meta.insert("meta", encode(&Meta::new()?)?.as_slice())
                    .map_err(unavailable)?;
            }
        }
        tx.commit().map_err(unavailable)?;
        Ok(Self {
            database: Arc::new(database),
            #[cfg(feature = "fault-injection")]
            probe: crate::probe::Probe::default(),
        })
    }
}
impl Storage for RedbStore {
    fn capabilities(&self) -> Capabilities {
        CAPABILITIES
    }
    fn load(&self, key: &ResourceKey) -> Result<Option<Resource>, Error> {
        let tx = self.database.begin_read().map_err(unavailable)?;
        let table = tx.open_table(RESOURCES).map_err(unavailable)?;
        table
            .get(encode(key)?.as_slice())
            .map_err(unavailable)?
            .map(|v| decode(v.value()))
            .transpose()
    }
    fn receipt(&self, action: &str) -> Result<ReceiptStatus, Error> {
        let tx = self.database.begin_read().map_err(unavailable)?;
        let table = tx.open_table(RECEIPTS).map_err(unavailable)?;
        Ok(match table.get(action).map_err(unavailable)? {
            Some(v) => ReceiptStatus::Found(decode::<StoredReceipt>(v.value())?.receipt),
            None => ReceiptStatus::AbsentNow,
        })
    }
    fn commit(&self, transition: &Transition) -> Result<Receipt, Error> {
        transition.validate(self.capabilities())?;
        let mut tx = self.database.begin_write().map_err(unavailable)?;
        tx.set_durability(Durability::Immediate)
            .map_err(unavailable)?;
        let receipt;
        {
            let mut receipts = tx.open_table(RECEIPTS).map_err(unavailable)?;
            if let Some(bytes) = receipts
                .get(transition.action.as_str())
                .map_err(unavailable)?
            {
                return decode::<StoredReceipt>(bytes.value())?.resolve(transition);
            }
            let mut resources = tx.open_table(RESOURCES).map_err(unavailable)?;
            let key = encode(&transition.key)?;
            let current = resources
                .get(key.as_slice())
                .map_err(unavailable)?
                .map(|v| decode::<Resource>(v.value()))
                .transpose()?;
            let (resource, result, events) = transition.materialize(current.map(|r| r.revision))?;
            receipt = result;
            let mut metadata = tx.open_table(META).map_err(unavailable)?;
            let mut meta: Meta = decode(
                metadata
                    .get("meta")
                    .map_err(unavailable)?
                    .ok_or(Error::Unavailable)?
                    .value(),
            )?;
            resources
                .insert(key.as_slice(), encode(&resource)?.as_slice())
                .map_err(unavailable)?;
            #[cfg(feature = "fault-injection")]
            self.probe.hit(crate::probe::Checkpoint::AfterResource)?;
            receipts
                .insert(
                    transition.action.as_str(),
                    encode(&StoredReceipt {
                        transition: transition.clone(),
                        receipt: receipt.clone(),
                    })?
                    .as_slice(),
                )
                .map_err(unavailable)?;
            #[cfg(feature = "fault-injection")]
            self.probe.hit(crate::probe::Checkpoint::AfterReceipt)?;
            let mut journal = tx.open_table(EVENTS).map_err(unavailable)?;
            for event in events {
                journal
                    .insert(meta.allocate()?, encode(&event)?.as_slice())
                    .map_err(unavailable)?;
                #[cfg(feature = "fault-injection")]
                self.probe.hit(crate::probe::Checkpoint::AfterEvent)?;
            }
            metadata
                .insert("meta", encode(&meta)?.as_slice())
                .map_err(unavailable)?;
        }
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::BeforeCommit)?;
        tx.commit().map_err(|_| Error::Unknown {
            action: transition.action.clone(),
        })?;
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::AfterCommit)?;
        Ok(receipt)
    }
    fn journal(&self, after: Option<&Cursor>, limit: usize) -> Result<Page, Error> {
        let tx = self.database.begin_read().map_err(unavailable)?;
        let metadata = tx.open_table(META).map_err(unavailable)?;
        let meta: Meta = decode(
            metadata
                .get("meta")
                .map_err(unavailable)?
                .ok_or(Error::Unavailable)?
                .value(),
        )?;
        let mut position = meta.position(b'R', after, limit)?;
        let journal = tx.open_table(EVENTS).map_err(unavailable)?;
        let mut events = vec![];
        for entry in journal
            .range((
                std::ops::Bound::Excluded(position),
                std::ops::Bound::Unbounded,
            ))
            .map_err(unavailable)?
            .take(limit)
        {
            let (sequence, bytes) = entry.map_err(unavailable)?;
            events.push(decode(bytes.value())?);
            position = sequence.value();
        }
        Ok(Page {
            events,
            cursor: meta.cursor(b'R', position),
        })
    }
}
