//! Bounded atomic Work input admission and source-compatible Storage fallback.
use super::*;

fn validate_count(updates: &[WorkUpdate]) -> Result<()> {
    if updates.len() > 32 {
        return Err(Error::TooLarge);
    }
    Ok(())
}

/// Validate at most 32 updates and charge their entire canonical UTF-8 JSON array.
/// Native adapters use their persisted Work byte policy before any publication.
/// This is an input byte bound; normal ledger capacity/fanout accounting still applies.
/// Serialization streams into a budget without allocating an encoded copy.
pub fn validate_work_update_batch(updates: &[WorkUpdate], max_bytes: usize) -> Result<usize> {
    validate_count(updates)?;
    struct Budget(usize);
    impl std::io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("Work update batch byte limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut budget = Budget(max_bytes);
    serde_json::to_writer(&mut budget, updates).map_err(|_| Error::TooLarge)?;
    max_bytes.checked_sub(budget.0).ok_or(Error::TooLarge)
}

pub(crate) fn default_atomic_updates<S: Storage + ?Sized>(
    storage: &S,
    updates: Vec<WorkUpdate>,
) -> Result<Vec<WorkResult>> {
    validate_count(&updates)?;
    match updates.len() {
        0 => Ok(vec![]),
        1 => {
            let update = updates.into_iter().next().ok_or(Error::Storage)?;
            storage.reaction_update(update).map(|result| vec![result])
        }
        _ => Err(Error::Unsupported("atomic Work update batches".into())),
    }
}
