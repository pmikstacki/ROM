//! Finite load fixture preparation and Host lifecycle. No production deployment changes.
use crate::{
    ObservedStorage, application, configuration, database::Database, fixture_identity,
    host_configuration,
};
use rom::{Resource, Storage, WorkState};
use serde::Serialize;
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    sync::{Arc, atomic::AtomicUsize},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Serialize)]
struct SeedProof {
    resources: usize,
    candidate_bytes: usize,
    seed_work: usize,
    done_work: usize,
    work_bytes: usize,
    drain_ms: u128,
    elapsed_ms: u128,
}
fn write(directory: &std::path::Path, name: &str, value: &impl Serialize) -> Result<()> {
    write_bounded(directory, name, value, 256 * 1024)
}
pub(crate) fn write_bounded(
    directory: &std::path::Path,
    name: &str,
    value: &impl Serialize,
    maximum: usize,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > maximum {
        return Err("finite load evidence bytes".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join(name))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
fn progress(stage: &str, rows: usize, elapsed: Duration, phase: Duration) {
    println!(
        "{}",
        serde_json::json!({"stage":stage,"completed_rows":rows,"elapsed_ms":elapsed.as_millis(),"phase_ms":phase.as_millis()})
    );
}
async fn prepare(c: &configuration::Configuration) -> Result<()> {
    std::fs::create_dir(c.directory.join("objects"))?;
    let db = Database::open(&c.adapter, &c.directory.join("database"))?;
    #[cfg(feature = "storage-stage-timings")]
    let stages = crate::stage_measurements::enable(&db)?;
    let storage = Arc::new(ObservedStorage::new(db.storage()));
    let runtime = application::runtime(storage.clone(), Arc::new(AtomicUsize::new(0)))?;
    fixture_identity::seed_identity(&runtime, c).await?;
    let baseline = storage.calls();
    #[cfg(feature = "storage-stage-timings")]
    let stage_baseline = crate::stage_measurements::summary(stages.snapshot());
    #[cfg(feature = "storage-stage-timings")]
    let publication_baseline = stages.publication_snapshot();
    #[cfg(feature = "storage-stage-timings")]
    let mut publication_batches = Vec::with_capacity(100);
    let mut batches = Vec::with_capacity(100);
    let result = prepare_workload(
        c,
        &runtime,
        &storage,
        &mut batches,
        #[cfg(feature = "storage-stage-timings")]
        &stages,
        #[cfg(feature = "storage-stage-timings")]
        &mut publication_batches,
    )
    .await;
    let proof = write_bounded(
        &c.directory,
        "load-storage-calls.json",
        &serde_json::json!({"semantics":"synchronous-storage-boundary-wall-time",
            "batch_operations":crate::seed_measurements::SELECTED,
            "baseline":baseline,"batches":batches,"final":storage.calls(),
            "live_encoded_state_bytes_measured":false}),
        65536,
    );
    #[cfg(feature = "storage-stage-timings")]
    let proof = proof.and(write_bounded(
        &c.directory,
        "load-storage-stages.json",
        &serde_json::json!({
            "semantics": "fixed-category-adapter-stage-wall-time",
            "acceptance": false,
            "observer_enabled": true,
            "exclusive_cpu_time": false,
            "boundary_totals_include_these_stages": true,
            "stage_scope": {
                "metadata_read": "StorageMetadata decoding only; Work codecs occur in shared_prepare and native_publication",
                "shared_prepare": "receipt, revision and reference reads, core checks and Work preparation",
                "native_publication": "serialization, native writes, read-fact rechecks and pre-commit checkpoints",
                "native_commit": "native transaction commit; post-commit acknowledgement checkpoint is outside",
            },
            "baseline": stage_baseline,
            "final": crate::stage_measurements::summary(stages.snapshot()),
        }),
        65536,
    ));
    #[cfg(feature = "storage-stage-timings")]
    let proof = proof.and(write_bounded(
        &c.directory,
        "load-storage-publication.json",
        &crate::publication_measurements::proof(
            publication_baseline,
            &publication_batches,
            stages.publication_snapshot(),
        ),
        65536,
    ));
    match (result, proof) {
        (Err(error), metric) => {
            if metric.is_err() {
                eprintln!("storage call evidence unavailable");
            }
            Err(error)
        }
        (Ok(()), proof) => proof,
    }
}
async fn prepare_workload(
    c: &configuration::Configuration,
    runtime: &rom::Runtime,
    storage: &ObservedStorage,
    batches: &mut Vec<crate::seed_measurements::Batch>,
    #[cfg(feature = "storage-stage-timings")] stages: &rom_sqlite::StageObservation,
    #[cfg(feature = "storage-stage-timings")] publication_batches: &mut Vec<
        crate::publication_measurements::Batch,
    >,
) -> Result<()> {
    let started = Instant::now();
    let mut drain = Duration::ZERO;
    for batch in 0..100 {
        let creating = Instant::now();
        if started.elapsed() > Duration::from_secs(120) {
            return Err("finite seed admission deadline".into());
        }
        for index in batch * 100..(batch + 1) * 100 {
            if started.elapsed() > Duration::from_secs(120) {
                return Err("finite seed create deadline".into());
            }
            runtime
                .execute(&rom_demo::bootstrap_actor(), application::seeded(index))
                .await?;
            if (index + 1).is_multiple_of(50) {
                progress("create", index + 1, started.elapsed(), creating.elapsed());
            }
        }
        let draining = Instant::now();
        loop {
            if started.elapsed() > Duration::from_secs(120) {
                return Err("finite seed drain deadline".into());
            }
            if runtime.process_reactions(32).await? == 0 {
                break;
            }
        }
        drain += draining.elapsed();
        progress(
            "drain",
            (batch + 1) * 100,
            started.elapsed(),
            draining.elapsed(),
        );
        crate::seed_measurements::record(batches, (batch + 1) * 100, started.elapsed(), storage)?;
        #[cfg(feature = "storage-stage-timings")]
        crate::publication_measurements::record(
            publication_batches,
            (batch + 1) * 100,
            stages.publication_snapshot(),
        )?;
    }
    let candidate = storage.snapshot(application::LoadRecord::KIND, 12000, 32 * 1024 * 1024)?;
    let ledger = storage.work_snapshot(12000, 16 * 1024 * 1024)?;
    let proof = SeedProof {
        resources: candidate.len(),
        candidate_bytes: serde_json::to_vec(&candidate)?.len(),
        seed_work: ledger.records.len(),
        done_work: ledger
            .records
            .iter()
            .filter(|r| r.state == WorkState::Done)
            .count(),
        work_bytes: serde_json::to_vec(&ledger)?.len(),
        drain_ms: drain.as_millis(),
        elapsed_ms: started.elapsed().as_millis(),
    };
    if proof.resources != 10000 || proof.seed_work != 10000 || proof.done_work != 10000 {
        return Err("exact seed reaction profile".into());
    }
    write(&c.directory, "load-seed-proof.json", &proof)?;
    runtime.shutdown().await?;
    println!("{{\"status\":\"seeded\",\"resources\":10000,\"work\":10000}}");
    Ok(())
}
pub(crate) async fn serve(c: configuration::Configuration, long: bool) -> Result<()> {
    // Listener admission remains outside the shared lifecycle body.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:44391").await?;
    serve_with_listener(c, long, listener).await
}
pub(crate) async fn serve_with_listener(
    c: configuration::Configuration,
    long: bool,
    listener: tokio::net::TcpListener,
) -> Result<()> {
    let db = Database::open(&c.adapter, &c.directory.join("database"))?;
    #[cfg(feature = "storage-stage-timings")]
    let stage_observation = crate::stage_measurements::LifecycleObservation::begin(&db)?;
    let observed = Arc::new(ObservedStorage::new(db.storage()));
    #[cfg(feature = "storage-stage-timings")]
    let work_observation =
        crate::work_terminal_proof::WorkTerminalObservation::begin(&db, &observed)?;
    #[cfg(feature = "storage-stage-timings")]
    let claim_prefix_observation =
        crate::claim_prefix_terminal_proof::ClaimPrefixTerminalObservation::begin(&db)?;
    let runtime = application::runtime(observed.clone(), Arc::new(AtomicUsize::new(0)))?;
    let core_overload_baseline = runtime.core_overload_stats();
    let store =
        rom_blob_object_store::Adapter::trusted_folder(&c.directory.join("objects"), 65536)?;
    let blobs = rom_blob::BlobService::builder(runtime.clone())
        .store("attachments", Arc::new(store))
        .build()?;
    let mut config = host_configuration::configured(
        &c.issuer,
        &c.client_id,
        &c.client_secret,
        rom_demo::bootstrap_actor(),
        blobs,
    );
    config.limits.sessions = 32;
    config.limits.authentication_jobs = 8;
    // Reference comparison: match 32 active groups in both HTTP body pools.
    config.http_limits.bodies = 32;
    let host =
        rom_studio_host::StudioHost::new(runtime.clone(), config.authentication_diagnostics(true))?;
    let authentication_observer = host.clone();
    let worker = runtime.start_reactions()?;
    println!(
        "{{\"status\":\"ready\",\"candidate_rows\":12000,\"candidate_bytes\":33554432,\"work_records\":12000,\"work_bytes\":16777216}}"
    );
    let stop = c.stop_file.clone();
    let limit = if long { 960 } else { 240 };
    let host_result = host
        .serve(listener, async move {
            let end = Instant::now() + Duration::from_secs(limit);
            while Instant::now() < end && !stop.exists() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
    let worker_result = worker.join().await;
    // These results exist before any fallible evidence capture. Never short-circuit auth.
    // Successful Host::serve includes HTTP producer completion and its shutdown path;
    // worker.join includes reaction-loop completion. Error paths attest conservatively false.
    let producers_stopped = host_result.is_ok() && worker_result.is_ok();
    let lifecycle_result = host_result.and(worker_result);
    #[cfg(feature = "storage-stage-timings")]
    let stage_proof =
        stage_observation.finish(&c.directory, &runtime, &lifecycle_result, producers_stopped);
    #[cfg(feature = "storage-stage-timings")]
    let work_proof = work_observation.finish(
        &c.directory,
        &observed,
        &runtime,
        &lifecycle_result,
        producers_stopped,
    );
    #[cfg(feature = "storage-stage-timings")]
    let claim_prefix_proof = claim_prefix_observation.finish(
        &c.directory,
        &runtime,
        &lifecycle_result,
        producers_stopped,
    );
    let core_proof = crate::core_overload_evidence::finish(
        &c.directory,
        &runtime,
        core_overload_baseline,
        &lifecycle_result,
        producers_stopped,
    );
    let authentication_result = crate::authentication_evidence::finish(
        &c.directory,
        &authentication_observer,
        lifecycle_result,
    );
    // Authentication preserves the original lifecycle error; all proof attempts
    // above have already run before any evidence error can short-circuit.
    let proof_result = authentication_result.and(core_proof);
    #[cfg(feature = "storage-stage-timings")]
    let proof_result = proof_result.and(stage_proof);
    #[cfg(feature = "storage-stage-timings")]
    let proof_result = proof_result.and(work_proof);
    #[cfg(feature = "storage-stage-timings")]
    let proof_result = proof_result.and(claim_prefix_proof);
    proof_result?;
    if let Some((samples, missing, rejected)) = observed.timings() {
        write(
            &c.directory,
            "queue-confirmation-proof.json",
            &serde_json::json!({"semantics":"fixture-commit-confirmation-to-claim-confirmation","restart_durable":false,"nanosecond_samples":samples,"missing":missing,"rejected":rejected}),
        )?;
    }
    let ledger = observed.work_snapshot(12000, 16 * 1024 * 1024)?;
    write(
        &c.directory,
        "load-final-work-proof.json",
        &serde_json::json!({"records":ledger.records.len(),"bytes":serde_json::to_vec(&ledger)?.len(),"done":ledger.records.iter().filter(|r|r.state==WorkState::Done).count()}),
    )?;
    write_bounded(
        &c.directory,
        "load-final-storage-calls.json",
        &serde_json::json!({
            "schema": "rom-application-load-storage-calls-v1",
            "scope": "all observed adapter calls during host lifecycle; not per-request attribution",
            "calls": observed.calls(),
        }),
        16 * 1024,
    )?;
    Ok(())
}
pub async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    #[cfg(feature = "storage-stage-timings")]
    if args
        .first()
        .is_some_and(|argument| argument == crate::linked_engine_diagnostic::FLAG)
    {
        return crate::linked_engine_diagnostic::run(&args);
    }
    if !(args.len() == 1 || (args.len() == 2 && args[1] == "--load-window")) {
        return Err("closed load arguments".into());
    }
    let c = configuration::read()?;
    match c.mode.as_str() {
        "prepare" => prepare(&c).await,
        "post-snapshot" => crate::inspection::record(&c),
        "serve" => serve(c, args.len() == 2).await,
        _ => Err("closed load mode".into()),
    }
}
