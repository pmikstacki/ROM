//! Public, credential-free browser storage configuration approved by the deployment host.
use rom::{Error, Result};
use serde::{Deserialize, Serialize};

const MAX_JSON: usize = 65_536;
fn invalid() -> Error {
    Error::Invalid {
        kind: "studio-bootstrap".into(),
        field: "configuration".into(),
    }
}
fn identifier(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(())
}

/// Explicit browser record allocation. These settings grant no backend permission.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BrowserStore {
    name: String,
    max_bytes: usize,
    max_slots: usize,
    timeout_ms: u64,
}
impl BrowserStore {
    pub fn new(name: &str, max_bytes: usize, max_slots: usize, timeout_ms: u64) -> Result<Self> {
        let value = Self {
            name: name.into(),
            max_bytes,
            max_slots,
            timeout_ms,
        };
        value.validate(0)?;
        Ok(value)
    }
    fn validate(&self, payload: usize) -> Result<()> {
        identifier(&self.name)?;
        if self.max_bytes < payload.checked_add(4096).ok_or(Error::TooLarge)?
            || self.max_bytes > 4 * 1024 * 1024
            || !(1..=4096).contains(&self.max_slots)
            || !(1..=60_000).contains(&self.timeout_ms)
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Recovery {
    namespace: String,
    retry_epoch: String,
    max_bytes: usize,
    intent_store: BrowserStore,
    editor_store: BrowserStore,
}

/// Stable session-binding namespace and explicit draft/command storage. No identity is inferred.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudioBootstrap {
    version: u8,
    authority: String,
    recovery: Recovery,
}
impl StudioBootstrap {
    pub fn new(
        authority: &str,
        namespace: &str,
        retry_epoch: u64,
        max_bytes: usize,
        intent_store: BrowserStore,
        editor_store: BrowserStore,
    ) -> Result<Self> {
        let value = Self {
            version: 1,
            authority: authority.into(),
            recovery: Recovery {
                namespace: namespace.into(),
                retry_epoch: retry_epoch.to_string(),
                max_bytes,
                intent_store,
                editor_store,
            },
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        identifier(&self.authority)?;
        identifier(&self.recovery.namespace)?;
        let epoch = self
            .recovery
            .retry_epoch
            .parse::<u64>()
            .map_err(|_| invalid())?;
        if self.version != 1
            || epoch.to_string() != self.recovery.retry_epoch
            || !(1..=1024 * 1024).contains(&self.recovery.max_bytes)
        {
            return Err(invalid());
        }
        self.recovery
            .intent_store
            .validate(self.recovery.max_bytes)?;
        self.recovery
            .editor_store
            .validate(self.recovery.max_bytes)?;
        Ok(())
    }
    pub fn json(&self) -> Result<String> {
        self.validate()?;
        let value = serde_json::to_string(self).map_err(|_| Error::Storage)?;
        if value.len() > MAX_JSON {
            return Err(Error::TooLarge);
        }
        Ok(value)
    }
    pub(crate) fn html(&self) -> Result<String> {
        let value = self
            .json()?
            .replace('&', "\\u0026")
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029");
        if value.len() > MAX_JSON {
            return Err(Error::TooLarge);
        }
        Ok(format!(
            "<script type=\"application/json\" id=\"rom-studio-auth-profile\">{value}</script>"
        ))
    }
}
