//! Pure eligibility; no discovery I/O, reservation, dispatch or shared mutable state.
use super::{CatalogSnapshot, ModelPrice, RouteCursor, RoutingPolicy, RoutingTier, UsdNanos};
use crate::{AiError, AiResult, CompletionRequest};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDecision {
    model: String,
    tier: RoutingTier,
    next_cursor: RouteCursor,
    maximum_cost: UsdNanos,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    eligibility_catalog_identity: Option<String>,
}
impl RouteDecision {
    /// Recompute a continuation ceiling while preserving the original run routing identity.
    /// This checks request/policy coherence; current catalog eligibility and grants remain required.
    pub fn for_request(
        &self,
        policy: &RoutingPolicy,
        previous: &CompletionRequest,
        next: &CompletionRequest,
    ) -> AiResult<Self> {
        policy.validate()?;
        previous.validate()?;
        next.validate()?;
        self.validate_for(policy, previous)?;
        let cap = match self.tier {
            RoutingTier::Free => ModelPrice::free(),
            RoutingTier::Paid => policy.paid_caps().ok_or(AiError::InvalidRequest)?,
        };
        let mut derived = self.clone();
        derived.maximum_cost =
            cap.maximum_cost(next.input_bound()?, u64::from(next.max_output_tokens()))?;
        derived.validate_for(policy, next)?;
        Ok(derived)
    }
    /// Checks policy/request coherence, not catalog capability or budget authorization.
    pub(crate) fn validate_for(
        &self,
        policy: &RoutingPolicy,
        request: &CompletionRequest,
    ) -> AiResult<()> {
        self.next_cursor.validate()?;
        if self
            .eligibility_catalog_identity
            .as_ref()
            .is_some_and(|id| !crate::request::valid_name(id, 128))
        {
            return Err(AiError::InvalidRequest);
        }
        if self.next_cursor.policy_version() != policy.version()
            || self.next_cursor.tier() != self.tier
            || self.next_cursor.is_rejected(&self.model)
        {
            return Err(AiError::Conflict);
        }
        let (candidates, cap) = match self.tier {
            RoutingTier::Free => (policy.free(), ModelPrice::free()),
            RoutingTier::Paid => {
                if policy.budget_reference().is_none() {
                    return Err(AiError::Denied);
                }
                (
                    policy.paid(),
                    policy.paid_caps().ok_or(AiError::InvalidRequest)?,
                )
            }
        };
        let index = self
            .next_cursor
            .next_candidate()
            .checked_sub(1)
            .ok_or(AiError::InvalidRequest)?;
        if candidates.get(index).map(String::as_str) != Some(self.model.as_str()) {
            return Err(AiError::InvalidRequest);
        }
        let required = cap.maximum_cost(
            request.input_bound()?,
            u64::from(request.max_output_tokens()),
        )?;
        if self.maximum_cost != required {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
    /// Catalog used for a fresh continuation eligibility decision; not an authority grant.
    pub fn eligibility_catalog_identity(&self) -> Option<&str> {
        self.eligibility_catalog_identity.as_deref()
    }
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn tier(&self) -> RoutingTier {
        self.tier
    }
    pub fn next_cursor(&self) -> &RouteCursor {
        &self.next_cursor
    }
    pub fn maximum_cost(&self) -> UsdNanos {
        self.maximum_cost
    }
}
fn choose_in_catalog(
    policy: &RoutingPolicy,
    catalog: &CatalogSnapshot,
    cursor: &RouteCursor,
    request: &CompletionRequest,
) -> AiResult<RouteDecision> {
    let input = request.input_bound()?;
    let context = input
        .checked_add(u64::from(request.max_output_tokens()))
        .ok_or(AiError::InvalidRequest)?;
    let mut unsupported = false;
    for (tier, candidates, cap) in [
        (RoutingTier::Free, policy.free(), Some(ModelPrice::free())),
        (RoutingTier::Paid, policy.paid(), policy.paid_caps()),
    ] {
        if cursor.tier() == RoutingTier::Paid && tier == RoutingTier::Free {
            continue;
        }
        let Some(cap) = cap else {
            continue;
        };
        if tier == RoutingTier::Paid
            && !candidates.is_empty()
            && policy.budget_reference().is_none()
        {
            return Err(AiError::Denied);
        }
        let start = if cursor.tier() == tier {
            cursor.next_candidate()
        } else {
            0
        };
        for (index, id) in candidates.iter().enumerate().skip(start) {
            if cursor.is_rejected(id) {
                continue;
            }
            let Some(model) = catalog.models().iter().find(|model| &model.id == id) else {
                continue;
            };
            if model.price.prompt_per_million > cap.prompt_per_million
                || model.price.completion_per_million > cap.completion_per_million
                || model.price.request > cap.request
            {
                continue;
            }
            if !model.input_text
                || !model.output_text
                || model.context_tokens < context
                || (request.schema().is_some() && !model.supports_schema)
                || (!request.tools().is_empty() && !model.supports_tools)
                || (!policy.allowed_providers().is_empty()
                    && !model
                        .providers
                        .iter()
                        .any(|provider| policy.allowed_providers().contains(provider)))
            {
                unsupported = true;
                continue;
            }
            return Ok(RouteDecision {
                model: model.id.clone(),
                tier,
                next_cursor: cursor.advanced(tier, index + 1),
                maximum_cost: cap.maximum_cost(input, u64::from(request.max_output_tokens()))?,
                eligibility_catalog_identity: None,
            });
        }
    }
    Err(if unsupported {
        AiError::UnsupportedCapability
    } else {
        AiError::ProviderUnavailable
    })
}

fn validate_selection_inputs(
    policy: &RoutingPolicy,
    catalog: &CatalogSnapshot,
    cursor: &RouteCursor,
    request: &CompletionRequest,
) -> AiResult<()> {
    policy.validate()?;
    catalog.validate()?;
    cursor.validate()?;
    request.validate()?;
    if cursor.policy_version() != policy.version() {
        return Err(AiError::Conflict);
    }
    Ok(())
}

/// Initial selection preserves the existing exact catalog/cursor binding.
pub fn choose(
    policy: &RoutingPolicy,
    catalog: &CatalogSnapshot,
    cursor: &RouteCursor,
    request: &CompletionRequest,
) -> AiResult<RouteDecision> {
    validate_selection_inputs(policy, catalog, cursor, request)?;
    if cursor.catalog_identity() != catalog.identity() {
        return Err(AiError::Conflict);
    }
    choose_in_catalog(policy, catalog, cursor, request)
}
/// Pure selection only. The host must supply a committed forward cursor, current
/// authority, original deadline, and an account-first reservation before dispatch.
pub fn choose_continuation(
    policy: &RoutingPolicy,
    catalog: &CatalogSnapshot,
    cursor: &RouteCursor,
    request: &CompletionRequest,
) -> AiResult<RouteDecision> {
    validate_selection_inputs(policy, catalog, cursor, request)?;
    if policy.failover() != super::FailoverPolicy::AdvanceOnConfirmedNonacceptance {
        return Err(AiError::Denied);
    }
    let mut decision = choose_in_catalog(policy, catalog, cursor, request)?;
    decision.eligibility_catalog_identity = Some(catalog.identity().to_owned());
    decision.validate_for(policy, request)?;
    Ok(decision)
}
