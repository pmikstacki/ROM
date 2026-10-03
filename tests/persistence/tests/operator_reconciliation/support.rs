use rom::operator::*;
use rom::*;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Resource)]
#[resource(name = "operator-deliveries")]
pub struct Notice {
    pub count: u64,
}
pub const MAIL: Channel<String> = Channel::new("reconciled-mail", 1);
pub const NOTIFY: Action<Notice, ()> = Action::new("notify", |notice, ()| {
    notice.count += 1;
    Ok(vec![MAIL.intent("PRIVATE-FROZEN-PAYLOAD".into())])
});
pub struct TestClock(pub AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
pub fn actor() -> Actor {
    Actor::trusted("test-host", "operator")
}
pub fn service() -> Actor {
    Actor::trusted("test-host", "mailer").with_kind(PrincipalKind::Service)
}
pub struct Policy(pub Arc<AtomicBool>);
impl OperatorAuthorizer for Policy {
    fn authorize(
        &self,
        _: &Actor,
        _: OperatorAccess,
        _: Option<&WorkScope>,
        _: &mut dyn AuthorizationRead,
    ) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
}
pub enum Db {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Db {
    pub fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    pub fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    pub fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
        }
    }
}
#[derive(Clone, Copy)]
pub enum SendMode {
    Accepted,
    Unknown,
    Retryable,
    Permanent,
    Timeout,
    Panic,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    pub id: String,
    pub attempt: u32,
    pub payload: String,
}
pub struct Fixture {
    pub root: PathBuf,
    pub redb: bool,
    pub db: Option<Db>,
    pub clock: Arc<TestClock>,
    pub sends: Arc<Mutex<Vec<Sent>>>,
    pub revoked: Arc<AtomicBool>,
}
impl Fixture {
    pub fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "rom-reconciliation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self {
            db: Some(Db::open(redb, &root.join("database"))),
            root,
            redb,
            clock: Arc::new(TestClock(AtomicU64::new(100))),
            sends: Arc::new(Mutex::new(vec![])),
            revoked: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn db(&self) -> &Db {
        self.db.as_ref().unwrap()
    }
    pub fn storage(&self) -> Arc<dyn Storage> {
        self.db().storage()
    }
    pub fn snapshot(&self) -> StorageWorkSnapshot {
        self.storage().work_snapshot(2048, 4 * 1024 * 1024).unwrap()
    }
    pub fn builder(&self) -> Builder {
        Runtime::builder()
            .clock(self.clock.clone())
            .operator_authorizer(Arc::new(Policy(self.revoked.clone())))
            .delivery_timeout(Duration::from_millis(20))
            .resource(
                Notice::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields()
                    .field_policy(|actor, access, _, notice| {
                        actor.principal_kind() != PrincipalKind::Service
                            || !matches!(access, Access::Read)
                            || notice.count != 99
                    })
                    .action(NOTIFY),
            )
    }
    pub fn configure(
        &self,
        builder: Builder,
        registration: ChannelRegistration<String>,
        mode: SendMode,
    ) -> Builder {
        let sends = self.sends.clone();
        builder.channel_with(
            registration,
            service(),
            move |delivery: Delivery<String>| {
                sends.lock().unwrap().push(Sent {
                    id: delivery.id,
                    attempt: delivery.attempt,
                    payload: delivery.payload,
                });
                async move {
                    match mode {
                        SendMode::Accepted => DeliveryOutcome::Accepted,
                        SendMode::Unknown => DeliveryOutcome::Unknown,
                        SendMode::Retryable => DeliveryOutcome::Retryable,
                        SendMode::Permanent => DeliveryOutcome::Permanent,
                        SendMode::Timeout => std::future::pending().await,
                        SendMode::Panic => panic!("PRIVATE-PROVIDER-PANIC"),
                    }
                }
            },
        )
    }
    pub fn runtime(&self, registration: ChannelRegistration<String>, mode: SendMode) -> Runtime {
        self.configure(self.builder(), registration, mode)
            .build(self.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap()
    }
    pub fn close(&mut self) {
        self.db.take();
    }
    pub fn reopen(&mut self) {
        self.close();
        self.db = Some(Db::open(self.redb, &self.root.join("database")));
    }
    pub fn send_count(&self) -> usize {
        self.sends.lock().unwrap().len()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.close();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
pub async fn enqueue(runtime: &Runtime) {
    runtime
        .execute(
            &actor(),
            Command::create("one", Notice { count: 0 }).idempotency("create"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor(),
            Command::action("one", NOTIFY, ())
                .at_revision(1)
                .idempotency("notify"),
        )
        .await
        .unwrap();
}
pub fn request(fixture: &Fixture, key: &str) -> WorkControlRequest {
    let snapshot = fixture.snapshot();
    let record = &snapshot.records[0];
    WorkControlRequest {
        handle: WorkHandle::from_work_id(&record.pending.id),
        expected: snapshot.version(record),
        key: key.into(),
        retry_epoch: 0,
        operation: WorkControlOperation::Reconcile {
            evidence_ref: Some("opaque-provider-reference".into()),
        },
    }
}
pub async fn hold(fixture: &Fixture, runtime: &Runtime) {
    enqueue(runtime).await;
    assert_eq!(runtime.process_work(1).await.unwrap(), 1);
    assert_eq!(
        fixture.snapshot().records[0].state,
        WorkState::AwaitingReconciliation
    );
    assert_eq!(fixture.send_count(), 1);
}
pub async fn shutdown(runtime: Runtime) {
    tokio::time::timeout(Duration::from_secs(3), runtime.shutdown())
        .await
        .unwrap()
        .unwrap();
    drop(runtime);
}
