//! Actual host cancellation must retain accepted header knowledge without another POST.
#![cfg(feature = "test-support")]
#[path = "support/credentials.rs"]
mod credentials;
use credentials::Credentials;
#[path = "support/stalled_body.rs"]
mod stalled;
use rom::{Actor, Runtime, Storage};
use rom_ai::flow::{AiBudget, AiRun, FlowAuthority, FlowHost, OwnerIdentity, Submission};
use rom_ai::{
    AiClock, AiError, AiResult, CompletionRequest, Message, ModelPrice, OutputValidator,
    PreparedAttempt, RoutingPolicy, RunLimits, UsdNanos,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
fn owner() -> Actor {
    Actor::trusted("fixture", "owner")
}
fn service() -> Actor {
    Actor::trusted("fixture", "ai-host").with_kind(rom::PrincipalKind::Service)
}
struct Clock;
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
struct Authority(AtomicBool);
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        if OwnerIdentity::from_actor(&owner()).unwrap().matches(actor) {
            OwnerIdentity::from_actor(actor)
        } else {
            Err(AiError::Denied)
        }
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor) && identity.matches(&owner()) {
            Ok(())
        } else {
            Err(AiError::Denied)
        }
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        self.inspect(actor, identity)
    }
    fn resolve(
        &self,
        identity: &OwnerIdentity,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if !identity.matches(&owner()) {
            return Err(rom::Error::Denied);
        }
        if self.0.swap(false, Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
        Ok(owner())
    }
    fn attempt(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        prepared: &PreparedAttempt,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        prepared.validate().map_err(|_| rom::Error::Denied)?;
        if identity.matches(actor)
            && prepared.policy().budget_reference() == Some("supervisor-account")
            && prepared.route().maximum_cost() == UsdNanos(10)
        {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        Ok(value.clone())
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_outer_callback_retains_generation_header_after_accepted_body_stalls() {
    journey(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_outer_callback_retains_generation_header_after_accepted_body_stalls() {
    journey(true).await;
}
async fn journey(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rom-openrouter-supervisor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&path).unwrap();
    let open = || -> Arc<dyn Storage> {
        if redb {
            Arc::new(rom_redb::Redb::open(path.join("database")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(path.join("database")).unwrap())
        }
    };
    let server = stalled::StalledBody::new();
    let provider = Arc::new(server.provider());
    let authority = Arc::new(Authority(AtomicBool::new(true)));
    let build = |storage: Arc<dyn Storage>| {
        FlowHost::new(
            service(),
            provider.clone(),
            authority.clone(),
            Arc::new(Validator),
            Arc::new(Clock),
        )
        .unwrap()
        .install(Runtime::builder())
        .unwrap()
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
    };
    let (runtime, client) = build(open());
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "supervisor-account",
                AiBudget::new(&service(), "supervisor-account", UsdNanos(20)).unwrap(),
            )
            .idempotency("supervisor-account-create"),
        )
        .await
        .unwrap();
    let policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["fixture/model".into()],
        Some(ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10))),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("supervisor-account")
    .unwrap();
    let run = client
        .submit(
            &owner(),
            Submission {
                id: "supervisor-run".into(),
                idempotency: "supervisor-submit".into(),
                request: CompletionRequest::new(vec![Message::user("private fixture")], 128)
                    .unwrap(),
                policy,
            },
        )
        .await
        .unwrap();
    let work_runtime = runtime.clone();
    let work = tokio::spawn(async move { work_runtime.process_work(8).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !server.headers_received() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let started = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let original = started.active_attempt().unwrap().clone();
    assert_eq!(server.posts(), 1);
    assert!(server.requests() >= 3);
    tokio::time::timeout(std::time::Duration::from_secs(20), work)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let held = client.view(&owner(), &run).await.unwrap();
    client
        .resume(&owner(), &run, held.revision(), "supervisor-recover")
        .await
        .unwrap();
    let durable = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(durable.active_attempt(), Some(&original));
    let generation = durable
        .attempt_evidence()
        .iter()
        .find(|evidence| evidence.attempt_id() == original.prepared().identity())
        .and_then(|evidence| evidence.generation_id())
        .map(str::to_owned);
    assert_eq!(
        runtime
            .read::<AiBudget>(&service(), "supervisor-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap()
            .reserved(),
        UsdNanos(10)
    );
    assert!(
        client
            .view(&owner(), &run)
            .await
            .unwrap()
            .output()
            .is_none()
    );
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(open());
    let current = client.view(&owner(), &run).await.unwrap();
    client
        .resume(
            &owner(),
            &run,
            current.revision(),
            "supervisor-recover-after-restart",
        )
        .await
        .unwrap();
    assert_eq!(server.posts(), 1);
    assert!(
        client
            .view(&owner(), &run)
            .await
            .unwrap()
            .output()
            .is_none()
    );
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    std::fs::remove_dir_all(path).unwrap();
    assert_eq!(
        generation.as_deref(),
        Some("gen-supervisor"),
        "known accepted headers must survive outer callback cancellation and restart"
    );
}
