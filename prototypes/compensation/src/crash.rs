//! Actual child-process termination, distinct from the clean reopen cases.
use crate::{fixture::*, model::*, scenarios::stock_setup};
use rom::{Command, Resource};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command as Process, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
pub async fn child(path: &Path, stage: &str) {
    let f = Fixture::open("sqlite", path, Options::default());
    let code = match stage {
        "failure" => 86,
        "compensation" => 87,
        _ => panic!("invalid crash stage"),
    };
    f.sqlite
        .as_ref()
        .unwrap()
        .on_commit(Some(Arc::new(move |point| {
            if point == usize::MAX {
                std::process::exit(code);
            }
            Ok(())
        })));
    let mut trace = vec![];
    match stage {
        "failure" => {
            f.act(
                "flow-a",
                OBSERVE,
                "confirmed_permanent".into(),
                "confirmed-rejection",
                &mut trace,
            )
            .await;
        }
        "compensation" => f.drain(&mut trace).await,
        _ => unreachable!(),
    }
    panic!("post-commit failpoint was not reached");
}
pub async fn cases(executable: &Path) -> Vec<Value> {
    let mut results = vec![];
    for stage in ["failure", "compensation"] {
        let dir = Scratch::new();
        let path = dir.0.join("db");
        let mut trace = vec![];
        let f = Fixture::open("sqlite", &path, Options::default());
        stock_setup(&f, &mut trace, true).await;
        if stage == "compensation" {
            f.act(
                "flow-a",
                OBSERVE,
                "confirmed_permanent".into(),
                "confirmed-rejection",
                &mut trace,
            )
            .await;
        }
        f.finish().await;
        let mut child = Process::new(executable)
            .args(["crash-child", path.to_str().unwrap(), stage])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("crash child timeout");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(
            status.code(),
            Some(if stage == "failure" { 86 } else { 87 })
        );
        trace.push(json!({"step":"actual_child_process_exit_after_SQLite_commit","exit_code":status.code(),"stage":stage,"rust_destructors_ran":false,"method":"std::process::exit in SQLite post-commit observer"}));
        let f = Fixture::open("sqlite", &path, Options::default());
        let before = f.read::<Inventory>("sku").await;
        record("immediately_after_process_restart", &before, &mut trace);
        assert_eq!(
            before.value.as_ref().unwrap().released.len(),
            if stage == "failure" { 0 } else { 1 }
        );
        f.drain(&mut trace).await;
        let replay = f
            .runtime
            .execute(
                &author(),
                Command::action("flow-a", OBSERVE, "confirmed_permanent".into())
                    .at_revision(1)
                    .idempotency("confirmed-rejection"),
            )
            .await
            .unwrap();
        record(
            "replay_original_failure_receipt_after_process_restart",
            &replay,
            &mut trace,
        );
        f.drain(&mut trace).await;
        let after = f.read::<Inventory>("sku").await;
        record("final_after_crash_recovery", &after, &mut trace);
        assert_eq!(after.revision, 5);
        let stock = after.value.unwrap();
        assert_eq!(stock.released, ["flow-a"]);
        assert_eq!(stock.reservations.get("flow-b"), Some(&2));
        assert_eq!(stock.total, 15);
        let facts = f
            .runtime
            .journal(&author(), Inventory::KIND, None)
            .await
            .unwrap();
        assert_eq!(facts.events.len(), 5);
        results.push(evidence("sqlite",&format!("process_exit_after_{stage}_commit"),"native_commit_and_durable_identity_survive_process_exit",trace,json!({"inventory_revision":5,"inventory_journal_facts":5,"release_entries":1,"other_reservation_preserved":true,"crash_evidence":"real_process_exit_after_SQLite_commit_not_power_loss"})));
        f.finish().await;
    }
    results
}
