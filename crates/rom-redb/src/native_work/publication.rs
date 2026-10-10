use redb::Table;
use rom::storage_support::work::WorkDelta;
use rom::{Error, Result, WorkState};

/// Serialized Work publication; callers validate all joint facts before writing.
pub(crate) struct PreparedWork {
    records: Vec<(String, String, bool)>,
    roots: Vec<(String, String)>,
    header: String,
}
impl PreparedWork {
    pub(crate) fn new(delta: &WorkDelta) -> Result<Self> {
        Ok(Self {
            records: delta
                .records()
                .map(|(id, _, r)| {
                    Ok((
                        id.to_owned(),
                        serde_json::to_string(r).map_err(|_| Error::Storage)?,
                        matches!(r.state, WorkState::Pending | WorkState::Leased { .. }),
                    ))
                })
                .collect::<Result<_>>()?,
            roots: delta
                .roots()
                .map(|(id, _, r)| {
                    Ok((
                        id.to_owned(),
                        serde_json::to_string(r).map_err(|_| Error::Storage)?,
                    ))
                })
                .collect::<Result<_>>()?,
            header: serde_json::to_string(&delta.header().parts()).map_err(|_| Error::Storage)?,
        })
    }
    pub(crate) fn publish(
        &self,
        state: &mut Table<'_, &'static str, &'static str>,
        records: &mut Table<'_, &'static str, &'static str>,
        roots: &mut Table<'_, &'static str, &'static str>,
        active: &mut Table<'_, &'static str, u8>,
        checkpoint: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        for (id, raw, is_active) in &self.records {
            records
                .insert(id.as_str(), raw.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
            if *is_active {
                active
                    .insert(id.as_str(), 1)
                    .map_err(|_| Error::NotCommitted)?;
            } else {
                active
                    .remove(id.as_str())
                    .map_err(|_| Error::NotCommitted)?;
            }
            checkpoint()?;
        }
        for (id, raw) in &self.roots {
            roots
                .insert(id.as_str(), raw.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        state
            .insert("work_header", self.header.as_str())
            .map_err(|_| Error::NotCommitted)?;
        checkpoint()?;
        Ok(())
    }
}
