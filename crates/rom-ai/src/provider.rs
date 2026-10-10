//! Object-safe adapter contracts contain no client implementation or storage driver.
use crate::{
    AiError, AiResult, AttemptEvidence, CatalogSnapshot, Completion, CompletionRequest, Deadline,
    Reconciliation, RouteDecision, RoutingPolicy, Usage, request::valid_name,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, future::Future, pin::Pin};

pub type AiFuture<'a, T> = Pin<Box<dyn Future<Output = AiResult<T>> + Send + 'a>>;
pub trait Provider: Send + Sync {
    /// Pure bounded adapter validation before reservation and dispatch.
    /// Implementations must not perform provider I/O or treat validation as an authority grant.
    /// The compatible default cannot certify provider-specific framing or capabilities.
    fn preflight<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, ()> {
        Box::pin(async move { attempt.validate() })
    }
    fn catalog<'a>(&'a self, deadline: Deadline) -> AiFuture<'a, CatalogSnapshot>;
    /// Compatible demand-aware discovery. Catalog data never grants budget authority.
    fn catalog_for<'a>(
        &'a self,
        request: &'a CompletionRequest,
        policy: &'a RoutingPolicy,
        deadline: Deadline,
    ) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async move {
            request.validate()?;
            policy.validate()?;
            self.catalog(deadline).await
        })
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion>;
    /// Compatible observation path. Legacy rate limits retain their trusted nonacceptance meaning.
    fn complete_observed<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
    ) -> AiFuture<'a, crate::outcome::DispatchOutcome> {
        Box::pin(async move {
            use crate::outcome::DispatchOutcome;
            let outcome = self.complete(attempt).await;
            Ok(match outcome {
                Ok(completion) => DispatchOutcome::Completed(completion),
                Err(AiError::RateLimited { retry_after_ms }) => DispatchOutcome::NotAccepted {
                    evidence: AttemptEvidence::new(attempt.identity(), None, None)?,
                    retry_after_ms,
                },
                Err(cause) => DispatchOutcome::Uncertain {
                    evidence: AttemptEvidence::new(attempt.identity(), None, None)?,
                    usage: Usage::default(),
                    cause,
                },
            })
        })
    }
    /// Compatible execution bound; legacy implementations still need a caller timeout.
    fn complete_observed_within<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
        execution: crate::ExecutionDeadline,
    ) -> AiFuture<'a, crate::DispatchOutcome> {
        Box::pin(async move {
            execution.remaining_ms()?;
            attempt.validate()?;
            self.complete_observed(attempt).await
        })
    }
    fn reconcile<'a>(&'a self, attempt: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation>;
    /// Compatible observation path; metadata-only knowledge cannot claim completed output.
    fn reconcile_observed<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
        evidence: &'a AttemptEvidence,
    ) -> AiFuture<'a, crate::ReconciliationObservation> {
        Box::pin(async move {
            prepared.validate()?;
            evidence.validate()?;
            if evidence.attempt_id() != prepared.identity() {
                return Err(AiError::InvalidOutput);
            }
            let observed =
                crate::ReconciliationObservation::Resolved(self.reconcile(evidence).await?);
            observed.validate_for(prepared, evidence)?;
            Ok(observed)
        })
    }
    /// The same lookup budget includes prior authorization and provider-permit waits.
    fn reconcile_observed_within<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
        evidence: &'a AttemptEvidence,
        execution: crate::ExecutionDeadline,
    ) -> AiFuture<'a, crate::ReconciliationObservation> {
        Box::pin(async move {
            execution.remaining_ms()?;
            self.reconcile_observed(prepared, evidence).await
        })
    }
}
pub trait OutputValidator: Send + Sync {
    fn validate(&self, value: &Value) -> AiResult<Value>;
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAttempt {
    identity: String,
    request: CompletionRequest,
    policy: RoutingPolicy,
    route: RouteDecision,
    deadline: Deadline,
}
impl PreparedAttempt {
    pub fn new(
        identity: impl Into<String>,
        request: CompletionRequest,
        policy: RoutingPolicy,
        route: RouteDecision,
        deadline: Deadline,
    ) -> AiResult<Self> {
        let value = Self {
            identity: identity.into(),
            request,
            policy,
            route,
            deadline,
        };
        value.validate()?;
        Ok(value)
    }
    /// Check neutral contract coherence after decoding; this does not authorize dispatch.
    pub fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.identity, 160) {
            return Err(AiError::InvalidRequest);
        }
        self.request.validate()?;
        self.policy.validate()?;
        if self.deadline.remaining_ms() == 0
            || self.deadline.remaining_ms() > self.deadline.expires_at_unix_ms()
            || self.deadline.remaining_ms() > self.policy.limits().age_seconds * 1000
        {
            return Err(AiError::InvalidRequest);
        }
        self.route.validate_for(&self.policy, &self.request)
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn request(&self) -> &CompletionRequest {
        &self.request
    }
    pub fn policy(&self) -> &RoutingPolicy {
        &self.policy
    }
    pub fn route(&self) -> &RouteDecision {
        &self.route
    }
    pub fn deadline(&self) -> Deadline {
        self.deadline
    }
}
impl fmt::Debug for PreparedAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparedAttempt { .. }")
    }
}
