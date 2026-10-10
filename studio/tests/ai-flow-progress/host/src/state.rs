//! Construct one real Runtime, install flow tools, then expose only owner projections.
use crate::{authority::{Authority, Grants, owner, service}, domain, gate::ReadGate, provider::{Adapter, Validator}};
use rom::{Runtime, Storage};
use rom_ai::{AiClock, AiError, AiResult, CompletionRequest, Message, RoutingPolicy, RunLimits, flow::{AiRun, FlowAuthority, FlowClient, FlowHost, RunHandle, RunView, Submission}};
use std::{path::Path, sync::{Arc, atomic::{AtomicU64, Ordering}}, time::{SystemTime, UNIX_EPOCH}};
pub struct Clock;
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)) }
}
impl rom::Clock for Clock {
    fn now(&self) -> u64 { self.now_unix_ms() / 1000 }
}
#[derive(Clone)]
pub struct State {
    pub runtime: Runtime,
    pub client: Arc<FlowClient>,
    pub grants: Arc<Grants>,
    pub gate: Arc<ReadGate>,
    pub provider: Arc<Adapter>,
    pub publication: bool,
    pub key: Arc<String>,
}
impl State {
    pub async fn build(adapter: &str, path: &Path, publication: bool, mode: &str, key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let storage: Arc<dyn Storage> = match adapter {
            "sqlite" => Arc::new(rom_sqlite::Sqlite::open(path)?),
            "redb" => Arc::new(rom_redb::Redb::open(path)?),
            _ => return Err("unsupported adapter".into()),
        };
        let grants = Arc::new(Grants::allowed());
        let gate = Arc::new(ReadGate::new());
        let provider = Arc::new(Adapter { publication, unknown: mode == "unknown", calls: AtomicU64::new(0) });
        let registry = domain::tools(publication, gate.clone())?;
        let request = CompletionRequest::new(vec![Message::user(if publication { "Publish the captured immutable edition" } else { "Classify the authorized ticket" })], 128)?.with_tools(registry.descriptors())?;
        let (runtime, client) = FlowHost::new(service(), provider.clone(), Arc::new(Authority(grants.clone())), Arc::new(Validator), Arc::new(Clock))?
            .tools(registry)?.install(domain::builder(publication).clock(Arc::new(Clock)))?
            .build(storage, Runtime::shared_cpu_pool(2)?)?;
        domain::seed(&runtime, publication).await?;
        client.submit(&owner(), Submission {
            id: "browser-run".into(), idempotency: "browser-original-submission".into(), request,
            policy: RoutingPolicy::new(1, vec!["fixture/free".into()], vec![], None, RunLimits { generation_attempts: if mode == "budget" { 1 } else { 8 }, ..Default::default() })?,
        }).await?;
        Ok(Self { runtime, client: Arc::new(client), grants, gate, provider, publication, key: Arc::new(key) })
    }
    pub async fn view(&self, durable_only: bool) -> AiResult<RunView> {
        let run = RunHandle::new("browser-run")?;
        let authorized = self.client.view(&owner(), &run).await?;
        if !durable_only { return Ok(authorized); }
        // Never return raw AiRun. Current FlowClient checks precede the pure advisory projection.
        let snapshot = self.runtime.read::<AiRun>(&service(), &run.0).await.map_err(|_| AiError::Storage)?;
        let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
        RunView::project(&record, snapshot.revision, &owner(), |actor, identity| {
            Authority(self.grants.clone()).inspect(actor, identity)?;
            if !self.grants.tool.load(Ordering::SeqCst) || !self.grants.row.load(Ordering::SeqCst) { return Err(AiError::Denied); }
            Ok(())
        })
    }
    pub async fn inspect(&self) -> AiResult<serde_json::Value> {
        let view = self.view(false).await?;
        let domain = domain::inspect(&self.runtime, self.publication).await.map_err(|_| AiError::Denied)?;
        Ok(serde_json::json!({ "view": view, "domain": domain, "provider_calls": self.provider.calls.load(Ordering::SeqCst), "read_calls": self.gate.calls.load(Ordering::SeqCst), "physical_started": self.gate.started.load(Ordering::SeqCst) }))
    }
}
