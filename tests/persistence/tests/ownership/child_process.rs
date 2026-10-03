//! Shared bounded subprocess lifecycle for native ownership acceptance tests.
use super::Backend;
use std::{
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

pub struct Process(Child);
impl Process {
    pub fn spawn(test: &str, root: &Path, backend: Backend) -> Self {
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", test, "--nocapture"])
                .env("ROM_OWNERSHIP_TEST_ROOT", root)
                .env("ROM_OWNERSHIP_TEST_BACKEND", backend.name())
                .stdin(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }

    pub fn wait_ready(&mut self, ready: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                self.0.try_wait().unwrap().is_none(),
                "child exited before commit"
            );
            assert!(Instant::now() < deadline, "child readiness timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "child completion timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn kill(&mut self) -> ExitStatus {
        self.0.kill().unwrap();
        self.wait()
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
