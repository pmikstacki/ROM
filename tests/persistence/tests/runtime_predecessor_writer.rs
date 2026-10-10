//! Public Runtime replay of commands written by the accepted release executable.
#[path = "../support/runtime-predecessor-writer/src/model.rs"]
mod model;

use rom::*;
use rom_backup::{Backend, BackupLimits};
use std::{path::Path, sync::Arc};

fn open(backend: Backend, path: &Path) -> Arc<dyn Storage> {
    match backend {
        Backend::Sqlite => Arc::new(rom_sqlite::Sqlite::open(path).unwrap()),
        Backend::Redb => Arc::new(rom_redb::Redb::open(path).unwrap()),
    }
}

fn backup(backend: Backend, path: &Path, archive: &Path) -> rom_backup::Snapshot {
    match backend {
        Backend::Sqlite => rom_sqlite::Sqlite::open(path)
            .unwrap()
            .backup_to(archive, BackupLimits::default())
            .unwrap(),
        Backend::Redb => rom_redb::Redb::open(path)
            .unwrap()
            .backup_to(archive, BackupLimits::default())
            .unwrap(),
    };
    rom_backup::read(archive, backend, BackupLimits::default())
        .unwrap()
        .1
}

#[tokio::test]
#[ignore = "requires witnessed accepted 0.0.3 Runtime output; run with ROM_RUNTIME_PREDECESSOR_EVIDENCE"]
async fn actual_predecessor_runtime_custom_codec_receipts_replay_without_duplicate_effects() {
    let root = std::env::var_os("ROM_RUNTIME_PREDECESSOR_EVIDENCE")
        .expect("explicit historical Runtime evidence");
    for (name, backend) in [("sqlite", Backend::Sqlite), ("redb", Backend::Redb)] {
        let directory = Path::new(&root).join(name);
        let source = directory.join("source");
        let archive = directory.join("before.rombk");
        let source_bytes = std::fs::read(&source).unwrap();
        let archive_bytes = std::fs::read(&archive).unwrap();
        let summary: Value =
            serde_json::from_slice(&std::fs::read(directory.join("summary.json")).unwrap())
                .unwrap();
        assert_eq!(summary["writer_release"], "0.0.3");
        let original: rom_backup::Snapshot =
            serde_json::from_value(summary["snapshot"].clone()).unwrap();
        let destination = directory.join("upgraded");
        match backend {
            Backend::Sqlite => drop(
                rom_sqlite::Sqlite::upgrade_from(
                    &source,
                    &destination,
                    &[model::Ticket::descriptor()],
                    BackupLimits::default(),
                )
                .unwrap(),
            ),
            Backend::Redb => drop(
                rom_redb::Redb::upgrade_from(
                    &source,
                    &destination,
                    &[model::Ticket::descriptor()],
                    BackupLimits::default(),
                )
                .unwrap(),
            ),
        }
        let runtime = Runtime::builder()
            .resource(model::definition(true))
            .build(
                open(backend, &destination),
                Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        for (index, command) in model::commands().into_iter().enumerate() {
            let replay = runtime.execute(&model::actor(), command).await.unwrap();
            let value = replay.value.unwrap();
            assert_eq!(replay.id, summary["results"][index]["id"]);
            assert_eq!(
                json!(replay.revision),
                summary["results"][index]["revision"]
            );
            assert_eq!(value.encode(), summary["results"][index]["value"]);
            assert!(matches!(value.code.0.as_str(), "FIRST" | "SECOND"));
        }
        // A different request cannot obtain the historical command's receipt.
        assert!(matches!(
            runtime
                .execute(
                    &model::actor(),
                    Command::action("one", model::INCREMENT, 99)
                        .at_revision(2)
                        .idempotency("runtime-action"),
                )
                .await,
            Err(Error::IdentityMismatch)
        ));
        runtime.shutdown().await.unwrap();
        drop(runtime);

        // The exact original principal cannot disclose an old receipt after policy revocation.
        let denied = Runtime::builder()
            .resource(model::definition(false))
            .build(
                open(backend, &destination),
                Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        for command in model::commands() {
            assert!(matches!(
                denied.execute(&model::actor(), command).await,
                Err(Error::Denied)
            ));
        }
        denied.shutdown().await.unwrap();
        drop(denied);
        let after = backup(backend, &destination, &directory.join("after.rombk"));
        assert_eq!(after.rows, original.rows);
        assert_eq!(after.receipts, original.receipts);
        assert_eq!(after.events, original.events);
        assert_eq!(
            serde_json::to_value(after.effects).unwrap(),
            serde_json::to_value(original.effects).unwrap()
        );
        assert_eq!(after.descriptors, original.descriptors);
        assert_eq!(after.references, original.references);
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&archive).unwrap(), archive_bytes);
        println!("actual predecessor Runtime replay and current denial verified {name}");
    }
}
