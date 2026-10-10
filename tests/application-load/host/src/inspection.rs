//! Read a newly copied failure database through the public Storage contract.
use crate::{application::LoadRecord, configuration::Configuration, database::Database};
use rom::{Resource, WorkState};
use std::{collections::BTreeMap, io::Write, os::unix::fs::OpenOptionsExt};
pub fn record(c: &Configuration) -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::open(&c.adapter, &c.directory.join("database"))?;
    let storage = database.storage();
    let rows = storage.snapshot(LoadRecord::KIND, 12000, 32 * 1024 * 1024)?;
    let work = storage.work_snapshot(12000, 16 * 1024 * 1024)?;
    let mut states = BTreeMap::<&str, usize>::new();
    for record in &work.records {
        let state = match record.state {
            WorkState::Pending => "Pending",
            WorkState::AwaitingReconciliation => "AwaitingReconciliation",
            WorkState::Leased { .. } => "Leased",
            WorkState::Done => "Done",
            WorkState::Stopped(_) => "Stopped",
        };
        *states.entry(state).or_default() += 1;
    }
    let bytes = serde_json::to_vec(
        &serde_json::json!({"rows":rows.len(),"work_records":work.records.len(),"states":states,"candidate_bytes":serde_json::to_vec(&rows)?.len(),"work_bytes":serde_json::to_vec(&work)?.len(),"copied_database_reopened":true}),
    )?;
    if bytes.len() > 256 * 1024 {
        return Err("finite inspection evidence".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(c.directory.join("load-inspection.json"))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
