use super::support::{Directory, sidecar};
use rom::Error;
use rom_backup::{NativeAccess, NativeOwnership};
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Process(Child);
impl Process {
    fn spawn(path: &Path, ready: &Path, go: &Path, release: &Path, access: &str) -> Self {
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "processes::ownership_child", "--nocapture"])
                .env("ROM_NATIVE_OWNER_CHILD_PATH", path)
                .env("ROM_NATIVE_OWNER_CHILD_READY", ready)
                .env("ROM_NATIVE_OWNER_CHILD_GO", go)
                .env("ROM_NATIVE_OWNER_CHILD_RELEASE", release)
                .env("ROM_NATIVE_OWNER_CHILD_ACCESS", access)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    fn wait(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "ownership child timed out");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "ownership child did not signal");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn ownership_child() {
    let Some(path) = std::env::var_os("ROM_NATIVE_OWNER_CHILD_PATH") else {
        return;
    };
    let ready = std::path::PathBuf::from(std::env::var_os("ROM_NATIVE_OWNER_CHILD_READY").unwrap());
    let go = std::path::PathBuf::from(std::env::var_os("ROM_NATIVE_OWNER_CHILD_GO").unwrap());
    let release =
        std::path::PathBuf::from(std::env::var_os("ROM_NATIVE_OWNER_CHILD_RELEASE").unwrap());
    wait_file(&go);
    let access = match std::env::var("ROM_NATIVE_OWNER_CHILD_ACCESS")
        .unwrap()
        .as_str()
    {
        "existing" => NativeAccess::Existing,
        "fresh" => NativeAccess::Fresh,
        _ => panic!("unknown child access"),
    };
    let signal = |bytes: &[u8]| {
        let pending = ready.with_extension("pending");
        fs::write(&pending, bytes).unwrap();
        fs::rename(pending, &ready).unwrap();
    };
    match NativeOwnership::acquire(Path::new(&path), access) {
        Ok(_owner) => {
            signal(b"owned");
            wait_file(&release);
            std::process::exit(86);
        }
        Err(Error::Conflict) => {
            signal(b"conflict");
            std::process::exit(87);
        }
        Err(error) => panic!("unexpected ownership error: {error:?}"),
    }
}

#[test]
fn process_exit_releases_ownership_without_sidecar_removal() {
    let directory = Directory::new();
    let path = directory.database();
    let ready = directory.file("ready");
    let go = directory.file("go");
    let release = directory.file("release");
    fs::write(&go, b"go").unwrap();
    let mut child = Process::spawn(&path, &ready, &go, &release, "existing");
    wait_file(&ready);
    assert_eq!(fs::read(&ready).unwrap(), b"owned");
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Existing).err(),
        Some(Error::Conflict)
    );
    fs::write(&release, b"release").unwrap();
    assert_eq!(child.wait().code(), Some(86));
    assert!(sidecar(&path).exists());
    NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
}

#[test]
fn two_processes_contend_before_a_fresh_database_is_created() {
    let directory = Directory::new();
    let path = directory.file("not-created");
    let ready_a = directory.file("ready-a");
    let ready_b = directory.file("ready-b");
    let go = directory.file("go");
    let release = directory.file("release");
    let mut a = Process::spawn(&path, &ready_a, &go, &release, "fresh");
    let mut b = Process::spawn(&path, &ready_b, &go, &release, "fresh");
    fs::write(&go, b"go").unwrap();
    wait_file(&ready_a);
    wait_file(&ready_b);
    let mut results = vec![fs::read(&ready_a).unwrap(), fs::read(&ready_b).unwrap()];
    results.sort();
    assert_eq!(results, vec![b"conflict".to_vec(), b"owned".to_vec()]);
    assert!(!path.exists());
    fs::write(&release, b"release").unwrap();
    let mut statuses = vec![a.wait().code().unwrap(), b.wait().code().unwrap()];
    statuses.sort();
    assert_eq!(statuses, vec![86, 87]);
    NativeOwnership::acquire(&path, NativeAccess::Fresh).unwrap();
}
