//! Standalone write/space workload that also builds against the pre-index adapter.
use crate::{
    Failure,
    fixture::{self, Dataset, Distribution},
    space, write,
};
use rom::Runtime;
use std::{io::Write, path::Path, sync::Arc};

pub async fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        return Err("usage: rom-query-write-measure PATH independent|skewed SIZE [SEED]".into());
    }
    let distribution = match args[1].as_str() {
        "independent" => Distribution::Independent,
        "skewed" => Distribution::Skewed,
        _ => return Err("unknown distribution".into()),
    };
    let size: usize = args[2].parse()?;
    if size == 0 || size > 100_000 {
        return Err("size must be 1..=100000".into());
    }
    let seed = args.get(3).map(|v| v.parse()).transpose()?.unwrap_or(11);
    run(
        Path::new(&args[0]),
        Dataset { distribution, seed },
        size,
        &mut std::io::stdout(),
    )
    .await
}

pub async fn run(
    path: &Path,
    dataset: Dataset,
    size: usize,
    output: &mut impl Write,
) -> Result<(), Failure> {
    if cfg!(feature = "heap") {
        return Err("write measurements require a build without the heap feature".into());
    }
    if path.exists() {
        return Err("write measurement requires a fresh database path".into());
    }
    let limits = fixture::storage_limits(size);
    let store = Arc::new(rom_sqlite::Sqlite::open_with_limits(path, limits.clone())?);
    let runtime = Runtime::builder()
        .resource(fixture::definition())
        .build(store.clone(), Runtime::shared_cpu_pool(2)?)?;
    writeln!(
        output,
        "{}",
        rom::json!({"record":"write_run","dataset":dataset,"size":size,"storage_limits":limits,"debug_assertions":cfg!(debug_assertions),"heap_instrumented":cfg!(feature="heap"),"io_scope":"Linux process kernel-accounted storage writes; not physical SSD bytes"})
    )?;
    let seed = write::seed(&runtime, dataset, size).await?;
    writeln!(
        output,
        "{}",
        rom::json!({"record":"write","dataset":dataset,"size":size,"metrics":seed})
    )?;
    writeln!(
        output,
        "{}",
        rom::json!({"record":"space","phase":"after_seed_open","dataset":dataset,"size":size,"space":space::capture(path)?})
    )?;
    for metrics in write::update_batch(&runtime, size).await? {
        writeln!(
            output,
            "{}",
            rom::json!({"record":"write","dataset":dataset,"size":size,"metrics":metrics})
        )?;
    }
    writeln!(
        output,
        "{}",
        rom::json!({"record":"space","phase":"after_updates_open","dataset":dataset,"size":size,"space":space::capture(path)?})
    )?;
    runtime.shutdown().await?;
    drop(runtime);
    drop(store);
    writeln!(
        output,
        "{}",
        rom::json!({"record":"files","phase":"closed","dataset":dataset,"size":size,"files":space::files(path)})
    )?;
    Ok(())
}
