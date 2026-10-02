use rom_salsa_reads_prototype::{RegisteredTaskReads, Snapshot, plain_open_ids};

fn show(label: &str, snapshot: &Snapshot, model: &RegisteredTaskReads) {
    let ids = model.open_ids();
    let count = model.open_count();
    assert_eq!(ids, plain_open_ids(snapshot));
    assert_eq!(count, ids.len());
    println!(
        "{label}: source_revision={} ids={ids:?} count={count} executions={:?}",
        snapshot.revision,
        model.executions()
    );
}

fn main() {
    let mut snapshot = Snapshot::fixture();
    let mut model = RegisteredTaskReads::default();
    model.apply(&snapshot).unwrap();
    show("cold", &snapshot, &model);
    for _ in 0..1_000 {
        assert_eq!(model.open_ids(), plain_open_ids(&snapshot));
        assert_eq!(model.open_count(), 1);
    }
    show("1000 unchanged reads", &snapshot, &model);
    snapshot.revision += 1;
    snapshot.tasks.get_mut(&1).unwrap().title = "renamed".into();
    println!("title setters={}", model.apply(&snapshot).unwrap());
    show("title edit", &snapshot, &model);
    snapshot.revision += 1;
    snapshot.tasks.get_mut(&2).unwrap().done = false;
    println!("completion setters={}", model.apply(&snapshot).unwrap());
    show("reopen task 2", &snapshot, &model);
    snapshot.revision += 1;
    snapshot.enabled = false;
    println!("policy setters={}", model.apply(&snapshot).unwrap());
    show("revoke actor", &snapshot, &model);
    snapshot.revision += 1;
    snapshot.enabled = true;
    snapshot.actor = 8;
    println!("actor/policy setters={}", model.apply(&snapshot).unwrap());
    show("switch actor", &snapshot, &model);
}
