use crate::fixture::{self, Dataset, Distribution};
use rom::{Resource, Runtime};

#[test]
fn fixture_is_deterministic_and_skew_is_independent_of_amount() {
    let uniform = Dataset {
        distribution: Distribution::Independent,
        seed: 11,
    };
    let skew = Dataset {
        distribution: Distribution::Skewed,
        seed: 11,
    };
    let count = 1024;
    let rows: Vec<_> = (0..count)
        .map(|i| fixture::value(uniform, i, count))
        .collect();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.encode(), fixture::value(uniform, i, count).encode());
        assert_eq!(row.amount, i as u64);
    }
    let hot = (0..count)
        .filter(|i| fixture::value(skew, *i, count).category == "hot")
        .count();
    assert!((850..=980).contains(&hot));
    assert!((0..count).any(|i| fixture::value(skew, i, count).category != "hot"));
    assert!(rows.iter().any(|row| row.active));
    assert!(rows.iter().any(|row| !row.active));
    assert_ne!(
        fixture::value(uniform, 1, count).encode(),
        fixture::value(
            Dataset {
                seed: 12,
                ..uniform
            },
            1,
            count
        )
        .encode()
    );
    assert_eq!(fixture::storage_limits(count).journal_rows, 32);
    for start in (0..count).step_by(256) {
        let hot = (start..start + 256)
            .filter(|i| fixture::value(skew, *i, count).category == "hot")
            .count();
        assert!((195..=250).contains(&hot));
        for active in [false, true] {
            assert!((start..start + 256).any(|i| {
                let row = fixture::value(uniform, i, count);
                row.active == active && row.category == "category-00"
            }));
        }
    }
}

#[test]
fn heldout_seed_does_not_repeat_shifted_calibration_random_fields() {
    let calibration = Dataset {
        distribution: Distribution::Independent,
        seed: 11,
    };
    let heldout = Dataset {
        seed: 12,
        ..calibration
    };
    let changed = (0..1000)
        .filter(|i| {
            let a = fixture::value(calibration, i + 1, 1024);
            let b = fixture::value(heldout, *i, 1024);
            (a.category, a.active, a.title, a.payload) != (b.category, b.active, b.title, b.payload)
        })
        .count();
    assert!(
        changed > 900,
        "adjacent seeds repeat shifted random fields: {changed}"
    );
}

#[tokio::test]
async fn reusable_write_workload_commits_and_preserves_expected_revisions() {
    let path = std::env::temp_dir().join(format!("rom-query-write-smoke-{}", std::process::id()));
    let store = std::sync::Arc::new(
        rom_sqlite::Sqlite::open_with_limits(&path, fixture::storage_limits(8)).unwrap(),
    );
    let runtime = Runtime::builder()
        .resource(fixture::definition())
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let dataset = Dataset {
        distribution: Distribution::Independent,
        seed: 5,
    };
    assert_eq!(
        crate::write::seed(&runtime, dataset, 8)
            .await
            .unwrap()
            .commits,
        8
    );
    let updates = crate::write::update_batch(&runtime, 8).await.unwrap();
    assert_eq!(updates[0].commits, 8);
    assert_eq!(updates[1].commits, 8);
    let row = runtime
        .read::<fixture::MeasurementRow>(&fixture::actor(), &fixture::id(0))
        .await
        .unwrap();
    assert_eq!(row.revision, 3);
    assert_eq!(row.value.unwrap().payload, vec![u64::MAX, 0]);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[cfg(all(feature = "queries", not(feature = "heap")))]
#[tokio::test]
async fn smoke_compares_complete_runtime_results_and_emits_all_three_modes() {
    let path = std::env::temp_dir().join(format!("rom-query-read-smoke-{}", std::process::id()));
    let settings = crate::run::Settings::smoke(path.clone());
    let mut output = Vec::new();
    crate::run::run(&settings, &mut output).await.unwrap();
    let records: Vec<rom::Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let samples: Vec<_> = records.iter().filter(|v| v["record"] == "query").collect();
    assert_eq!(samples.len(), 48);
    for mode in ["automatic", "reference", "native"] {
        assert_eq!(samples.iter().filter(|v| v["mode"] == mode).count(), 16);
    }
    assert!(
        samples
            .iter()
            .all(|v| v["result_digest"].is_string() && v["storage_elapsed_ns"].is_number())
    );
    let controls: Vec<_> = records
        .iter()
        .filter(|v| v["record"] == "production_query")
        .collect();
    assert_eq!(controls.len(), 16);
    assert!(
        controls
            .iter()
            .all(|v| v["elapsed_ns"].is_number() && v.get("decoded_rows").is_none())
    );
    std::fs::remove_dir_all(path).unwrap();
}

#[cfg(feature = "queries")]
#[test]
fn trial_orders_cover_all_six_mode_permutations() {
    let orders: std::collections::BTreeSet<_> = (0..6)
        .map(|i| format!("{:?}", crate::observed::Mode::order(i)))
        .collect();
    assert_eq!(orders.len(), 6);
}

#[cfg(feature = "queries")]
#[test]
fn duplicate_dataset_sizes_cannot_reuse_a_measured_database() {
    let mut settings = crate::run::Settings::smoke("unused-validation-path".into());
    settings.heap = cfg!(feature = "heap");
    settings.sizes = vec![16, 16];
    assert!(settings.validate().is_err());
    settings.sizes = vec![16, 32];
    assert!(settings.validate().is_ok());
}
