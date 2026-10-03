//! Real public Runtime commit workloads, reusable without query instrumentation.
use crate::fixture::{self, Dataset, MeasurementRow};
use rom::{Command, Patch, Result, Runtime};
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct WriteMetrics {
    pub phase: &'static str,
    pub commits: usize,
    pub elapsed_ns: u128,
    pub kernel_storage_write_bytes: Option<u64>,
    pub kernel_cancelled_write_bytes: Option<u64>,
}

fn io_counters() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/self/io").ok()?;
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.trim().parse::<u64>().ok())
    };
    Some((value("write_bytes:")?, value("cancelled_write_bytes:")?))
}
fn metrics(
    phase: &'static str,
    commits: usize,
    elapsed_ns: u128,
    before: Option<(u64, u64)>,
) -> WriteMetrics {
    let delta = before.zip(io_counters()).and_then(|(before, after)| {
        Some((
            after.0.checked_sub(before.0)?,
            after.1.checked_sub(before.1)?,
        ))
    });
    WriteMetrics {
        phase,
        commits,
        elapsed_ns,
        kernel_storage_write_bytes: delta.map(|v| v.0),
        kernel_cancelled_write_bytes: delta.map(|v| v.1),
    }
}

pub async fn seed(runtime: &Runtime, dataset: Dataset, size: usize) -> Result<WriteMetrics> {
    let actor = fixture::actor();
    let before = io_counters();
    let start = Instant::now();
    for i in 0..size {
        runtime
            .execute(
                &actor,
                Command::create(&fixture::id(i), fixture::value(dataset, i, size))
                    .idempotency(&format!("seed-{i}")),
            )
            .await?;
    }
    Ok(metrics("seed", size, start.elapsed().as_nanos(), before))
}

pub async fn update_batch(runtime: &Runtime, size: usize) -> Result<[WriteMetrics; 2]> {
    let commits = size.min(64);
    let actor = fixture::actor();
    let before = io_counters();
    let start = Instant::now();
    for i in 0..commits {
        runtime
            .execute(
                &actor,
                Command::patch(
                    &fixture::id(i),
                    Patch::new().set(MeasurementRow::amount_field(), (size + i) as u64),
                )
                .at_revision(1)
                .idempotency(&format!("indexed-update-{i}")),
            )
            .await?;
    }
    let indexed = metrics(
        "indexed_update",
        commits,
        start.elapsed().as_nanos(),
        before,
    );
    let before = io_counters();
    let start = Instant::now();
    for i in 0..commits {
        runtime
            .execute(
                &actor,
                Command::patch(
                    &fixture::id(i),
                    Patch::new().set(MeasurementRow::payload_field(), vec![u64::MAX, i as u64]),
                )
                .at_revision(2)
                .idempotency(&format!("payload-update-{i}")),
            )
            .await?;
    }
    Ok([
        indexed,
        metrics(
            "payload_update",
            commits,
            start.elapsed().as_nanos(),
            before,
        ),
    ])
}
