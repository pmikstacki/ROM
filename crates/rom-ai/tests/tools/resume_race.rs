//! A locally owned caller pauses outside the Runtime gate after its snapshot read.
use super::*;
use std::sync::Condvar;
use std::time::Duration;

#[derive(Default)]
pub(super) struct Gate {
    entered: tokio::sync::Notify,
    once: AtomicBool,
    released: Mutex<bool>,
    release: Condvar,
    timed_out: AtomicBool,
}
impl Gate {
    pub(super) fn pause_once(&self) {
        if self.once.swap(true, Ordering::SeqCst) {
            return;
        }
        let released = self.released.lock().unwrap();
        self.entered.notify_one();
        let (released, timeout) = self
            .release
            .wait_timeout_while(released, Duration::from_secs(1), |released| !*released)
            .unwrap();
        if timeout.timed_out() && !*released {
            self.timed_out.store(true, Ordering::SeqCst);
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.release.notify_all();
    }
}
struct ReleaseOnDrop(Arc<Gate>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

pub(super) struct Race<'a> {
    pub runtime: &'a Runtime,
    pub client: &'a rom_ai::flow::FlowClient,
    pub storage: &'a Arc<dyn Storage>,
    pub adapter: &'a Arc<Adapter>,
    pub handle: &'a rom_ai::flow::RunHandle,
    pub revision: u64,
    pub money: &'a rom::JournalCursor,
    pub request: &'a CompletionRequest,
    pub gate: Arc<Gate>,
}
fn checked(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn failure(error: impl std::fmt::Debug) -> String {
    format!("race fixture operation failed: {error:?}")
}
impl Race<'_> {
    pub(super) async fn run(self) -> Result<(), String> {
        let release = ReleaseOnDrop(self.gate.clone());
        let observed =
            tokio::time::timeout(Duration::from_secs(20), Box::pin(self.observe())).await;
        drop(release);
        // No semantic assertion can bypass accepted-work shutdown or gate release.
        let shutdown = self.runtime.shutdown().await;
        shutdown.map_err(failure)?;
        observed.map_err(|_| "race fixture observation deadline elapsed".to_owned())?
    }
    async fn observe(&self) -> Result<(), String> {
        let actor = owner();
        let task_head = self.storage.journal_head(Task::KIND).map_err(failure)?;
        let task_events = self
            .storage
            .journal(Task::KIND, None, 64, 64 * 1024)
            .map_err(failure)?;
        let original_identity = &task_events
            .events
            .last()
            .ok_or("original task event missing")?
            .identity;
        let original_receipt = self
            .storage
            .receipt(original_identity)
            .map_err(failure)?
            .ok_or("original task receipt missing")?;
        let mut first = Box::pin(self.client.resume(
            &actor,
            self.handle,
            self.revision,
            "resume-original-tool",
        ));
        let entered = tokio::time::timeout(Duration::from_millis(500), async {
            tokio::select! {
                result = &mut first => Err(format!("first resume completed before authority entry: {result:?}")),
                _ = self.gate.entered.notified() => Ok(()),
            }
        }).await;
        // The first caller is now unpolled. Release its callback before the duplicate
        // can acquire the Runtime gate; the callback never waits for the duplicate.
        self.gate.release();
        entered.map_err(|_| "authority readiness deadline elapsed".to_owned())??;
        let duplicate = Box::pin(self.client.resume(
            &actor,
            self.handle,
            self.revision,
            "resume-original-tool",
        ))
        .await;
        let original = first.await;
        let head = self.storage.journal_head(AiRun::KIND).map_err(failure)?;
        let work = self.storage.reaction_records().map_err(failure)?;
        let repeated = Box::pin(self.client.resume(
            &actor,
            self.handle,
            self.revision,
            "resume-original-tool",
        ))
        .await;
        let repeat_head = self.storage.journal_head(AiRun::KIND).map_err(failure)?;
        let repeat_work = self.storage.reaction_records().map_err(failure)?;
        let money_unchanged = self
            .storage
            .journal(
                rom_ai::flow::AiBudget::KIND,
                Some(self.money),
                64,
                64 * 1024,
            )
            .map_err(failure)?
            .events
            .is_empty();
        for _ in 0..8 {
            self.runtime.process_work(4).await.map_err(failure)?;
        }
        let completed = self
            .client
            .view(&actor, self.handle)
            .await
            .map_err(failure)?;
        let task = self
            .runtime
            .read::<Task>(&actor, "task")
            .await
            .map_err(failure)?;
        let record = self
            .runtime
            .read::<AiRun>(&service(), &self.handle.0)
            .await
            .map_err(failure)?
            .value
            .ok_or("run record missing")?
            .record()
            .map_err(failure)?;
        let original_outcome = match &original {
            Ok(_) => "ok",
            Err(rom_ai::AiError::Conflict) => "conflict",
            Err(rom_ai::AiError::DeadlineExceeded) => "deadline",
            Err(_) => "other_error",
        };
        eprintln!(
            "resume race observation: original={original_outcome} duplicate_ok={} completed={} task_revision={} lookups={} attempts={}",
            duplicate.is_ok(),
            completed.state() == &RunState::Completed,
            task.revision,
            self.adapter.lookups.load(Ordering::SeqCst),
            self.adapter.attempts.lock().unwrap().len()
        );
        checked(
            !self.gate.timed_out.load(Ordering::SeqCst),
            "authority callback release deadline elapsed",
        )?;
        checked(
            duplicate.is_ok(),
            "pending duplicate must retain its original operation stamp",
        )?;
        checked(
            original.is_ok(),
            &format!(
                "original recovery must succeed after a pending duplicate; original={original_outcome}"
            ),
        )?;
        checked(
            repeated.is_ok(),
            "completed duplicate operation must replay",
        )?;
        checked(
            head == repeat_head && work == repeat_work,
            "duplicate replay changed run journal or Work",
        )?;
        checked(
            money_unchanged,
            "duplicate recovery charged the original attempt again",
        )?;
        checked(
            completed.state() == &RunState::Completed,
            "original tool recovery did not complete",
        )?;
        checked(
            self.adapter.lookups.load(Ordering::SeqCst) == 0,
            "recovery added a provider lookup",
        )?;
        checked(
            self.adapter.attempts.lock().unwrap().len() == 2,
            "recovery repeated a provider attempt",
        )?;
        checked(
            task.revision == 2,
            "original receipt replay repeated the task mutation",
        )?;
        checked(
            self.storage
                .journal(Task::KIND, Some(&task_head), 64, 64 * 1024)
                .map_err(failure)?
                .events
                .is_empty(),
            "original receipt replay published another task event",
        )?;
        checked(
            self.storage.receipt(original_identity).map_err(failure)? == Some(original_receipt),
            "recovery changed the original durable receipt",
        )?;
        checked(
            record.request() == self.request && record.counters().tool_calls() == 2,
            "recovery changed the original request or tool count",
        )
    }
}
