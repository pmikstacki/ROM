//! Trusted host composition; serialized identities never create actor credentials.
use super::{
    AiBudget, FLOW_TICKS, FlowTick,
    builder::FlowBuilder,
    codec,
    resource::{AiRun, OwnerIdentity, RunState},
};
use crate::{
    AiClock, AiError, AiResult, CompletionRequest, FlowObserver, OutputValidator, Provider,
    RoutingPolicy, request::valid_name,
};
use std::{
    fmt,
    sync::{Arc, OnceLock, Weak, atomic::AtomicU64},
};

pub(crate) const FREE_ACCOUNT: &str = "rom-ai-free-v1";
pub struct Submission {
    pub id: String,
    pub idempotency: String,
    pub request: CompletionRequest,
    pub policy: RoutingPolicy,
}
impl Submission {
    pub fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.id, 64) || !valid_name(&self.idempotency, 128) {
            return Err(AiError::InvalidRequest);
        }
        self.request.validate()?;
        self.policy.validate()
    }
}
impl fmt::Debug for Submission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Submission { .. }")
    }
}
/// Host policy callbacks must be bounded. `resolve` reads current trusted identity state.
pub trait FlowAuthority: Send + Sync {
    fn submit(&self, actor: &rom::Actor, submission: &Submission) -> AiResult<OwnerIdentity>;
    fn inspect(&self, actor: &rom::Actor, owner: &OwnerIdentity) -> AiResult<()>;
    fn cancel(&self, actor: &rom::Actor, owner: &OwnerIdentity) -> AiResult<()>;
    fn resolve(
        &self,
        owner: &OwnerIdentity,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<rom::Actor>;
    fn attempt(
        &self,
        actor: &rom::Actor,
        owner: &OwnerIdentity,
        prepared: &crate::PreparedAttempt,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()>;
    /// Re-establish a trusted tool actor from current authority; default denies every tool.
    /// Neither a native call nor a persisted actor identity supplies credentials.
    fn tool_actor(
        &self,
        _actor: &rom::Actor,
        _owner: &OwnerIdentity,
        _call: &crate::ToolCall,
        _reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<rom::Actor> {
        Err(rom::Error::Denied)
    }
}
pub(crate) struct HostState {
    pub service: rom::Actor,
    pub provider: Arc<dyn Provider>,
    pub authority: Arc<dyn FlowAuthority>,
    pub validator: Arc<dyn OutputValidator>,
    pub clock: Arc<dyn AiClock>,
    pub observer: Option<Arc<dyn FlowObserver>>,
    pub runtime: OnceLock<Weak<rom::Runtime>>,
    pub provider_permits: Arc<tokio::sync::Semaphore>,
    pub lookup_admission: Arc<tokio::sync::Semaphore>,
    pub next_nonce: AtomicU64,
    pub tools: Option<Arc<crate::ToolRegistry>>,
    pub read_activity: crate::tools::attempt::ReadActivity,
}
impl HostState {
    pub fn runtime(&self) -> AiResult<Arc<rom::Runtime>> {
        self.runtime
            .get()
            .and_then(Weak::upgrade)
            .ok_or(AiError::Closed)
    }
}
pub struct FlowHost {
    state: HostState,
}
impl FlowHost {
    pub fn new(
        service: rom::Actor,
        provider: Arc<dyn Provider>,
        authority: Arc<dyn FlowAuthority>,
        validator: Arc<dyn OutputValidator>,
        clock: Arc<dyn AiClock>,
    ) -> AiResult<Self> {
        OwnerIdentity::from_actor(&service)?.validate_service()?;
        Ok(Self {
            state: HostState {
                service,
                provider,
                authority,
                validator,
                clock,
                observer: None,
                runtime: OnceLock::new(),
                provider_permits: Arc::new(tokio::sync::Semaphore::new(0)),
                lookup_admission: Arc::new(tokio::sync::Semaphore::new(8)),
                next_nonce: AtomicU64::new(1),
                tools: None,
                read_activity: crate::tools::attempt::ReadActivity::default(),
            },
        })
    }
    pub fn observer(mut self, observer: Arc<dyn FlowObserver>) -> Self {
        self.state.observer = Some(observer);
        self
    }
    /// Freeze host-selected typed tools before Runtime construction.
    pub fn tools(mut self, tools: crate::ToolRegistry) -> AiResult<Self> {
        if self.state.tools.is_some() {
            return Err(AiError::Conflict);
        }
        self.state.tools = Some(Arc::new(tools));
        Ok(self)
    }
    pub fn install(self, builder: rom::Builder) -> AiResult<FlowBuilder> {
        let state = Arc::new(self.state);
        let callback = state.clone();
        let builder = builder
            .limits(rom::Limits {
                command_bytes: 1024 * 1024,
                ..Default::default()
            })
            .delivery_timeout(std::time::Duration::from_secs(20))
            .reaction_limits(rom::ReactionLimits {
                lease_seconds: 30,
                ..Default::default()
            })
            .resource(
                AiRun::definition_for(&state.service)?
                    .action(ENQUEUE_FLOW)
                    .action(super::operation::BEGIN_OPERATION)
                    .action(super::operation::FINISH_OPERATION),
            )
            .resource(AiBudget::definition_for(&state.service)?)
            .reaction(rom::Reaction::new(
                "ai_initial_tick",
                1,
                state.service.clone(),
                ENQUEUE_FLOW,
                initial_tick,
            ))
            .channel_with(
                FLOW_TICKS
                    .delivery_profile(rom::DeliveryProfile::ReconcileBeforeRetry)
                    .verifier(|_| async { rom::DeliveryVerification::Unresolved }),
                state.service.clone(),
                move |delivery| super::worker::deliver(callback.clone(), delivery),
            );
        Ok(FlowBuilder {
            builder,
            state,
            limits: rom::Limits {
                command_bytes: 1024 * 1024,
                ..Default::default()
            },
        })
    }
}
impl fmt::Debug for FlowHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FlowHost { .. }")
    }
}
const ENQUEUE_FLOW: rom::Action<AiRun, FlowTick> =
    rom::Action::new("ai_enqueue_flow", |run, tick| {
        let mut record = run.record().map_err(codec::rom_error)?;
        tick.validate().map_err(codec::rom_error)?;
        if record.state() != &RunState::Queued
            || record.queue_admitted
            || tick.run_id != record.run_id()
            || tick.sequence != 1
        {
            return Err(rom::Error::Conflict);
        }
        record.queue_admitted = true;
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![FLOW_TICKS.intent(tick)])
    });
fn initial_tick(snapshot: &rom::Snapshot<AiRun>) -> rom::Result<Vec<rom::Target<FlowTick>>> {
    let Some(value) = &snapshot.value else {
        return Ok(vec![]);
    };
    let record = value.record().map_err(codec::rom_error)?;
    if snapshot.revision != 1 || record.state() != &RunState::Queued || record.checkpoint() != 0 {
        return Ok(vec![]);
    }
    Ok(vec![rom::Target::new(
        record.run_id(),
        FlowTick::new(record.run_id(), 1).map_err(codec::rom_error)?,
    )])
}
