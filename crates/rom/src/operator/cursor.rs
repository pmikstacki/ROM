//! Self-contained moving pagination over public opaque handles.
use super::*;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    generation: String,
    filter: String,
    after: WorkHandle,
}
fn filter(query: &WorkQuery) -> Result<String> {
    let bytes = serde_json::to_vec(&(&query.state, &query.category, &query.definition))
        .map_err(|_| Error::Storage)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub(super) fn encode(
    generation: &str,
    query: &WorkQuery,
    after: &WorkHandle,
) -> Result<WorkCursor> {
    let bytes = serde_json::to_vec(&Cursor {
        generation: generation.into(),
        filter: filter(query)?,
        after: after.clone(),
    })
    .map_err(|_| Error::Storage)?;
    WorkCursor::try_from(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}
pub(super) fn decode(generation: &str, query: &WorkQuery) -> Result<Option<WorkHandle>> {
    let Some(token) = &query.cursor else {
        return Ok(None);
    };
    let text = token.as_str();
    if text.len() % 2 != 0 {
        return Err(Error::HistoryGap);
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|i| {
            text.get(i..i + 2)
                .ok_or(Error::HistoryGap)
                .and_then(|s| u8::from_str_radix(s, 16).map_err(|_| Error::HistoryGap))
        })
        .collect::<Result<Vec<_>>>()?;
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| Error::HistoryGap)?;
    if cursor.generation != generation || cursor.filter != filter(query)? {
        return Err(Error::HistoryGap);
    }
    Ok(Some(cursor.after))
}
