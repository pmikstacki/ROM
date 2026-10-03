//! Signal the actual binary as soon as its first readiness line is observed.
#[allow(dead_code)]
#[path = "../../../tests/persistence/tests/support/child_process.rs"]
mod child_process;
#[path = "../../src/scratch.rs"]
mod scratch;
use child_process::Process;
use scratch::Scratch;
use std::{
    fs::File,
    path::Path,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

pub fn run(backend: &str, signal: &str) {
    let scratch = Scratch::new("demo-server-lifecycle").unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        exercise(&scratch, backend, signal)
    }));
    if let Err(error) = result {
        let path = scratch.0.clone();
        std::mem::forget(scratch);
        eprintln!(
            "Failed server lifecycle evidence retained: {}",
            path.display()
        );
        std::panic::resume_unwind(error);
    }
}
fn read(path: &Path) -> String {
    assert!(
        std::fs::metadata(path).unwrap().len() <= 64 * 1024,
        "server output bound exceeded"
    );
    std::fs::read_to_string(path).unwrap()
}
fn exercise(scratch: &Scratch, backend: &str, signal: &str) {
    let database = scratch.0.join("db");
    let out = scratch.0.join("stdout");
    let err = scratch.0.join("stderr");
    let mut command = Command::new(env!("CARGO_BIN_EXE_rom-demo"));
    command
        .args(["serve", backend])
        .arg(&database)
        .arg("0")
        .stdout(File::create(&out).unwrap())
        .stderr(File::create(&err).unwrap());
    let mut server = Process::spawn(command);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let stdout = read(&out);
        if stdout.lines().next().is_some_and(|line| {
            line.starts_with("Local synthetic demo at http://") && stdout.contains('\n')
        }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "server readiness timed out: {}",
            read(&err)
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut kill = Command::new("kill");
    kill.args(["-s", signal, &server.pid().to_string()]);
    assert!(
        Process::spawn(kill).wait().success(),
        "signal delivery failed"
    );
    let status = server.wait();
    assert!(
        status.success(),
        "{backend} SIG{signal} exited {status}: {}",
        read(&err)
    );
    let stdout = read(&out);
    let stopped = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Stopped: "))
        .expect("server must report completed drain");
    let status: serde_json::Value = serde_json::from_str(stopped).unwrap();
    assert_eq!(status["intake"], "Stopped");
    assert_eq!(status["owned_work"], 0);
    assert_eq!(status["failed"], false);
    assert!(read(&err).is_empty());
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let storage: Arc<dyn rom::Storage> = if backend == "redb" {
            Arc::new(rom_redb::Redb::open(&database).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&database).unwrap())
        };
        let runtime = rom_demo::build(storage, rom_demo::Notices::default()).unwrap();
        let blobs =
            rom_demo::attachments::open(runtime.clone(), &scratch.0.join("db.objects")).unwrap();
        assert_eq!(
            blobs
                .read(&rom_demo::session_actor(), rom_demo::attachments::ID)
                .await
                .unwrap(),
            rom_demo::attachments::CONTENT
        );
        blobs.shutdown().await.unwrap();
        runtime.shutdown().await.unwrap();
        assert_eq!(runtime.status().unwrap().owned_work, 0);
    });
}
