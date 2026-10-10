//! Compile only against separately verified accepted 0.0.3 source.
mod population;
use rom::*;
use rom_backup::{Backend, BackupLimits};
use std::path::Path;
#[derive(Clone, Resource)]
#[resource(name = "upgrade-delayed-items")]
struct Item {
    name: String,
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "usage: predecessor-writer sqlite|redb DIRECTORY populate|read-archive"
    );
    let root = Path::new(&args[2]);
    let backend = match args[1].as_str() {
        "sqlite" => Backend::Sqlite,
        "redb" => Backend::Redb,
        _ => panic!("unsupported adapter"),
    };
    let archive = root.join("before.rombk");
    if args[3] == "reject-archive" {
        assert!(archive.is_file(), "missing archive");
        assert!(
            matches!(
                rom_backup::read(&archive, backend, BackupLimits::default()),
                Err(Error::Unsupported(_))
            ),
            "old archive reader must reject new format"
        );
        return;
    }
    if args[3] == "read-archive" {
        assert!(archive.is_file(), "missing archive");
        let (manifest, _) = rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
        assert_eq!((manifest.archive_version, manifest.storage_format), (6, 8));
        return;
    }
    assert_eq!(args[3], "populate");
    assert!(root.is_dir(), "evidence parent must exist");
    let source = root.join("source");
    assert!(
        !source.exists() && !archive.exists() && !root.join("summary.json").exists(),
        "never overwrite evidence"
    );
    let finish = |store: &dyn Storage| {
        let (claim, operator) = population::populate(store);
        let cursor = store.journal_head(Item::KIND).unwrap();
        json!({"writer_release":"0.0.3","storage_format":8,"archive_version":6,"claim":claim,"operator":operator,"cursor":cursor})
    };
    let mut summary = match backend {
        Backend::Sqlite => {
            let store = rom_sqlite::Sqlite::open(&source).unwrap();
            let summary = finish(&store);
            store.backup_to(&archive, BackupLimits::default()).unwrap();
            summary
        }
        Backend::Redb => {
            let store = rom_redb::Redb::open(&source).unwrap();
            let summary = finish(&store);
            store.backup_to(&archive, BackupLimits::default()).unwrap();
            summary
        }
    };
    let (manifest, snapshot) =
        rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
    assert_eq!((manifest.archive_version, manifest.storage_format), (6, 8));
    summary["snapshot"] = json!(snapshot);
    std::fs::write(root.join("summary.json"), summary.to_string()).unwrap();
    assert_eq!(snapshot.rows.len(), 2);
    assert_eq!(snapshot.receipts.len(), 2);
    assert_eq!(manifest.operator_receipts, 1);
    println!("predecessor-writer {} populated native8/archive6", args[1]);
}
