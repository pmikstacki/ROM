//! Throwaway semantic persistence boundary: no backend types or query language.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceKey {
    pub kind: String,
    pub id: String,
}

impl ResourceKey {
    pub fn new(kind: &str, id: &str) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    pub key: ResourceKey,
    pub revision: u64,
    /// None is a retained tombstone; it does not reset the revision.
    pub value: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub action: String,
    pub key: ResourceKey,
    pub expected_revision: Option<u64>,
    pub value: Option<Vec<u8>>,
    pub events: Vec<Vec<u8>>,
}

impl Transition {
    pub fn validate(&self, capabilities: Capabilities) -> Result<(), Error> {
        if self.action.is_empty() || self.key.kind.is_empty() || self.key.id.is_empty() {
            return Err(Error::Rejected("empty identity"));
        }
        if self.events.len() > capabilities.max_events {
            return Err(Error::Rejected("event count limit"));
        }
        let mut bytes = self.action.len();
        for size in [
            self.key.kind.len(),
            self.key.id.len(),
            self.value.as_ref().map_or(0, Vec::len),
        ]
        .into_iter()
        .chain(self.events.iter().map(Vec::len))
        {
            bytes = bytes
                .checked_add(size)
                .ok_or(Error::Rejected("size overflow"))?;
        }
        if bytes > capabilities.max_bytes {
            return Err(Error::Rejected("transition byte limit"));
        }
        self.next_revision()?;
        Ok(())
    }
    pub fn next_revision(&self) -> Result<u64, Error> {
        self.expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::Rejected("revision overflow"))
    }
    /// Called while the adapter protects the read and commit with one transaction.
    pub fn materialize(
        &self,
        actual: Option<u64>,
    ) -> Result<(Resource, Receipt, Vec<Event>), Error> {
        if actual != self.expected_revision {
            return Err(Error::Conflict { actual });
        }
        let revision = self.next_revision()?;
        let resource = Resource {
            key: self.key.clone(),
            revision,
            value: self.value.clone(),
        };
        let receipt = Receipt {
            action: self.action.clone(),
            key: self.key.clone(),
            revision,
        };
        let events = self
            .events
            .iter()
            .enumerate()
            .map(|(ordinal, payload)| {
                Ok(Event {
                    action: self.action.clone(),
                    ordinal: u32::try_from(ordinal)
                        .map_err(|_| Error::Rejected("event ordinal overflow"))?,
                    key: self.key.clone(),
                    revision,
                    payload: payload.clone(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok((resource, receipt, events))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub action: String,
    pub key: ResourceKey,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub action: String,
    pub ordinal: u32,
    pub key: ResourceKey,
    pub revision: u64,
    pub payload: Vec<u8>,
}

/// Core transports bytes but never interprets an adapter's cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor(Vec<u8>);

impl Cursor {
    pub fn from_adapter_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
    pub fn adapter_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub events: Vec<Event>,
    pub cursor: Cursor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptStatus {
    Found(Receipt),
    /// Absence at this read is NOT evidence of terminal rollback.
    AbsentNow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Conflict { actual: Option<u64> },
    IdentityMismatch,
    Rejected(&'static str),
    Unsupported(&'static str),
    InvalidCursor,
    Unavailable,
    NotCommitted,
    Unknown { action: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    pub durable_atomic_commit: bool,
    pub durable_receipts: bool,
    pub resumable_journal: bool,
    pub snapshot_queries: bool,
    pub multi_resource_commit: bool,
    pub max_bytes: usize,
    pub max_events: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Requirements {
    pub snapshot_queries: bool,
    pub multi_resource_commit: bool,
}

pub trait Storage: Send + Sync {
    fn capabilities(&self) -> Capabilities;
    fn load(&self, key: &ResourceKey) -> Result<Option<Resource>, Error>;
    fn commit(&self, transition: &Transition) -> Result<Receipt, Error>;
    fn receipt(&self, action: &str) -> Result<ReceiptStatus, Error>;
    fn journal(&self, after: Option<&Cursor>, limit: usize) -> Result<Page, Error>;
}

pub struct Runtime<S> {
    storage: S,
}

impl<S: Storage> Runtime<S> {
    pub fn new(storage: S, requirements: Requirements) -> Result<Self, Error> {
        let c = storage.capabilities();
        if !c.durable_atomic_commit || !c.durable_receipts || !c.resumable_journal {
            return Err(Error::Unsupported("mandatory durable persistence"));
        }
        if requirements.snapshot_queries && !c.snapshot_queries {
            return Err(Error::Unsupported("snapshot queries"));
        }
        if requirements.multi_resource_commit && !c.multi_resource_commit {
            return Err(Error::Unsupported("multi-resource commit"));
        }
        Ok(Self { storage })
    }
    pub fn execute(&self, transition: &Transition) -> Result<Receipt, Error> {
        transition.validate(self.storage.capabilities())?;
        self.storage.commit(transition)
    }
    pub fn load(&self, key: &ResourceKey) -> Result<Option<Resource>, Error> {
        self.storage.load(key)
    }
    pub fn receipt(&self, action: &str) -> Result<ReceiptStatus, Error> {
        self.storage.receipt(action)
    }
    pub fn journal(&self, after: Option<&Cursor>, limit: usize) -> Result<Page, Error> {
        self.storage.journal(after, limit)
    }
}
