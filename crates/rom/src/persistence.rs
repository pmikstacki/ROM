use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub kind: String,
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub key: Key,
    pub revision: u64,
    pub value: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub identity: String,
    pub fingerprint: String,
    pub row: Row,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub expected: Option<u64>,
    pub receipt: Receipt,
    pub changed: bool,
    pub effects: Vec<Intent>,
}
#[derive(Clone, Copy)]
pub struct Capabilities {
    pub atomic_bundle: bool,
    pub snapshots: bool,
    pub effects: bool,
}
/// Single ROM owner per adapter. load/snapshot are authoritative; commit MUST conditionally
/// arbitrate expected revision and identity and atomically persist row/event/receipt/effects.
/// Implementations must not claim rollback for uncertain acknowledgment.
pub trait Storage: Send + Sync + 'static {
    fn capabilities(&self) -> Capabilities;
    fn load(&self, key: &Key) -> Result<Option<Row>>;
    /// Reject overflow without truncation. Charge serialized full Row bytes,
    /// using checked arithmetic before decoding/appending each row.
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>>;
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>>;
    fn commit(&self, bundle: &Bundle) -> Result<Receipt>;
}
