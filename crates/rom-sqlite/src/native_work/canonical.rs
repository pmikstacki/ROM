//! Bounded canonical reconstruction for open, archive and operator maintenance.
use super::validate_layout;
use rom::storage_support::work::{RootAccount, WorkHeader, WorkHeaderParts, WorkImage};
use rom::{Error, OperatorLedger, Result, StorageState, WorkRecord};
use rom_backup::BackupLimits;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Canonical {
    pub(crate) state: StorageState,
    pub(crate) derived_charges: Vec<usize>,
}
struct Bounds {
    limits: BackupLimits,
    bytes: usize,
    records: usize,
}
impl Bounds {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        self.bytes = self.bytes.checked_add(bytes).ok_or(Error::TooLarge)?;
        self.records = self.records.checked_add(1).ok_or(Error::TooLarge)?;
        if self.bytes > self.limits.max_bytes || self.records > self.limits.max_records {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
}
fn singleton(c: &Connection, table: &str, bounds: &mut Bounds) -> Result<String> {
    let mut statement = c
        .prepare(&format!("SELECT data FROM {table} WHERE id=1"))
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    let row = rows
        .next()
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    let text = row
        .get_ref(0)
        .map_err(|_| Error::Storage)?
        .as_str()
        .map_err(|_| Error::Storage)?;
    bounds.charge(text.len().checked_add(1).ok_or(Error::TooLarge)?)?;
    Ok(text.to_owned())
}
fn keyed(c: &Connection, table: &str, bounds: &mut Bounds) -> Result<Vec<(String, String)>> {
    let mut statement = c
        .prepare(&format!("SELECT id,data FROM {table} ORDER BY id"))
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let id = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let text = row
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        bounds.charge(id.len().checked_add(text.len()).ok_or(Error::TooLarge)?)?;
        entries.push((id.to_owned(), text.to_owned()));
    }
    Ok(entries)
}
pub(crate) fn reconstruct(c: &Connection, limits: BackupLimits) -> Result<Canonical> {
    validate_layout(c)?;
    let format: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if !matches!(format, 10 | 11) {
        return Err(Error::Unsupported("SQLite storage format".into()));
    }
    if format == 11 {
        crate::native_journal::admit_inventory(c, limits)?;
    } else {
        crate::native_journal::admit_inventory_for_format(c, limits, format)?;
    }
    let mut bounds = Bounds {
        limits,
        bytes: 0,
        records: 0,
    };
    let metadata_text = singleton(c, "rom_state", &mut bounds)?;
    // Bounds precede decoding or copying each native value, including whitespace.
    let (metadata, journal_derived) = if format == 11 {
        crate::native_journal::reconstruct_metadata(c, limits)?
    } else {
        (
            serde_json::from_str::<rom::storage_support::metadata::StorageMetadata>(&metadata_text)
                .map_err(|_| Error::Storage)?,
            Vec::new(),
        )
    };
    let header_text = singleton(c, "work_header", &mut bounds)?;
    let header = WorkHeader::from_parts(
        serde_json::from_str::<WorkHeaderParts>(&header_text).map_err(|_| Error::Storage)?,
    )?;
    if header.retry_epochs != metadata.retry_epochs() {
        return Err(Error::Storage);
    }
    let operator_text = singleton(c, "operator_state", &mut bounds)?;
    let operator: OperatorLedger =
        serde_json::from_str(&operator_text).map_err(|_| Error::Storage)?;
    let mut derived_charges = vec![header_text.len() + 1, operator_text.len() + 1];
    if format == 11 {
        derived_charges.push(metadata_text.len().checked_add(1).ok_or(Error::TooLarge)?);
        derived_charges.extend(journal_derived);
    }
    let mut records = BTreeMap::new();
    for (id, text) in keyed(c, "work_records", &mut bounds)? {
        let record: WorkRecord = serde_json::from_str(&text).map_err(|_| Error::Storage)?;
        if record.pending.id != id || records.insert(id, record).is_some() {
            return Err(Error::Storage);
        }
    }
    let mut roots = BTreeMap::new();
    for (id, text) in keyed(c, "work_roots", &mut bounds)? {
        let root: RootAccount = serde_json::from_str(&text).map_err(|_| Error::Storage)?;
        root.validate()?;
        derived_charges.push(id.len().checked_add(text.len()).ok_or(Error::TooLarge)?);
        if roots.insert(id, root).is_some() {
            return Err(Error::Storage);
        }
    }
    let mut active = BTreeSet::new();
    let mut statement = c
        .prepare("SELECT id FROM work_active ORDER BY id")
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let id = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        bounds.charge(id.len())?;
        derived_charges.push(id.len());
        if !active.insert(id.to_owned()) {
            return Err(Error::Storage);
        }
    }
    let image = WorkImage::from_native(header, records, roots, active)?;
    let state = metadata.reassemble(image.canonical(), operator);
    state.check_limits(&state.storage_limits())?;
    Ok(Canonical {
        state,
        derived_charges,
    })
}
