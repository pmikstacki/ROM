use super::*;

/// Resume position within a persisted history generation and Resource kind.
/// Positions acknowledge inspected history, including facts the caller cannot read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalCursor {
    pub generation: String,
    pub kind: String,
    pub position: u64,
}

/// Durable storage fact. Adapters return these only to the trusted runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEvent {
    pub position: u64,
    pub identity: String,
    pub row: Row,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalPage {
    pub events: Vec<JournalEvent>,
    pub cursor: JournalCursor,
}
