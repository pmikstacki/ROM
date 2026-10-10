//! Transparent actual-adapter wrapper with one lost tool action acknowledgement.
use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub struct ReadAdmissionFault {
    pub after_commit: bool,
    pub delivery_started: bool,
    pub result_commit: bool,
    pub armed: AtomicBool,
}
pub struct ActionFault {
    pub inner: Arc<dyn Storage>,
    pub armed: AtomicBool,
    pub fail_hold: AtomicBool,
    pub trace_read: bool,
    pub read_admission: Option<Arc<ReadAdmissionFault>>,
}
impl Storage for ActionFault {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        if let Some(fault) = &self.read_admission
            && fault.delivery_started
            && matches!(&update, WorkUpdate::DeliveryStarted { .. })
        {
            let scheduled = self
                .inner
                .load(&Key {
                    kind: "ai_runs".into(),
                    id: "tool-recovery".into(),
                })?
                .and_then(|row| row.value)
                .and_then(|value| value["encoded"].as_str().map(str::to_owned))
                .and_then(|encoded| serde_json::from_str::<serde_json::Value>(&encoded).ok())
                .is_some_and(|value| {
                    value["tool_turns"]
                        .as_array()
                        .and_then(|turns| turns.last())
                        .is_some_and(|turn| {
                            turn["steps"][0]["read_attempt"]["phase"] == "Scheduled"
                        })
                });
            if scheduled && fault.armed.swap(false, Ordering::SeqCst) {
                let WorkUpdate::DeliveryStarted { claim, .. } = &update else {
                    unreachable!()
                };
                let delivery_id = claim.id.clone();
                self.inner.reaction_update(update)?;
                assert!(
                    self.inner
                        .reaction_records()?
                        .iter()
                        .any(|record| record.pending.id == delivery_id
                            && record.delivery == Some(DeliveryOutcome::Unknown))
                );
                return Err(Error::Unknown);
            }
        }
        self.inner.reaction_update(update)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.inner.reaction_records()
    }
    fn supports_operator(&self) -> bool {
        self.inner.supports_operator()
    }
    fn work_snapshot(&self, rows: usize, bytes: usize) -> Result<StorageWorkSnapshot> {
        self.inner.work_snapshot(rows, bytes)
    }
    fn control_work(&self, control: &StorageWorkControl) -> Result<WorkControlReceipt> {
        self.inner.control_work(control)
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.inner.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(kind, rows, bytes)
    }
    fn query_read(&self, request: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        self.inner.query_read(request, bounds)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        self.inner.receipt(identity)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        let row = &bundle.receipt.row;
        if self.trace_read && row.key.kind == "ai_runs" {
            let state = row
                .value
                .as_ref()
                .and_then(|value| value["encoded"].as_str())
                .and_then(|encoded| serde_json::from_str::<serde_json::Value>(encoded).ok())
                .and_then(|value| value["state"].as_str().map(str::to_owned));
            eprintln!("fixture reaches durable run commit state {:?}", state);
        }
        if let Some(fault) = &self.read_admission {
            let phase = if fault.result_commit {
                "Completed"
            } else {
                "Scheduled"
            };
            let scheduled = row.key.kind == "ai_runs"
                && row
                    .value
                    .as_ref()
                    .and_then(|value| value["encoded"].as_str())
                    .and_then(|encoded| serde_json::from_str::<serde_json::Value>(encoded).ok())
                    .is_some_and(|value| {
                        value["tool_turns"]
                            .as_array()
                            .and_then(|turns| turns.last())
                            .is_some_and(|turn| turn["steps"][0]["read_attempt"]["phase"] == phase)
                    });
            if !fault.delivery_started && scheduled && fault.armed.swap(false, Ordering::SeqCst) {
                if fault.after_commit {
                    self.inner.commit(bundle)?;
                    return Err(Error::Unknown);
                }
                return Err(Error::NotCommitted);
            }
        }
        if row.key.kind == "typed_tool_task"
            && row
                .value
                .as_ref()
                .is_some_and(|value| value["label"] == "reviewed")
            && self.armed.swap(false, Ordering::SeqCst)
        {
            self.inner.commit(bundle)?;
            return Err(Error::Unknown);
        }
        if row.key.kind == "ai_runs"
            && row
                .value
                .as_ref()
                .and_then(|value| value["encoded"].as_str())
                .is_some_and(|encoded| encoded.contains("\"unknown\":true"))
            && self.fail_hold.swap(false, Ordering::SeqCst)
        {
            return Err(Error::NotCommitted);
        }
        self.inner.commit(bundle)
    }

    fn supports_journal(&self) -> bool {
        self.inner.supports_journal()
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.inner.journal_head(kind)
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        rows: usize,
        bytes: usize,
    ) -> Result<JournalPage> {
        self.inner.journal(kind, after, rows, bytes)
    }
}
