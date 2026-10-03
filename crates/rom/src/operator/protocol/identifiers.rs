//! Opaque work identities and bounded continuation tokens.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct WorkHandle(String);
impl WorkHandle {
    /// Derive from the internal identity only, never from a frozen payload or credential.
    pub fn from_work_id(id: &str) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"rom/operator/work-handle/v1\0");
        digest.update(id.as_bytes());
        Self(format!("{:x}", digest.finalize()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for WorkHandle {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Invalid {
                kind: "operator".into(),
                field: "handle".into(),
            });
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct WorkCursor(String);
impl WorkCursor {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for WorkCursor {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        bounded(&value, 2048)?;
        Ok(Self(value))
    }
}
