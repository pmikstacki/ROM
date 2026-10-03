//! Real process interruption on both sides of an operator control commit.
use super::support::{Db, Fixture, control, receipt_count};
#[path = "../support/child_process.rs"]
mod child_process;
use child_process::Process;
use rom::{StorageWorkControl, WorkControlDecision};
use std::{
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

#[test]
fn operator_owner_child() {
    let Some(root) = std::env::var_os("ROM_OPERATOR_RECOVERY_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let redb = std::env::var("ROM_OPERATOR_RECOVERY_BACKEND").unwrap() == "redb";
    let committed = std::env::var("ROM_OPERATOR_RECOVERY_POINT").unwrap() == "after";
    let request =
        serde_json::from_slice(&std::fs::read(root.join("request.json")).unwrap()).unwrap();
    let db = Db::open(redb, &root.join("database"));
    let point = if committed { usize::MAX } else { 0 };
    db.observe(Some(Arc::new(move |observed| {
        if observed == point {
            std::fs::write(root.join("ready"), b"paused").unwrap();
            loop {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        Ok(())
    })));
    let _ = db.storage().control_work(&StorageWorkControl {
        principal: "operator".into(),
        request,
        decision: WorkControlDecision::Retry,
        now: 20,
    });
    panic!("child must reach its interruption checkpoint");
}

#[test]
fn owner_exit_preserves_exactly_the_committed_operator_transition() {
    for redb in [false, true] {
        for committed in [false, true] {
            let mut fixture = Fixture::new(redb);
            let before = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            let control = control(&before, "operator", "process-retry");
            std::fs::write(
                fixture.root().join("request.json"),
                serde_json::to_vec(&control.request).unwrap(),
            )
            .unwrap();
            fixture.close();
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", "process::operator_owner_child", "--nocapture"])
                .env("ROM_OPERATOR_RECOVERY_ROOT", fixture.root())
                .env(
                    "ROM_OPERATOR_RECOVERY_BACKEND",
                    if redb { "redb" } else { "sqlite" },
                )
                .env(
                    "ROM_OPERATOR_RECOVERY_POINT",
                    if committed { "after" } else { "before" },
                )
                .stdin(Stdio::null());
            let mut child = Process::spawn(command);
            child.wait_ready(&fixture.root().join("ready"));
            let status = child.kill();
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(status.signal(), Some(9));
            }
            fixture.reopen();
            let after = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            assert_eq!(receipt_count(&after), usize::from(committed));
            assert_eq!(
                after.records[0].revision,
                before.records[0].revision + u64::from(committed)
            );
            let recovered = fixture.db.storage().control_work(&control).unwrap();
            assert_eq!(recovered.result.replayed, committed);
            assert_eq!(
                recovered.result.version.revision,
                before.records[0].revision + 1
            );
            assert_eq!(fixture.db.counts(), [1, 1, 1, 0]);
            fixture.reopen();
            assert!(
                fixture
                    .db
                    .storage()
                    .control_work(&control)
                    .unwrap()
                    .result
                    .replayed
            );
            assert_eq!(
                receipt_count(&fixture.db.storage().work_snapshot(32, 100_000).unwrap()),
                1
            );
        }
    }
}
