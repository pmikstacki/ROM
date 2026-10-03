//! Strict request decoding and response admission for operator recovery.
use crate::{DeliveryOutcome, Error, Key, Result, StopReason};
use serde::{Deserialize, Serialize};
mod identifiers;
pub use identifiers::{WorkCursor, WorkHandle};

pub const OPERATOR_PROTOCOL_VERSION: u32 = 1;
pub const MAX_OPERATOR_IDENTIFIER_BYTES: usize = 1024;
pub const MAX_STORAGE_GENERATION_BYTES: usize = 128;

pub(crate) fn bounded(value: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.len() > max {
        return Err(Error::TooLarge);
    }
    Ok(())
}
fn check_protocol_version(version: u32) -> Result<()> {
    if version != OPERATOR_PROTOCOL_VERSION {
        return Err(Error::Unsupported(
            "operator protocol version is unsupported".into(),
        ));
    }
    Ok(())
}
fn protocol_version<'de, D: serde::Deserializer<'de>>(de: D) -> std::result::Result<u32, D::Error> {
    let version = u32::deserialize(de)?;
    check_protocol_version(version).map_err(serde::de::Error::custom)?;
    Ok(version)
}
fn identifier<'de, D: serde::Deserializer<'de>>(de: D) -> std::result::Result<String, D::Error> {
    let value = String::deserialize(de)?;
    bounded(&value, MAX_OPERATOR_IDENTIFIER_BYTES).map_err(serde::de::Error::custom)?;
    Ok(value)
}
fn generation<'de, D: serde::Deserializer<'de>>(de: D) -> std::result::Result<String, D::Error> {
    let value = String::deserialize(de)?;
    bounded(&value, MAX_STORAGE_GENERATION_BYTES).map_err(serde::de::Error::custom)?;
    Ok(value)
}
fn evidence<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> std::result::Result<Option<String>, D::Error> {
    let value = Option::<String>::deserialize(de)?;
    if let Some(value) = &value {
        bounded(value, MAX_OPERATOR_IDENTIFIER_BYTES).map_err(serde::de::Error::custom)?;
    }
    Ok(value)
}
fn limit<'de, D: serde::Deserializer<'de>>(de: D) -> std::result::Result<usize, D::Error> {
    let value = usize::deserialize(de)?;
    if value == 0 {
        return Err(serde::de::Error::custom("page limit must be nonzero"));
    }
    Ok(value)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkVersion {
    #[serde(deserialize_with = "generation")]
    pub generation: String,
    pub revision: u64,
}
impl WorkVersion {
    pub fn validate(&self) -> Result<()> {
        bounded(&self.generation, MAX_STORAGE_GENERATION_BYTES)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum WorkControlOperation {
    Retry,
    Reconcile {
        #[serde(default, deserialize_with = "evidence")]
        evidence_ref: Option<String>,
    },
}
impl WorkControlOperation {
    pub fn validate(&self) -> Result<()> {
        if let Self::Reconcile {
            evidence_ref: Some(value),
        } = self
        {
            bounded(value, MAX_OPERATOR_IDENTIFIER_BYTES)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkControlRequest {
    pub handle: WorkHandle,
    pub expected: WorkVersion,
    #[serde(deserialize_with = "identifier")]
    pub key: String,
    pub retry_epoch: u64,
    pub operation: WorkControlOperation,
}
impl WorkControlRequest {
    pub fn validate(&self) -> Result<()> {
        self.expected.validate()?;
        bounded(&self.key, MAX_OPERATOR_IDENTIFIER_BYTES)?;
        self.operation.validate()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkCategory {
    Reaction,
    Notification,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkStatus {
    Pending,
    Leased,
    AwaitingReconciliation,
    Done,
    Stopped(StopReason),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkDefinition {
    #[serde(deserialize_with = "identifier")]
    pub name: String,
    pub version: u32,
}
impl WorkDefinition {
    pub fn validate(&self) -> Result<()> {
        bounded(&self.name, MAX_OPERATOR_IDENTIFIER_BYTES)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkView {
    #[serde(deserialize_with = "protocol_version")]
    pub protocol_version: u32,
    pub handle: WorkHandle,
    pub version: WorkVersion,
    pub category: WorkCategory,
    pub definition: WorkDefinition,
    pub state: WorkStatus,
    pub attempts: u32,
    pub due: u64,
    pub delivery: Option<DeliveryOutcome>,
    pub source: Option<Key>,
    pub target: Option<Key>,
}
impl WorkView {
    fn validate_fields(&self) -> Result<()> {
        check_protocol_version(self.protocol_version)?;
        self.version.validate()?;
        self.definition.validate()
    }
    pub fn validate(&self, bounds: &WorkResponseLimits) -> Result<()> {
        self.validate_fields()?;
        bounds.check_response(self, 1)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkQuery {
    #[serde(default)]
    pub state: Option<WorkStatus>,
    #[serde(default)]
    pub category: Option<WorkCategory>,
    #[serde(default, deserialize_with = "evidence")]
    pub definition: Option<String>,
    #[serde(deserialize_with = "limit")]
    pub limit: usize,
    #[serde(default)]
    pub cursor: Option<WorkCursor>,
}
impl WorkQuery {
    pub fn validate(&self, bounds: &WorkResponseLimits) -> Result<()> {
        bounds.validate()?;
        if self.limit == 0 || self.limit > bounds.max_records {
            return Err(Error::TooLarge);
        }
        if let Some(name) = &self.definition {
            bounded(name, MAX_OPERATOR_IDENTIFIER_BYTES)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkResponseLimits {
    pub max_records: usize,
    pub max_bytes: usize,
}
impl Default for WorkResponseLimits {
    fn default() -> Self {
        Self {
            max_records: 64,
            max_bytes: 64 * 1024,
        }
    }
}
impl WorkResponseLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_records == 0 || self.max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        self.max_records
            .checked_mul(std::mem::size_of::<WorkView>())
            .ok_or(Error::TooLarge)?;
        Ok(())
    }
    /// Charge the complete serialized envelope before any response is disclosed.
    pub fn check_response<T: Serialize>(&self, response: &T, records: usize) -> Result<()> {
        self.validate()?;
        if records > self.max_records
            || serde_json::to_vec(response)
                .map_err(|_| Error::Storage)?
                .len()
                > self.max_bytes
        {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPage {
    #[serde(deserialize_with = "protocol_version")]
    pub protocol_version: u32,
    pub records: Vec<WorkView>,
    pub cursor: Option<WorkCursor>,
}
impl WorkPage {
    pub fn validate(&self, bounds: &WorkResponseLimits) -> Result<()> {
        check_protocol_version(self.protocol_version)?;
        for record in &self.records {
            record.validate_fields()?;
        }
        bounds.check_response(self, self.records.len())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorCapabilities {
    #[serde(deserialize_with = "protocol_version")]
    pub protocol_version: u32,
    pub inspect: bool,
    pub retry: bool,
    pub reconcile: bool,
}
impl OperatorCapabilities {
    pub fn validate(&self, bounds: &WorkResponseLimits) -> Result<()> {
        check_protocol_version(self.protocol_version)?;
        bounds.check_response(self, 0)
    }
}
impl Default for OperatorCapabilities {
    fn default() -> Self {
        Self {
            protocol_version: OPERATOR_PROTOCOL_VERSION,
            inspect: false,
            retry: false,
            reconcile: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkControlOutcome {
    Scheduled,
    Completed,
    Stopped(StopReason),
    Unresolved,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkControlResult {
    #[serde(deserialize_with = "protocol_version")]
    pub protocol_version: u32,
    pub handle: WorkHandle,
    pub version: WorkVersion,
    #[serde(deserialize_with = "identifier")]
    pub key: String,
    pub operation: WorkControlOperation,
    pub outcome: WorkControlOutcome,
    pub replayed: bool,
}
impl WorkControlResult {
    /// Validate a reply's bounds and correspondence to the exact submitted request.
    pub fn validate_for(
        &self,
        request: &WorkControlRequest,
        bounds: &WorkResponseLimits,
    ) -> Result<()> {
        request.validate()?;
        self.validate(bounds)?;
        self.validate_correspondence(request)
    }
    pub(crate) fn validate_correspondence(&self, request: &WorkControlRequest) -> Result<()> {
        if self.handle != request.handle
            || self.key != request.key
            || self.operation != request.operation
            || self.version.generation != request.expected.generation
        {
            return Err(Error::Storage);
        }
        let coherent = match (&request.operation, &self.outcome) {
            (WorkControlOperation::Reconcile { .. }, WorkControlOutcome::Unresolved) => {
                !self.replayed && self.version == request.expected
            }
            (WorkControlOperation::Retry, WorkControlOutcome::Scheduled)
            | (
                WorkControlOperation::Reconcile { .. },
                WorkControlOutcome::Scheduled
                | WorkControlOutcome::Completed
                | WorkControlOutcome::Stopped(_),
            ) => request.expected.revision.checked_add(1) == Some(self.version.revision),
            _ => false,
        };
        if !coherent {
            return Err(Error::Storage);
        }
        Ok(())
    }
    pub fn validate(&self, bounds: &WorkResponseLimits) -> Result<()> {
        check_protocol_version(self.protocol_version)?;
        self.version.validate()?;
        bounded(&self.key, MAX_OPERATOR_IDENTIFIER_BYTES)?;
        self.operation.validate()?;
        bounds.check_response(self, 1)
    }
}
