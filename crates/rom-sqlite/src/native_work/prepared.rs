//! Fully encoded touched Work values before the first native publication.
use rom::storage_support::work::WorkDelta;
use rom::{Error, Result, WorkState};

pub(crate) struct PreparedWork {
    pub(crate) delta: WorkDelta,
    pub(crate) records: Vec<(String, String, bool)>,
    pub(crate) roots: Vec<(String, String)>,
    pub(crate) header: String,
}
impl PreparedWork {
    pub(crate) fn encode(delta: WorkDelta) -> Result<Self> {
        let records = delta
            .records()
            .map(|(id, _, record)| {
                Ok((
                    id.to_owned(),
                    serde_json::to_string(record).map_err(|_| Error::Storage)?,
                    matches!(record.state, WorkState::Pending | WorkState::Leased { .. }),
                ))
            })
            .collect::<Result<_>>()?;
        let roots = delta
            .roots()
            .map(|(id, _, root)| {
                Ok((
                    id.to_owned(),
                    serde_json::to_string(root).map_err(|_| Error::Storage)?,
                ))
            })
            .collect::<Result<_>>()?;
        let header = serde_json::to_string(&delta.header().parts()).map_err(|_| Error::Storage)?;
        Ok(Self {
            delta,
            records,
            roots,
            header,
        })
    }
}
