//! Response consistency checks. They do not prove an adapter omitted no matches.
use super::{
    protocol::*,
    selection::{native_eligible, valid_snapshot},
};
use crate::{Error, Result, Row};

pub(crate) fn validate_query_read(
    request: &StorageQuery,
    bounds: QueryBounds,
    response: QueryRead,
) -> Result<Vec<Row>> {
    match response {
        QueryRead::Reference { rows } => validate_rows(request, bounds, rows).map(|(rows, _)| rows),
        QueryRead::NativeCandidates {
            rows,
            admission,
            binding,
        } => {
            if !native_eligible(request)
                || binding.request != *request
                || !valid_snapshot(&binding.snapshot)
            {
                return Err(Error::Storage);
            }
            if rows.len() > admission.rows
                || admission.rows > admission.persisted_bytes
                || admission.rows > admission.canonical_bytes
                || (admission.rows == 0
                    && (admission.persisted_bytes != 0 || admission.canonical_bytes != 0))
            {
                return Err(Error::Storage);
            }
            if admission.rows > bounds.max_rows
                || admission.persisted_bytes > bounds.max_bytes
                || admission.canonical_bytes > bounds.max_bytes
            {
                return Err(Error::TooLarge);
            }
            let (rows, bytes) = validate_rows(request, bounds, rows)?;
            if bytes > admission.canonical_bytes {
                return Err(Error::Storage);
            }
            Ok(rows)
        }
    }
}
/// Preserve legacy validation order: count; each scope and canonical byte charge;
/// then ascending IDs and duplicate rejection. No policy or codec executes here.
fn validate_rows(
    request: &StorageQuery,
    bounds: QueryBounds,
    mut rows: Vec<Row>,
) -> Result<(Vec<Row>, usize)> {
    if rows.len() > bounds.max_rows {
        return Err(Error::TooLarge);
    }
    let mut bytes = 0usize;
    for row in &rows {
        if row.key.kind != request.descriptor.kind {
            return Err(Error::Storage);
        }
        bytes = bytes
            .checked_add(serde_json::to_vec(row).map_err(|_| Error::Storage)?.len())
            .ok_or(Error::TooLarge)?;
        if bytes > bounds.max_bytes {
            return Err(Error::TooLarge);
        }
    }
    rows.sort_by(|a, b| a.key.id.cmp(&b.key.id));
    if rows.windows(2).any(|pair| pair[0].key.id == pair[1].key.id) {
        return Err(Error::Storage);
    }
    Ok((rows, bytes))
}
