//! Feature-gated engine identity from an ephemeral in-memory connection.
use crate::Sqlite;
use rom::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedEngineIdentity {
    pub version: String,
    pub source_id: String,
    pub compile_options: Vec<String>,
}
impl LinkedEngineIdentity {
    /// Validate fixed diagnostic bounds; this does not assert a selected profile.
    pub fn validate(&self) -> Result<()> {
        fn text(value: &str, maximum: usize) -> bool {
            !value.is_empty()
                && value.len() <= maximum
                && value.bytes().all(|b| (32..=126).contains(&b))
        }
        let mut seen = std::collections::BTreeSet::new();
        if !text(&self.version, 32)
            || !text(&self.source_id, 192)
            || self.compile_options.len() > 128
            || self
                .compile_options
                .iter()
                .any(|value| !text(value, 256) || !seen.insert(value))
        {
            return Err(Error::Storage);
        }
        Ok(())
    }
}
impl Sqlite {
    /// Read the linked engine without opening a persisted database or a Runtime.
    pub fn linked_engine_identity() -> Result<LinkedEngineIdentity> {
        let connection = rusqlite::Connection::open_in_memory().map_err(|_| Error::Storage)?;
        let (version, source_id) = connection
            .query_row("SELECT sqlite_version(), sqlite_source_id()", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|_| Error::Storage)?;
        let mut statement = connection
            .prepare("PRAGMA compile_options")
            .map_err(|_| Error::Storage)?;
        let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
        let mut compile_options = Vec::new();
        while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            if compile_options.len() == 128 {
                return Err(Error::TooLarge);
            }
            compile_options.push(row.get::<_, String>(0).map_err(|_| Error::Storage)?);
        }
        compile_options.sort();
        let identity = LinkedEngineIdentity {
            version,
            source_id,
            compile_options,
        };
        identity.validate()?;
        Ok(identity)
    }
}

#[cfg(test)]
mod tests;
