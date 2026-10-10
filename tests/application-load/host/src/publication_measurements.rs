//! Opt-in numeric publication evidence; this module never records Resource data.
use rom_sqlite::PublicationSnapshot;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
pub(crate) struct Batch {
    pub completed_rows: usize,
    pub cumulative_samples_and_encoded_bytes: [[u64; 2]; 10],
}

pub(crate) fn record(
    batches: &mut Vec<Batch>,
    rows: usize,
    snapshot: PublicationSnapshot,
) -> rom::Result<()> {
    if batches.len() >= 100 || rows > 10000 {
        return Err(rom::Error::TooLarge);
    }
    batches.push(Batch {
        completed_rows: rows,
        cumulative_samples_and_encoded_bytes: snapshot
            .entries
            .map(|entry| [entry.samples, entry.encoded_bytes]),
    });
    Ok(())
}

fn summary(snapshot: &PublicationSnapshot) -> Value {
    let entries: Vec<_> = snapshot
        .entries
        .iter()
        .map(|entry| {
            json!({
                "operation": entry.operation.as_str(), "category": entry.category.as_str(),
                "samples": entry.samples, "encoded_bytes": entry.encoded_bytes,
                "dropped_samples": entry.dropped_samples, "saturated": entry.saturated,
            })
        })
        .collect();
    json!({"entries":entries,"poison_recovered":snapshot.poison_recovered})
}

pub(crate) fn proof(
    baseline: PublicationSnapshot,
    batches: &[Batch],
    final_snapshot: PublicationSnapshot,
) -> Value {
    let usable = [&baseline, &final_snapshot].into_iter().all(|snapshot| {
        !snapshot.poison_recovered
            && snapshot
                .entries
                .iter()
                .all(|entry| entry.dropped_samples == 0 && !entry.saturated)
    });
    json!({
        "semantics":"successful-sql-publication-payload-bytes",
        "acceptance":false,"durable_commit_bytes":false,"wal_write_bytes":false,
        "scope":"only the ten named metadata/event/work payload cells; Resource, receipt, effect, row-key and index writes are excluded; rolled-back successful SQL remains counted",
        "batch_order":"same fixed ten operation/category cells as baseline and final entries",
        "batch_comparisons_usable":usable,
        "baseline":summary(&baseline),"batches":batches,"final":summary(&final_snapshot),
    })
}

#[cfg(test)]
#[path = "publication_measurements_tests.rs"]
mod tests;
