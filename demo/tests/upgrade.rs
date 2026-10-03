use std::{
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-upgrade-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn source_bytes(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .map(|path| {
        let bytes = std::fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect()
}
fn assert_source_unchanged(source: &Path, expected: &[(PathBuf, Vec<u8>)]) {
    let actual = source_bytes(source);
    assert_eq!(
        actual.len(),
        expected.len(),
        "source file inventory changed for {}",
        source.display()
    );
    for ((path, bytes), (old_path, old_bytes)) in actual.iter().zip(expected) {
        assert_eq!(path, old_path, "source file identity changed");
        let first_difference = bytes.iter().zip(old_bytes).position(|(a, b)| a != b);
        assert!(
            bytes == old_bytes,
            "offline source {} changed: length {} -> {}, first differing byte {:?}",
            path.display(),
            old_bytes.len(),
            bytes.len(),
            first_difference
        );
    }
}
fn wait(mut child: std::process::Child, backend: &str) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("upgrade preparation timed out for {backend}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn upgrade_command_runs_the_real_application_on_both_backends() {
    for backend in ["sqlite", "redb"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rom-demo"))
            .args(["upgrade", backend])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Upgrade recovery passed"));
    }
}
#[test]
fn process_exit_preserves_pending_compensation_through_migration_backup_and_restore() {
    for backend in ["sqlite", "redb"] {
        let scratch = Scratch::new(backend);
        let source = scratch.0.join("source");
        let objects = scratch.0.join("objects");
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "upgrade_process_exit_child",
                "--nocapture",
            ])
            .env("ROM_UPGRADE_CHILD_PATH", &source)
            .env("ROM_UPGRADE_CHILD_BACKEND", backend)
            .spawn()
            .unwrap();
        assert_eq!(
            wait(child, backend).code(),
            Some(86),
            "child must reach confirmed rejection commit"
        );
        let before = source_bytes(&source);
        if backend == "sqlite" {
            assert_eq!(
                before.len(),
                2,
                "process exit must leave committed WAL bytes"
            );
        }
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            rom_demo::upgrade::recover(
                backend == "redb",
                &source,
                &scratch.0.join("migrated"),
                &scratch.0.join("archive"),
                &scratch.0.join("restored"),
                &objects,
            )
            .await
            .unwrap();
        });
        assert_source_unchanged(&source, &before);
        assert!(scratch.0.join("migrated").exists());
        assert!(scratch.0.join("archive").exists());
        assert!(scratch.0.join("restored").exists());
        let (manifest, archived) = rom_backup::read(
            scratch.0.join("archive"),
            if backend == "redb" {
                rom_backup::Backend::Redb
            } else {
                rom_backup::Backend::Sqlite
            },
            rom_backup::BackupLimits::default(),
        )
        .unwrap();
        assert!(!manifest.external_blobs_included);
        assert!(
            archived
                .references
                .iter()
                .any(|edge| edge.source.kind == "checkouts"
                    && edge.source.id == "upgrade-pending-checkout"
                    && edge.target.kind == "reservation-stock"
                    && edge.target.id == "upgrade-empty-stock")
        );
        assert!(archived.state.work.records().iter().any(|record| record.state == rom::WorkState::Pending
            && matches!(&record.pending.payload, rom::WorkPayload::Source(row) if row.key.kind == "checkouts"
                && row.key.id == "checkout-a" && row.revision == 3
                && row.value.as_ref().is_some_and(|value| value["payment_outcome"] == "confirmed_rejected"))));
    }
}
#[test]
#[ignore = "actual process-exit fixture invoked by process_exit_preserves_pending_compensation_through_migration_backup_and_restore"]
fn upgrade_process_exit_child() {
    let database =
        PathBuf::from(std::env::var_os("ROM_UPGRADE_CHILD_PATH").expect("parent fixture"));
    let redb = std::env::var("ROM_UPGRADE_CHILD_BACKEND").unwrap() == "redb";
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let runtime = rom_demo::upgrade::legacy_runtime(redb, &database).unwrap();
        let blobs = rom_demo::attachments::open(
            runtime.clone(),
            &database.parent().unwrap().join("objects"),
        )
        .unwrap();
        rom_demo::attachments::attach(&blobs).await.unwrap();
        rom_demo::upgrade::prepare_scenario(&runtime).await.unwrap();
        // Deliberately bypass Drop, shutdown, worker drain and all finalizers.
        std::process::exit(86);
    });
}
#[test]
fn upgrade_rejects_invalid_arguments_without_creating_files() {
    for args in [
        ["upgrade", "unknown", ""],
        ["upgrade", "sqlite", "extra"],
        ["upgrade", "redb", "extra"],
    ] {
        let scratch = Scratch::new("args");
        let args: Vec<_> = args.into_iter().filter(|arg| !arg.is_empty()).collect();
        let output = Command::new(env!("CARGO_BIN_EXE_rom-demo"))
            .args(args)
            .current_dir(&scratch.0)
            .env("TMPDIR", &scratch.0)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(std::fs::read_dir(&scratch.0).unwrap().next().is_none());
    }
}

// Independent frozen descriptor makes negative plans use the real application's
// public migration boundary without exporting its private compatibility model.
#[derive(Clone, rom::Resource)]
#[resource(name = "checkouts")]
struct CheckoutV1 {
    stock_id: String,
    reservation_id: String,
    payment_outcome: String,
}
fn convert(value: CheckoutV1) -> rom::Result<rom_demo::compensation::Checkout> {
    Ok(rom_demo::compensation::Checkout {
        stock_id: rom::ResourceRef::new(value.stock_id)?,
        reservation_id: value.reservation_id,
        payment_outcome: value.payment_outcome,
    })
}
fn plan(
    convert: fn(CheckoutV1) -> rom::Result<rom_demo::compensation::Checkout>,
) -> rom_backup::MigrationPlan {
    rom_backup::MigrationPlan::new(vec![rom_backup::ResourceMigration::new(convert).unwrap()])
        .unwrap()
}
fn migrate(
    redb: bool,
    source: &Path,
    destination: &Path,
    plan: &rom_backup::MigrationPlan,
) -> rom::Result<()> {
    let limits = rom_backup::BackupLimits::default();
    if redb {
        drop(rom_redb::Redb::migrate_from(
            source,
            destination,
            plan,
            limits,
        )?);
    } else {
        drop(rom_sqlite::Sqlite::migrate_from(
            source,
            destination,
            plan,
            limits,
        )?);
    }
    Ok(())
}
#[test]
fn invalid_conversion_missing_target_and_unaccepted_work_never_publish() {
    for redb in [false, true] {
        let scratch = Scratch::new("invalid");
        let source = scratch.0.join("source");
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(rom_demo::upgrade::prepare(
                redb,
                &source,
                &scratch.0.join("objects"),
            ))
            .unwrap();
        let before = source_bytes(&source);
        let invalid = plan(|_| {
            Err(rom::Error::invalid(
                "checkouts",
                "deliberate invalid conversion",
            ))
        })
        .validate_work(|_| Ok(()));
        let missing_target = plan(|mut value| {
            value.stock_id = "absent-target".into();
            convert(value)
        })
        .validate_work(|_| Ok(()));
        let rejected_work = plan(convert)
            .validate_work(|_| Err(rom::Error::Unsupported("callback contract changed".into())));
        let absent_work_validator = plan(convert);
        for (name, plan) in [
            ("invalid", invalid),
            ("missing-target", missing_target),
            ("rejected-work", rejected_work),
            ("absent-validator", absent_work_validator),
        ] {
            let destination = scratch.0.join(name);
            assert!(
                migrate(redb, &source, &destination, &plan).is_err(),
                "{name}"
            );
            assert!(!destination.exists(), "{name} must not publish");
            assert_source_unchanged(&source, &before);
        }
        let occupied = scratch.0.join("occupied");
        std::fs::write(&occupied, b"preserve destination").unwrap();
        assert!(
            migrate(
                redb,
                &source,
                &occupied,
                &rom_demo::upgrade::migration_plan().unwrap()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&occupied).unwrap(), b"preserve destination");
        assert_source_unchanged(&source, &before);
        // A failed attempt does not poison a later valid fresh publication.
        migrate(
            redb,
            &source,
            &scratch.0.join("valid"),
            &rom_demo::upgrade::migration_plan().unwrap(),
        )
        .unwrap();
        assert_source_unchanged(&source, &before);
    }
}

fn open_storage(redb: bool, path: &Path) -> std::sync::Arc<dyn rom::Storage> {
    if redb {
        std::sync::Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        std::sync::Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn backup(redb: bool, database: &Path, archive: &Path) -> rom_backup::Snapshot {
    let limits = rom_backup::BackupLimits::default();
    let backend = if redb {
        rom_redb::Redb::open(database)
            .unwrap()
            .backup_to(archive, limits)
            .unwrap();
        rom_backup::Backend::Redb
    } else {
        rom_sqlite::Sqlite::open(database)
            .unwrap()
            .backup_to(archive, limits)
            .unwrap();
        rom_backup::Backend::Sqlite
    };
    rom_backup::read(archive, backend, limits).unwrap().1
}
fn old_create() -> rom::Invocation {
    rom::Invocation {
        retry_epoch: 0,
        kind: "checkouts".into(),
        id: "checkout-a".into(),
        expected: None,
        idempotency: "seed-checkouts-checkout-a".into(),
        operation: rom::Operation::Create(rom::json!({
            "stock_id":"workshop-stock", "reservation_id":"checkout-a", "payment_outcome":"pending"
        })),
    }
}
#[test]
fn migrated_receipt_requires_its_recorded_codec_and_current_authority() {
    use rom::Resource;
    for redb in [false, true] {
        let scratch = Scratch::new("codec");
        let source = scratch.0.join("source");
        let migrated = scratch.0.join("migrated");
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            rom_demo::upgrade::prepare(redb, &source, &scratch.0.join("objects"))
                .await
                .unwrap();
            migrate(
                redb,
                &source,
                &migrated,
                &rom_demo::upgrade::migration_plan().unwrap(),
            )
            .unwrap();
            let before = backup(redb, &migrated, &scratch.0.join("before"));
            let receipts: Vec<_> = before
                .receipts
                .iter()
                .filter(|receipt| {
                    receipt.row.key.kind == "checkouts" && receipt.row.key.id == "checkout-a"
                })
                .collect();
            assert!(!receipts.is_empty());
            assert!(
                receipts
                    .iter()
                    .all(|receipt| receipt.replay_version == Some(1))
            );
            let storage = open_storage(redb, &migrated);
            let without_codec = rom::Runtime::builder()
                .resource(
                    rom_demo::compensation::Stock::definition()
                        .policy(|_, _, _| true)
                        .allow_all_fields(),
                )
                .resource(
                    rom_demo::compensation::Checkout::definition()
                        .policy(|_, _, _| true)
                        .allow_all_fields(),
                )
                .build(storage.clone(), rom::Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            assert!(matches!(
                without_codec
                    .invoke(&rom_demo::bootstrap_actor(), old_create())
                    .await,
                Err(rom::Error::Unsupported(_))
            ));
            without_codec.shutdown().await.unwrap();
            drop(without_codec);
            // Revoke the original receipt owner's host grant. Keep its identity
            // and the current application's replay codec and Resource policies.
            let revoked = rom_demo::declarations(rom_demo::Notices::default())
                .unwrap()
                .actor_gate(std::sync::Arc::new(rom_identity::IdentityGate::default()))
                .build(storage.clone(), rom::Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            assert!(matches!(
                revoked
                    .invoke(&rom_demo::bootstrap_actor(), old_create())
                    .await,
                Err(rom::Error::Denied)
            ));
            revoked.shutdown().await.unwrap();
            drop(revoked);
            let runtime = rom_demo::build(storage.clone(), rom_demo::Notices::default()).unwrap();
            assert!(matches!(
                runtime
                    .invoke(
                        &rom::Actor::trusted("demo-host", "unprovisioned"),
                        old_create()
                    )
                    .await,
                Err(rom::Error::Denied)
            ));
            let replayed = runtime
                .invoke(&rom_demo::bootstrap_actor(), old_create())
                .await
                .unwrap();
            assert_eq!(replayed.revision, 1);
            runtime.shutdown().await.unwrap();
            drop(runtime);
            drop(storage);
            let after = backup(redb, &migrated, &scratch.0.join("after"));
            assert_eq!(before.rows, after.rows);
            assert_eq!(before.receipts, after.receipts);
            assert_eq!(before.events, after.events);
            assert_eq!(before.state.work, after.state.work);
            assert_eq!(
                serde_json::to_value(before.effects).unwrap(),
                serde_json::to_value(after.effects).unwrap()
            );
        });
    }
}
