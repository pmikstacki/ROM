//! End-to-end maintained Runtime query trials with untimed equivalence checks.
pub use crate::settings::Settings;
use crate::{
    Failure, cases,
    fixture::{self, Dataset, Distribution, MeasurementRow},
    memory,
    observed::{Mode, Observed},
    space, write,
};
use rom::{Limits, ProjectedView, QuerySpec, QueryStrategy, Resource, Runtime, json};
use std::{io::Write, sync::Arc, time::Instant};

pub async fn main() -> Result<(), Failure> {
    run(&Settings::arguments()?, &mut std::io::stdout()).await
}

pub async fn run(settings: &Settings, output: &mut impl Write) -> Result<(), Failure> {
    settings.validate()?;
    std::fs::create_dir(&settings.directory)?;
    writeln!(
        output,
        "{}",
        json!({"record":"run","label":settings.label,"sizes":settings.sizes,"repetitions":settings.repetitions,"warmups":settings.warmups,"seed":settings.seed,"heap_instrumented":cfg!(feature="heap"),"debug_assertions":cfg!(debug_assertions),"amount_order":"amount equals ascending Resource ID rank","rss_scope":"process lifetime high-water includes setup and earlier datasets","heap_scope":"Rust allocations begun during query; result retained; excludes pre-existing and SQLite C allocations","vm_scope":"materialization statement only; excludes metadata and planning","elapsed_scope":"actor construction, QuerySpec clone and public Runtime query through final disclosure; excludes result comparison, formatting and destruction","storage_elapsed_scope":"adapter query_read_observed including metadata and planning","mode_order":"all six permutations, shifted by case and distribution","production_control_order":"separate warmed automatic pass after each case's observed trials; no Observed wrapper","write_phases":["seed","indexed_update","payload_update"],"directory":settings.directory})
    )?;
    let pool = Runtime::shared_cpu_pool(2)?;
    for (distribution_index, distribution) in [Distribution::Independent, Distribution::Skewed]
        .into_iter()
        .enumerate()
    {
        for &size in &settings.sizes {
            let dataset = Dataset {
                distribution,
                seed: settings.seed,
            };
            let path = settings
                .directory
                .join(format!("{distribution_index}-{size}.sqlite"));
            let store = Arc::new(rom_sqlite::Sqlite::open_with_limits(
                &path,
                fixture::storage_limits(size),
            )?);
            let observed = Arc::new(Observed::new(store.clone()));
            let limits = Limits {
                snapshot_rows: size,
                snapshot_bytes: size.checked_mul(2048).ok_or("byte bound overflow")?,
                ..Limits::default()
            };
            let build_runtime = |storage: Arc<dyn rom::Storage>| {
                Runtime::builder()
                    .limits(limits)
                    .resource(fixture::definition())
                    .build(storage, pool.clone())
            };
            let mut runtime = build_runtime(observed.clone())?;
            let seeded = write::seed(&runtime, dataset, size).await?;
            if !settings.heap {
                writeln!(
                    output,
                    "{}",
                    json!({"record":"write","dataset":dataset,"size":size,"metrics":seeded,"heap_instrumented":false})
                )?;
            }
            writeln!(
                output,
                "{}",
                json!({"record":"space","phase":"after_seed_open","dataset":dataset,"size":size,"space":space::capture(&path)?})
            )?;
            let selected: Vec<_> = cases::cases(dataset, size)
                .into_iter()
                .filter(|case| settings.case.as_ref().is_none_or(|name| name == case.name))
                .collect();
            if selected.is_empty() {
                return Err("unknown case name".into());
            }
            for (case_index, case) in selected.into_iter().enumerate() {
                let expected =
                    query(&runtime, &observed, Mode::Reference, case.query.clone()).await?;
                let match_count = if case.query.limit.is_some() {
                    let mut full = case.query.clone();
                    full.limit = None;
                    query(&runtime, &observed, Mode::Reference, full)
                        .await?
                        .len()
                } else {
                    expected.len()
                };
                for _ in 0..settings.warmups {
                    for mode in Mode::ALL {
                        ensure_equal(
                            &expected,
                            &query(&runtime, &observed, mode, case.query.clone()).await?,
                        )?;
                    }
                }
                for repetition in 0..settings.repetitions {
                    let order = Mode::order(repetition + case_index + distribution_index);
                    for (offset, mode) in order.into_iter().enumerate() {
                        observed.mode(mode)?;
                        let profile = memory::Profile::begin(settings.heap);
                        let start = Instant::now();
                        let result = runtime
                            .query_spec_projected(
                                &fixture::actor(),
                                MeasurementRow::KIND,
                                case.query.clone(),
                            )
                            .await?;
                        let elapsed = start.elapsed().as_nanos();
                        // Read heap counters while the profile and query result are alive.
                        // No comparison, serialization, RSS read or report formatting is in scope.
                        let heap = profile.finish();
                        let metrics = observed.take()?;
                        ensure_equal(&expected, &result)?;
                        let common = json!({"record":if settings.heap {"heap"}else{"query"},"dataset":dataset,"size":size,"case":case.name,"repetition":repetition,"order_offset":offset,"mode":mode,"actual_strategy":match metrics.adapter.strategy {QueryStrategy::Reference=>"reference",QueryStrategy::NativeCandidates=>"native_candidates"},"decoded_rows":metrics.adapter.decoded_rows,"decoded_bytes":metrics.adapter.decoded_bytes,"materialization_vm_steps":metrics.adapter.vm_steps,"match_count":match_count,"result_count":result.len(),"result_digest":digest(&result)?});
                        let mut record = common;
                        record["probe_rows"] = json!(metrics.adapter.probe_rows);
                        record["probe_statements"] = json!(metrics.adapter.probe_statements);
                        record["probe_vm_steps"] = json!(metrics.adapter.probe_vm_steps);
                        if settings.heap {
                            record["heap"] = serde_json::to_value(heap)?;
                        } else {
                            record["elapsed_ns"] = json!(elapsed);
                            record["storage_elapsed_ns"] = json!(metrics.storage_elapsed_ns);
                            record["probe_elapsed_ns"] = json!(metrics.adapter.probe_elapsed_ns);
                        }
                        writeln!(output, "{record}")?;
                    }
                }
                if !settings.heap {
                    // The control uses the same dataset after the observed owner releases it.
                    runtime.shutdown().await?;
                    drop(runtime);
                    let production = build_runtime(store.clone())?;
                    for _ in 0..settings.warmups {
                        ensure_equal(
                            &expected,
                            &production
                                .query_spec_projected(
                                    &fixture::actor(),
                                    MeasurementRow::KIND,
                                    case.query.clone(),
                                )
                                .await?,
                        )?;
                    }
                    for repetition in 0..settings.repetitions {
                        let start = Instant::now();
                        let result = production
                            .query_spec_projected(
                                &fixture::actor(),
                                MeasurementRow::KIND,
                                case.query.clone(),
                            )
                            .await?;
                        let elapsed = start.elapsed().as_nanos();
                        ensure_equal(&expected, &result)?;
                        writeln!(
                            output,
                            "{}",
                            json!({"record":"production_query","dataset":dataset,"size":size,"case":case.name,"repetition":repetition,"mode":"automatic","elapsed_ns":elapsed,"match_count":match_count,"result_count":result.len(),"result_digest":digest(&result)?})
                        )?;
                    }
                    production.shutdown().await?;
                    drop(production);
                    runtime = build_runtime(observed.clone())?;
                }
            }
            writeln!(
                output,
                "{}",
                json!({"record":"rss","phase":"after_warm_queries","dataset":dataset,"size":size,"rss":space::rss()})
            )?;
            if !settings.heap {
                for metrics in write::update_batch(&runtime, size).await? {
                    writeln!(
                        output,
                        "{}",
                        json!({"record":"write","dataset":dataset,"size":size,"metrics":metrics})
                    )?;
                }
                writeln!(
                    output,
                    "{}",
                    json!({"record":"space","phase":"after_updates_open","dataset":dataset,"size":size,"space":space::capture(&path)?})
                )?;
            }
            runtime.shutdown().await?;
            drop(runtime);
            drop(observed);
            drop(store);
            writeln!(
                output,
                "{}",
                json!({"record":"files","phase":"closed","dataset":dataset,"size":size,"files":space::files(&path)})
            )?;
        }
    }
    Ok(())
}

async fn query(
    runtime: &Runtime,
    observed: &Observed,
    mode: Mode,
    spec: QuerySpec,
) -> Result<Vec<ProjectedView>, Failure> {
    observed.mode(mode)?;
    let result = runtime
        .query_spec_projected(&fixture::actor(), MeasurementRow::KIND, spec)
        .await?;
    observed.take()?;
    Ok(result)
}
fn ensure_equal(expected: &[ProjectedView], actual: &[ProjectedView]) -> Result<(), Failure> {
    if expected != actual {
        return Err("query strategies returned different ordered keys, revisions or values".into());
    }
    Ok(())
}
fn digest(rows: &[ProjectedView]) -> Result<String, Failure> {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in serde_json::to_vec(rows)? {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}
