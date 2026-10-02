use rom_persistence_core::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub(crate) const CAPABILITIES: Capabilities = Capabilities {
    durable_atomic_commit: true,
    durable_receipts: true,
    resumable_journal: true,
    snapshot_queries: false,
    multi_resource_commit: false,
    max_bytes: 65536,
    max_events: 32,
};
pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value).map_err(|_| Error::Unavailable)
}
pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    serde_json::from_slice(bytes).map_err(|_| Error::Unavailable)
}
#[derive(Serialize, Deserialize)]
pub(crate) struct StoredReceipt {
    pub transition: Transition,
    pub receipt: Receipt,
}
impl StoredReceipt {
    pub fn resolve(&self, transition: &Transition) -> Result<Receipt, Error> {
        if &self.transition == transition {
            Ok(self.receipt.clone())
        } else {
            Err(Error::IdentityMismatch)
        }
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Meta {
    pub namespace: [u8; 16],
    pub sequence: u64,
}
impl Meta {
    pub fn new() -> Result<Self, Error> {
        let mut namespace = [0; 16];
        getrandom::fill(&mut namespace).map_err(|_| Error::Unavailable)?;
        Ok(Self {
            namespace,
            sequence: 0,
        })
    }
    pub fn allocate(&mut self) -> Result<u64, Error> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(Error::Rejected("journal capacity"))?;
        Ok(self.sequence)
    }
    pub fn cursor(&self, backend: u8, position: u64) -> Cursor {
        let mut bytes = vec![1, backend];
        bytes.extend_from_slice(&self.namespace);
        bytes.extend_from_slice(&position.to_be_bytes());
        Cursor::from_adapter_bytes(bytes)
    }
    pub fn position(
        &self,
        backend: u8,
        cursor: Option<&Cursor>,
        limit: usize,
    ) -> Result<u64, Error> {
        if !(1..=128).contains(&limit) {
            return Err(Error::Rejected("journal page limit"));
        }
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        let bytes = cursor.adapter_bytes();
        if bytes.len() != 26
            || bytes[0] != 1
            || bytes[1] != backend
            || bytes[2..18] != self.namespace
        {
            return Err(Error::InvalidCursor);
        }
        let position =
            u64::from_be_bytes(bytes[18..26].try_into().map_err(|_| Error::InvalidCursor)?);
        if position > self.sequence {
            return Err(Error::InvalidCursor);
        }
        Ok(position)
    }
}
