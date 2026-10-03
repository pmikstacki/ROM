//! Shared bounded subprocess lifecycle for native persistence acceptance tests.
use std::{
    path::Path,
    process::{Child, Command, ExitStatus},
    time::{Duration, Instant},
};

pub struct Process(Child);
impl Process {
    pub fn spawn(mut command: Command) -> Self {
        Self(command.spawn().unwrap())
    }

    pub fn wait_ready(&mut self, ready: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                self.0.try_wait().unwrap().is_none(),
                "child exited before readiness"
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
