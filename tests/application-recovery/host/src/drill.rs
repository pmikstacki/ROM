use crate::{application, configuration, database::Database, serving};
use rom::{
    Clock, Command, Error, Resource, SystemClock, WorkClaim, WorkOutcome, WorkResult, WorkUpdate,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Serialize, Deserialize)]
struct Snapshot {
    cursor: rom::JournalCursor,
    original_claim: WorkClaim,
    work: Vec<rom::WorkRecord>,
    last_snapshot_mutation: String,
}
pub fn write(path: &std::path::Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > 1024 * 1024 {
        return Err("evidence byte bound".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
fn require(value: bool, reason: &str) -> Result<()> {
    if value { Ok(()) } else { Err(reason.into()) }
}
pub fn blobs(runtime: rom::Runtime, root: &std::path::Path) -> Result<rom_blob::BlobService> {
    let adapter = rom_blob_object_store::Adapter::trusted_folder(root, 65_536)?;
    Ok(rom_blob::BlobService::builder(runtime)
        .store("attachments", std::sync::Arc::new(adapter))
        .build()?)
}
fn content(i: usize) -> Vec<u8> {
    format!("retained-attachment-{i:02}\n")
        .repeat(128)
        .into_bytes()
}
fn unix_milliseconds() -> Result<u128> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis())
}
async fn prepare(c: &configuration::Configuration) -> Result<()> {
    fs::create_dir(c.directory.join("objects"))?;
    fs::write(
        c.directory.join("source-only-canary"),
        b"source must not be available to restore",
    )?;
    let db = Database::open(&c.adapter, &c.directory.join("database"))?;
    let runtime = application::runtime(db.storage())?;
    application::seed_identity(&runtime, c).await?;
    for i in 0..1000 {
        runtime
            .execute(&application::actor(), application::record(i))
            .await?;
    }
    for i in 0..100 {
        runtime
            .execute(
                &application::actor(),
                Command::<application::Record>::delete(&format!("record-{i:04}"))
                    .at_revision(1)
                    .idempotency(&format!("r8-delete-{i:04}")),
            )
            .await?;
    }
    let blob = blobs(runtime.clone(), &c.directory.join("objects"))?;
    for i in 0..20 {
        let id = format!("attachment-{i:02}");
        let bytes = content(i);
        blob.reserve(
            &rom_demo::session_actor(),
            &id,
            "attachments",
            rom_blob::Digest::of(&bytes),
            bytes.len() as u64,
            &format!("r8-blob-{i:02}"),
        )
        .await?;
        require(
            matches!(
                blob.upload(
                    &rom_demo::session_actor(),
                    &id,
                    Box::pin(futures_util::stream::iter([Ok(bytes)]))
                )
                .await?,
                rom_blob::UploadOutcome::Attached(_)
            ),
            "attachment publication",
        )?;
    }
    blob.shutdown().await?;
    drop(blob);
    rom_demo::reference::prepare(&runtime).await?;
    let storage = db.storage();
    let WorkResult::Claimed(claim) = storage.reaction_update(WorkUpdate::Claim {
        now: SystemClock.now(),
    })?
    else {
        return Err("known pending work missing".into());
    };
    let state = Snapshot {
        cursor: storage.journal_head(application::Record::KIND)?,
        original_claim: *claim,
        work: storage.reaction_records()?,
        last_snapshot_mutation: "reference-rejection".into(),
    };
    // The application owns no active worker after this awaited drain boundary.
    runtime.shutdown().await?;
    drop(runtime);
    write(&c.backup_directory.join("application-state.json"), &state)?;
    db.backup(&c.backup_directory.join("database.archive"))?;
    write(
        &c.directory.join("prepared.json"),
        &rom::json!({"resources":1000,"tombstones":100,"attachments":20,"snapshot_finished_unix_milliseconds":unix_milliseconds()?}),
    )?;
    Ok(())
}
async fn post_snapshot(c: &configuration::Configuration) -> Result<()> {
    let db = Database::open(&c.adapter, &c.directory.join("database"))?;
    let runtime = application::runtime(db.storage())?;
    for i in 1000..1020 {
        runtime
            .execute(&application::actor(), application::record(i))
            .await?;
    }
    runtime.shutdown().await?;
    write(
        &c.directory.join("post-snapshot.json"),
        &rom::json!({"acknowledged":20,"last_identity":"r8-record-1019","last_acknowledged_unix_milliseconds":unix_milliseconds()?}),
    )?;
    Ok(())
}
async fn restore(c: &configuration::Configuration) -> Result<()> {
    let started = Instant::now();
    require(
        fs::read(c.source_directory.join("source-only-canary")).is_err(),
        "source canary remains available",
    )?;
    require(
        !c.directory.join("database").exists(),
        "restore destination already populated",
    )?;
    let state: Snapshot = serde_json::from_slice(&fs::read(
        c.backup_directory.join("application-state.json"),
    )?)?;
    let db = Database::restore(
        &c.adapter,
        &c.backup_directory.join("database.archive"),
        &c.directory.join("database"),
    )?;
    let storage = db.storage();
    let runtime = application::runtime(storage.clone())?;
    require(
        matches!(
            storage.journal(application::Record::KIND, Some(&state.cursor), 10, 65_536),
            Err(Error::HistoryGap)
        ),
        "old journal cursor was not fenced",
    )?;
    require(
        storage.reaction_update(WorkUpdate::Finish {
            claim: state.original_claim.key(),
            now: SystemClock.now(),
            outcome: WorkOutcome::Done,
        }) == Err(Error::Conflict),
        "old work claim was not fenced",
    )?;
    let old_ids: Vec<_> = state
        .work
        .iter()
        .map(|work| work.pending.id.clone())
        .collect();
    require(
        old_ids
            == storage
                .reaction_records()?
                .iter()
                .map(|work| work.pending.id.clone())
                .collect::<Vec<_>>(),
        "restored work identities changed",
    )?;
    for i in 0..100 {
        require(
            storage
                .load(&rom::Key {
                    kind: application::Record::KIND.into(),
                    id: format!("record-{i:04}"),
                })?
                .is_some_and(|row| row.value.is_none() && row.revision == 2),
            "tombstone restore",
        )?;
    }
    for i in 100..1000 {
        require(
            runtime
                .read::<application::Record>(&application::actor(), &format!("record-{i:04}"))
                .await?
                .value
                .is_some(),
            "snapshot row restore",
        )?;
    }
    let events_before = storage.journal_head(application::Record::KIND)?;
    for i in 100..200 {
        runtime
            .execute(&application::actor(), application::record(i))
            .await?;
    }
    require(
        storage.journal_head(application::Record::KIND)? == events_before,
        "receipt replay created new journal events",
    )?;
    for i in 1000..1020 {
        require(
            storage
                .load(&rom::Key {
                    kind: application::Record::KIND.into(),
                    id: format!("record-{i:04}"),
                })?
                .is_none(),
            "post-snapshot data survived backup boundary",
        )?;
    }
    let blob = blobs(runtime.clone(), &c.directory.join("objects"))?;
    for i in 0..20 {
        require(
            blob.read(&rom_demo::session_actor(), &format!("attachment-{i:02}"))
                .await?
                == content(i),
            "restored attachment bytes",
        )?;
    }
    rom_demo::reference::recover(&runtime).await?;
    let after = storage.reaction_records()?;
    require(
        old_ids
            .iter()
            .all(|id| after.iter().any(|work| work.pending.id == *id)),
        "work identity lost after completion",
    )?;
    require(
        after.iter().any(|work| {
            work.pending.id == state.original_claim.work.pending.id
                && work.state == rom::WorkState::Done
                && work.generation > state.original_claim.work.generation
        }),
        "original incomplete work not recovered",
    )?;
    blob.shutdown().await?;
    runtime.shutdown().await?;
    write(
        &c.directory.join("restored.json"),
        &rom::json!({"resources":1000,"tombstones":100,"verified_receipt_replays":100,"attachments":20,"post_snapshot_acknowledged_lost":20,"database_restore_and_checks_milliseconds":started.elapsed().as_millis(),"source_canary_unavailable":true,"original_work_id":state.original_claim.work.pending.id,"original_work_done":true}),
    )?;
    Ok(())
}
pub async fn run() -> Result<()> {
    let configuration = configuration::read()?;
    match configuration.mode.as_str() {
        "prepare" => prepare(&configuration).await,
        "post-snapshot" => post_snapshot(&configuration).await,
        "restore" => restore(&configuration).await,
        "serve" => serving::run(configuration).await,
        _ => Err("unknown fixture mode".into()),
    }
}
