use std::process::Command;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn runtime(redb: bool, path: &std::path::Path) -> rom::Runtime {
    let store: Arc<dyn rom::Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    rom_demo::build(store, rom_demo::Notices::default()).unwrap()
}

#[test]
fn reference_command_recovers_pending_work_on_both_stores() {
    for backend in ["sqlite", "redb"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rom-demo"))
            .args(["reference", backend])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Reference recovery passed"));
    }
}

#[test]
fn committed_rejection_recovers_after_process_exit_without_shutdown() {
    for backend in ["sqlite", "redb"] {
        let path = std::env::temp_dir().join(format!(
            "rom-reference-exit-{}-{backend}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let scratch = Scratch(path);
        let database = scratch.0.join("db");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "reference_process_exit_child",
                "--nocapture",
            ])
            .env("ROM_REFERENCE_CHILD_PATH", &database)
            .env("ROM_REFERENCE_CHILD_BACKEND", backend)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("child preparation timed out for {backend}");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(86), "child must reach post-commit exit");
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let reopened = runtime(backend == "redb", &database);
            let blobs =
                rom_demo::attachments::open(reopened.clone(), &scratch.0.join("objects")).unwrap();
            assert_eq!(
                blobs
                    .read(&rom_demo::session_actor(), rom_demo::attachments::ID)
                    .await
                    .unwrap(),
                rom_demo::attachments::CONTENT
            );
            rom_demo::reference::recover(&reopened).await.unwrap();
            blobs.shutdown().await.unwrap();
            reopened.shutdown().await.unwrap();
        });
    }
}

#[test]
#[ignore = "fault helper invoked only by committed_rejection_recovers_after_process_exit_without_shutdown"]
fn reference_process_exit_child() {
    let path = PathBuf::from(std::env::var_os("ROM_REFERENCE_CHILD_PATH").expect("parent fixture"));
    let redb = std::env::var("ROM_REFERENCE_CHILD_BACKEND").unwrap() == "redb";
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let runtime = runtime(redb, &path);
        let objects = path.parent().unwrap().join("objects");
        let blobs = rom_demo::attachments::open(runtime.clone(), &objects).unwrap();
        rom_demo::attachments::attach(&blobs).await.unwrap();
        rom_demo::reference::prepare(&runtime).await.unwrap();
        // No shutdown, drop, worker drain or finalizer after the confirmed commit.
        std::process::exit(86);
    });
}
