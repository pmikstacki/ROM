//! Optional bounded AI contracts. No HTTP client, database driver or scheduler.
mod clock;
mod error;
mod execution;
pub mod flow;
mod outcome;
mod provider;
mod reconciliation;
mod request;
mod response;
pub mod routing;
mod telemetry;
pub mod tools;

pub use clock::{AiClock, Deadline};
pub use error::{AiError, AiResult};
pub use execution::ExecutionDeadline;
pub use outcome::DispatchOutcome;
pub use provider::{AiFuture, OutputValidator, PreparedAttempt, Provider};
pub use reconciliation::ReconciliationObservation;
pub use request::{CompletionRequest, Message, MessageRole, OutputSchema, ToolDescriptor};
pub use response::{AttemptEvidence, Completion, Reconciliation, ToolCall, Usage};
pub use routing::{
    CatalogModel, CatalogSnapshot, FailoverPolicy, ModelPrice, RouteCursor, RouteDecision,
    RoutingPolicy, RoutingTier, RunLimits, UsdNanos, choose, choose_continuation,
};
pub use telemetry::{FlowObservation, FlowObserver, ObservationKind, observe_safely};

pub use tools::{PreparedToolAction, ReadContext, ToolRegistry};
