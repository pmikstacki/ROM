//! Test subprocess: exits without Rust destructors at a real write boundary.
use rom_persistence_adapters::{
    RedbStore, SqliteStore,
    probe::{Checkpoint, Probe},
};
use rom_persistence_core::*;
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let point = match args[3].as_str() {
        "resource" => Checkpoint::AfterResource,
        "event" => Checkpoint::AfterEvent,
        "precommit" => Checkpoint::BeforeCommit,
        "committed" => Checkpoint::AfterCommit,
        _ => panic!("invalid test checkpoint"),
    };
    let probe = Probe::new(move |stage| {
        if stage == point {
            std::process::exit(73);
        }
        Ok(())
    });
    let transition = Transition {
        action: "crashed".into(),
        key: ResourceKey::new("tasks", "crashed"),
        expected_revision: None,
        value: Some(b"saved".to_vec()),
        events: vec![b"first".to_vec(), b"second".to_vec()],
    };
    let result = match args[1].as_str() {
        "sqlite" => SqliteStore::open(Path::new(&args[2]))
            .unwrap()
            .with_probe(probe)
            .commit(&transition),
        "redb" => RedbStore::open(Path::new(&args[2]))
            .unwrap()
            .with_probe(probe)
            .commit(&transition),
        _ => panic!("invalid test backend"),
    };
    panic!("checkpoint was not reached: {result:?}");
}
