//! Public Storage fixture: one selected commit succeeds, then its receipt stays unavailable until reopen.
use rom::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[derive(Clone, Copy)]
pub enum Point {
    Waiting,
    Reserve,
    ReserveBefore,
    Settlement,
    Tombstone,
    Prepared,
    Start,
}
pub struct FaultAudit {
    pub point: Point,
    pub successor_starts: AtomicU64,
    pub work: Mutex<Vec<WorkRecord>>,
    pub hide_receipt_until_reopen: AtomicBool,
    pub armed: AtomicBool,
    pub fired: AtomicBool,
    pub captured: Mutex<Option<Bundle>>,
    pub reopened_receipt: Mutex<Option<Receipt>>,
}
impl FaultAudit {
    pub fn new() -> Self {
        Self::at(Point::Waiting)
    }
    pub fn at(point: Point) -> Self {
        Self {
            point,
            successor_starts: AtomicU64::new(0),
            work: Mutex::new(vec![]),
            hide_receipt_until_reopen: AtomicBool::new(false),
            armed: AtomicBool::new(true),
            fired: AtomicBool::new(false),
            captured: Mutex::new(None),
            reopened_receipt: Mutex::new(None),
        }
    }
}
pub struct WaitingFault {
    pub inner: Arc<dyn Storage>,
    pub audit: Arc<FaultAudit>,
}
impl Storage for WaitingFault {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)?;
        if let Some(bundle) = self.audit.captured.lock().unwrap().as_ref() {
            *self.audit.reopened_receipt.lock().unwrap() =
                self.inner.receipt(&bundle.receipt.identity)?;
            self.audit
                .hide_receipt_until_reopen
                .store(false, Ordering::SeqCst);
        }
        Ok(())
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        let result = self.inner.reaction_update(update)?;
        *self.audit.work.lock().unwrap() = self.inner.reaction_records()?;
        Ok(result)
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
        if self.audit.hide_receipt_until_reopen.load(Ordering::SeqCst)
            && self
                .audit
                .captured
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|b| b.receipt.identity == identity)
        {
            return Err(Error::Unknown);
        }
        self.inner.receipt(identity)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        let row = &bundle.receipt.row;
        let selected = match self.audit.point {
            Point::Waiting | Point::Prepared | Point::Start => {
                row.key.kind == "ai_runs"
                    && row
                        .value
                        .as_ref()
                        .and_then(|v| rom_ai::flow::AiRun::decode(v.clone()).ok())
                        .and_then(|v| v.record().ok())
                        .is_some_and(|r| match self.audit.point {
                            Point::Waiting => {
                                matches!(r.state(), rom_ai::flow::RunState::Waiting { .. })
                            }
                            Point::Prepared => {
                                r.counters().generation_attempts() == 2
                                    && r.state() == &rom_ai::flow::RunState::Prepared
                            }
                            Point::Start => {
                                r.counters().generation_attempts() == 2
                                    && r.state() == &rom_ai::flow::RunState::Executing
                            }
                            _ => false,
                        })
            }
            Point::Reserve | Point::ReserveBefore | Point::Settlement | Point::Tombstone => {
                row.key.kind == "ai_budgets"
                    && row
                        .value
                        .as_ref()
                        .and_then(|v| rom_ai::flow::AiBudget::decode(v.clone()).ok())
                        .and_then(|v| v.record().ok())
                        .is_some_and(|r| match self.audit.point {
                            Point::Reserve | Point::ReserveBefore => r.entries().iter().any(|e| {
                                e.key().attempt_ordinal() == 2
                                    && e.status() == &rom_ai::flow::ReservationStatus::Reserved
                            }),
                            Point::Tombstone => r.entries().iter().any(|e| {
                                e.key().attempt_ordinal() == 2
                                    && e.status()
                                        == &rom_ai::flow::ReservationStatus::Settled {
                                            actual_cost: rom_ai::UsdNanos(0),
                                        }
                            }),
                            Point::Settlement => r.entries().iter().any(|e| {
                                e.key().attempt_ordinal() == 1
                                    && matches!(
                                        e.status(),
                                        rom_ai::flow::ReservationStatus::Settled { .. }
                                    )
                            }),
                            _ => false,
                        })
            }
        };
        if selected && self.audit.armed.swap(false, Ordering::SeqCst) {
            if matches!(self.audit.point, Point::ReserveBefore) {
                *self.audit.captured.lock().unwrap() = Some(bundle.clone());
                self.audit
                    .hide_receipt_until_reopen
                    .store(true, Ordering::SeqCst);
                self.audit.fired.store(true, Ordering::SeqCst);
                return Err(Error::Unknown);
            }
            let receipt = self.inner.commit(bundle)?;
            assert_eq!(
                receipt, bundle.receipt,
                "exact successful checkpoint receipt"
            );
            assert_eq!(self.inner.receipt(&receipt.identity)?, Some(receipt));
            *self.audit.captured.lock().unwrap() = Some(bundle.clone());
            self.audit
                .hide_receipt_until_reopen
                .store(true, Ordering::SeqCst);
            self.audit.fired.store(true, Ordering::SeqCst);
            return Err(Error::Unknown);
        }
        let receipt = self.inner.commit(bundle)?;
        if row.key.kind == "ai_runs"
            && row
                .value
                .as_ref()
                .and_then(|v| rom_ai::flow::AiRun::decode(v.clone()).ok())
                .and_then(|v| v.record().ok())
                .is_some_and(|r| {
                    r.counters().generation_attempts() == 2
                        && r.state() == &rom_ai::flow::RunState::Executing
                })
        {
            self.audit.successor_starts.fetch_add(1, Ordering::SeqCst);
        }
        Ok(receipt)
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
