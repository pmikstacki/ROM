//! Admission of touched native values before allocation and decoding.
use super::Reader;
use rom::{Descriptor, Error, Key, Receipt, Result, Row};
use rusqlite::{Params, params};
impl Reader<'_> {
    fn admitted_text(
        &self,
        sql: &str,
        params: impl Params,
        key_bytes: usize,
    ) -> Result<Option<String>> {
        let mut statement = self
            .connection
            .prepare_cached(sql)
            .map_err(|_| Error::Storage)?;
        let mut rows = statement.query(params).map_err(|_| Error::Storage)?;
        let Some(row) = rows.next().map_err(|_| Error::Storage)? else {
            return Ok(None);
        };
        let raw = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        self.charge(raw.len().checked_add(key_bytes).ok_or(Error::TooLarge)?, 1)?;
        Ok(Some(raw.to_owned()))
    }
    pub(crate) fn resource(&self, key: &Key) -> Result<Option<(Row, usize)>> {
        self.admitted_text(
            "SELECT data FROM resources WHERE kind=? AND id=?",
            params![key.kind, key.id],
            key.kind
                .len()
                .checked_add(key.id.len())
                .ok_or(Error::TooLarge)?,
        )?
        .map(|raw| {
            let row: Row = serde_json::from_str(&raw).map_err(|_| Error::Storage)?;
            if row.key != *key {
                return Err(Error::Storage);
            }
            Ok((row, raw.len()))
        })
        .transpose()
    }
    pub(crate) fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.admitted_text("SELECT data FROM receipts WHERE identity=?", [id], id.len())?
            .map(|raw| {
                let receipt: Receipt = serde_json::from_str(&raw).map_err(|_| Error::Storage)?;
                if receipt.identity != id {
                    return Err(Error::Storage);
                }
                Ok(receipt)
            })
            .transpose()
    }
    pub(crate) fn descriptor(&self, kind: &str) -> Result<Descriptor> {
        let raw = self
            .admitted_text("SELECT data FROM schemas WHERE kind=?", [kind], kind.len())?
            .ok_or(Error::Unregistered)?;
        crate::references::decode_descriptor(kind, &raw)
    }
}
