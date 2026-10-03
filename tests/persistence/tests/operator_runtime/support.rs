use rom::operator::*;
use rom::*;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
#[derive(Clone, Resource)]
#[resource(name = "operator-runtime-items")]
pub struct Item {
    pub secret: String,
}
pub const NOTICE: Channel<String> = Channel::new("notice", 1);
pub const CHANGE: Action<Item, String> = Action::new("change", |state, input| {
    state.secret = input;
    Ok(vec![])
});
pub struct Clock20;
impl Clock for Clock20 {
    fn now(&self) -> u64 {
        20
    }
}
pub fn service() -> Actor {
    Actor::trusted("host", "secret-worker").with_kind(PrincipalKind::Service)
}
pub fn actor() -> Actor {
    Actor::trusted("host", "operator")
}
pub struct Policy {
    pub inspect: bool,
    pub retry: bool,
    pub reconcile: bool,
    pub revoked: Arc<AtomicBool>,
}
impl Policy {
    pub fn all() -> Self {
        Self {
            inspect: true,
            retry: true,
            reconcile: true,
            revoked: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl OperatorAuthorizer for Policy {
    fn authorize(
        &self,
        _: &Actor,
        access: OperatorAccess,
        scope: Option<&WorkScope>,
        _: &mut dyn AuthorizationRead,
    ) -> Result<()> {
        let allowed = match access {
            OperatorAccess::Inspect => self.inspect,
            OperatorAccess::Retry => self.retry,
            OperatorAccess::Reconcile => self.reconcile,
        };
        if !allowed
            || self.revoked.load(Ordering::SeqCst)
            || scope.is_some_and(|s| {
                s.source
                    .as_ref()
                    .is_some_and(|k| k.id.starts_with("hidden"))
            })
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
type Counts = Arc<dyn Fn() -> [u64; 4] + Send + Sync>;
pub struct Fixture {
    pub store: Arc<dyn Storage>,
    pub path: PathBuf,
    pub counts: Counts,
}
impl Fixture {
    pub fn new(redb: bool, ids: &[&str]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-operator-runtime-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let (store, counts): (Arc<dyn Storage>, Counts) = if redb {
            let db = Arc::new(rom_redb::Redb::open(&path).unwrap());
            let stats = db.clone();
            (db, Arc::new(move || stats.counts().unwrap()))
        } else {
            let db = Arc::new(rom_sqlite::Sqlite::open(&path).unwrap());
            let stats = db.clone();
            (db, Arc::new(move || stats.counts().unwrap()))
        };
        store.register(&[Item::descriptor()]).unwrap();
        for id in ids {
            let row = Row {
                key: Key {
                    kind: Item::KIND.into(),
                    id: (*id).into(),
                },
                revision: 1,
                value: Some(
                    Item {
                        secret: "secret-frozen-row".into(),
                    }
                    .encode(),
                ),
                protected: ProtectedMetadata::default(),
            };
            let pending = PendingWork {
                id: format!("secret-internal-{id}"),
                cause: Cause {
                    retry_epoch: 0,
                    root: format!("secret-root-{id}"),
                    parent: None,
                    depth: 0,
                    started_at: 10,
                    path: vec![],
                },
                definition: "notice".into(),
                version: 1,
                service_key: json!(["host", "service", "secret-worker"]).to_string(),
                delivery_profile: DeliveryProfile::AtLeastOnce,
                payload: WorkPayload::Notification {
                    source: row.clone(),
                    payload: json!("secret-notification"),
                },
            };
            store
                .commit(&Bundle {
                    expected: None,
                    receipt: Receipt {
                        retry_epoch: 0,
                        replay_version: None,
                        identity: format!("seed-{id}"),
                        fingerprint: "seed".into(),
                        row,
                    },
                    changed: true,
                    effects: vec![],
                    reactions: vec![pending],
                    reaction_limits: Some(ReactionLimits::default()),
                    completed_work: None,
                })
                .unwrap();
        }
        Self {
            store,
            path,
            counts,
        }
    }
    pub fn runtime(&self, policy: Option<Policy>) -> Runtime {
        let mut builder = builder();
        if let Some(policy) = policy {
            builder = builder.operator_authorizer(Arc::new(policy));
        }
        builder
            .build(self.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap()
    }
    pub fn snapshot(&self) -> StorageWorkSnapshot {
        self.store.work_snapshot(2048, 4 * 1024 * 1024).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub fn builder() -> Builder {
    Runtime::builder()
        .clock(Arc::new(Clock20))
        .resource(
            Item::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(CHANGE),
        )
        .channel(NOTICE, service(), |_| async { DeliveryOutcome::Accepted })
}
pub fn query(limit: usize) -> WorkQuery {
    WorkQuery {
        state: None,
        category: None,
        definition: None,
        limit,
        cursor: None,
    }
}
pub fn request(snapshot: &StorageWorkSnapshot, key: &str) -> WorkControlRequest {
    let record = &snapshot.records[0];
    WorkControlRequest {
        handle: WorkHandle::from_work_id(&record.pending.id),
        expected: snapshot.version(record),
        key: key.into(),
        retry_epoch: 0,
        operation: WorkControlOperation::Retry,
    }
}
