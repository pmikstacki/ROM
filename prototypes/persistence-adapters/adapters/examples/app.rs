//! The application's persistence path is identical for either host-selected store.
use rom_persistence_adapters::{RedbStore, SqliteStore};
use rom_persistence_core::*;

fn application<S: Storage>(storage: S) -> Result<(), Error> {
    let runtime = Runtime::new(storage, Requirements::default())?;
    let task = ResourceKey::new("tasks", "one");
    let transition = Transition {
        action: "create-task-1".into(),
        key: task.clone(),
        expected_revision: None,
        value: Some(b"Try the persistence contract".to_vec()),
        events: vec![b"task-created".to_vec()],
    };
    let receipt = runtime.execute(&transition)?;
    assert_eq!(runtime.execute(&transition)?, receipt);
    println!(
        "revision={}, events={}, task={}",
        receipt.revision,
        runtime.journal(None, 10)?.events.len(),
        String::from_utf8(runtime.load(&task)?.unwrap().value.unwrap()).unwrap()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scratch = tempfile::tempdir()?;
    println!("SQLite {}", SqliteStore::engine_version());
    application(SqliteStore::open(&scratch.path().join("sqlite"))?)?;
    println!("redb 4.3.0");
    application(RedbStore::open(&scratch.path().join("redb"))?)?;
    Ok(())
}
