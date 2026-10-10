//! Exact keyed facts remain preconditions within the original native context.
use super::*;
use crate::format::STATE;

#[test]
fn same_sized_record_change_is_rejected_without_foreign_context() {
    let (storage, directory) = crate::native_work_tests::single_work_storage();
    let tx = storage.db.begin_write().unwrap();
    let mut state = tx.open_table(STATE).unwrap();
    let mut work = NativeWork::new(&tx, &state).unwrap();
    let delta =
        rom::storage_support::work::prepare_update(&work, rom::WorkUpdate::Claim { now: 0 })
            .unwrap();
    let before = work.record("work").unwrap().unwrap();
    let mut changed = before.clone();
    changed.pending.definition = "tast".into();
    let original = serde_json::to_string(&before).unwrap();
    let raw = serde_json::to_string(&changed).unwrap();
    assert_eq!(original.len(), raw.len());
    work.records.insert("work", raw.as_str()).unwrap();
    assert_eq!(
        work.apply(delta, &mut state, &mut || Ok(())),
        Err(Error::Conflict)
    );
    assert_eq!(work.record("work").unwrap(), Some(changed));
    drop(work);
    drop(state);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn same_sized_root_change_is_rejected_without_foreign_context() {
    let (storage, directory) = crate::native_work_tests::single_work_storage();
    let tx = storage.db.begin_write().unwrap();
    let mut state = tx.open_table(STATE).unwrap();
    let mut work = NativeWork::new(&tx, &state).unwrap();
    let delta =
        rom::storage_support::work::prepare_update(&work, rom::WorkUpdate::Claim { now: 0 })
            .unwrap();
    let before = work.root("request").unwrap().unwrap();
    let mut changed = before;
    changed.used += 1;
    let original = serde_json::to_string(&before).unwrap();
    let raw = serde_json::to_string(&changed).unwrap();
    assert_eq!(original.len(), raw.len());
    work.roots.insert("request", raw.as_str()).unwrap();
    assert_eq!(
        work.apply(delta, &mut state, &mut || Ok(())),
        Err(Error::Conflict)
    );
    assert_eq!(work.root("request").unwrap(), Some(changed));
    drop(work);
    drop(state);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}
