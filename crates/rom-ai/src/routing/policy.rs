//! Exact rate units, capability catalog and finite policy envelopes.
use crate::{AiError, AiResult, request::valid_name};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UsdNanos(pub u64);
impl UsdNanos {
    /// Parse plain decimal USD; round any sub-nanodollar remainder up.
    pub fn from_usd(decimal: &str) -> AiResult<Self> {
        if decimal.len() > 64 {
            return Err(AiError::InvalidRequest);
        }
        let (whole, fraction) = decimal.split_once('.').unwrap_or((decimal, ""));
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || (decimal.contains('.') && fraction.is_empty())
        {
            return Err(AiError::InvalidRequest);
        }
        let whole: u128 = whole.parse().map_err(|_| AiError::InvalidRequest)?;
        let mut result = whole
            .checked_mul(1_000_000_000)
            .ok_or(AiError::InvalidRequest)?;
        let mut fraction_nanos = 0_u128;
        for (index, digit) in fraction.bytes().take(9).enumerate() {
            fraction_nanos += u128::from(digit - b'0') * 10_u128.pow(8 - index as u32);
        }
        result = result
            .checked_add(fraction_nanos)
            .ok_or(AiError::InvalidRequest)?;
        if fraction.bytes().skip(9).any(|b| b != b'0') {
            result = result.checked_add(1).ok_or(AiError::InvalidRequest)?;
        }
        Ok(Self(
            u64::try_from(result).map_err(|_| AiError::InvalidRequest)?,
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPrice {
    pub prompt_per_million: UsdNanos,
    pub completion_per_million: UsdNanos,
    pub request: UsdNanos,
}
impl ModelPrice {
    pub const fn new(
        prompt_per_million: UsdNanos,
        completion_per_million: UsdNanos,
        request: UsdNanos,
    ) -> Self {
        Self {
            prompt_per_million,
            completion_per_million,
            request,
        }
    }
    pub const fn free() -> Self {
        Self::new(UsdNanos(0), UsdNanos(0), UsdNanos(0))
    }
    pub fn maximum_cost(self, input: u64, output: u64) -> AiResult<UsdNanos> {
        let prompt = u128::from(self.prompt_per_million.0)
            .checked_mul(u128::from(input))
            .ok_or(AiError::InvalidRequest)?;
        let completion = u128::from(self.completion_per_million.0)
            .checked_mul(u128::from(output))
            .ok_or(AiError::InvalidRequest)?;
        let tokens = prompt
            .checked_add(completion)
            .ok_or(AiError::InvalidRequest)?;
        let total = tokens
            .div_ceil(1_000_000)
            .checked_add(u128::from(self.request.0))
            .ok_or(AiError::InvalidRequest)?;
        Ok(UsdNanos(
            u64::try_from(total).map_err(|_| AiError::InvalidRequest)?,
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunLimits {
    pub generation_attempts: u32,
    pub tool_calls: u32,
    pub ticks: u32,
    pub age_seconds: u64,
}
impl Default for RunLimits {
    fn default() -> Self {
        Self {
            generation_attempts: 8,
            tool_calls: 16,
            ticks: 32,
            age_seconds: 300,
        }
    }
}
impl RunLimits {
    fn validate(&self) -> AiResult<()> {
        if !(1..=8).contains(&self.generation_attempts)
            || self.tool_calls > 16
            || !(1..=32).contains(&self.ticks)
            || !(1..=300).contains(&self.age_seconds)
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogModel {
    pub id: String,
    pub input_text: bool,
    pub output_text: bool,
    pub context_tokens: u64,
    pub supports_schema: bool,
    pub supports_tools: bool,
    pub price: ModelPrice,
    pub providers: Vec<String>,
}
impl CatalogModel {
    pub fn text(
        id: impl Into<String>,
        context_tokens: u64,
        supports_schema: bool,
        supports_tools: bool,
        price: ModelPrice,
    ) -> Self {
        Self {
            id: id.into(),
            input_text: true,
            output_text: true,
            context_tokens,
            supports_schema,
            supports_tools,
            price,
            providers: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshot {
    identity: String,
    models: Vec<CatalogModel>,
}
impl CatalogSnapshot {
    pub fn new(identity: impl Into<String>, models: Vec<CatalogModel>) -> AiResult<Self> {
        let value = Self {
            identity: identity.into(),
            models,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn models(&self) -> &[CatalogModel] {
        &self.models
    }
    pub fn validate(&self) -> AiResult<()> {
        let mut ids = std::collections::BTreeSet::new();
        if !valid_name(&self.identity, 128) || self.models.len() > 4096 {
            return Err(AiError::InvalidRequest);
        }
        for model in &self.models {
            if !valid_name(&model.id, 256)
                || !ids.insert(&model.id)
                || model.providers.len() > 64
                || model.providers.iter().any(|id| !valid_name(id, 128))
            {
                return Err(AiError::InvalidRequest);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicy {
    version: u32,
    free: Vec<String>,
    paid: Vec<String>,
    paid_caps: Option<ModelPrice>,
    limits: RunLimits,
    allowed_providers: Vec<String>,
    budget_reference: Option<String>,
    #[serde(default, skip_serializing_if = "super::FailoverPolicy::is_same_model")]
    failover: super::FailoverPolicy,
}
impl RoutingPolicy {
    pub fn new(
        version: u32,
        free: Vec<String>,
        paid: Vec<String>,
        paid_caps: Option<ModelPrice>,
        limits: RunLimits,
    ) -> AiResult<Self> {
        let value = Self {
            version,
            free,
            paid,
            paid_caps,
            limits,
            allowed_providers: Vec::new(),
            budget_reference: None,
            failover: super::FailoverPolicy::SameModel,
        };
        value.validate()?;
        Ok(value)
    }
    /// Explicit run-frozen opt-in. This policy is not proof of provider nonacceptance.
    pub fn with_failover(mut self, failover: super::FailoverPolicy) -> AiResult<Self> {
        self.failover = failover;
        self.validate()?;
        Ok(self)
    }
    pub fn failover(&self) -> super::FailoverPolicy {
        self.failover
    }
    pub fn with_budget_reference(mut self, reference: impl Into<String>) -> AiResult<Self> {
        self.budget_reference = Some(reference.into());
        self.validate()?;
        Ok(self)
    }
    pub fn budget_reference(&self) -> Option<&str> {
        self.budget_reference.as_deref()
    }
    pub fn with_allowed_providers(mut self, providers: Vec<String>) -> AiResult<Self> {
        self.allowed_providers = providers;
        self.validate()?;
        Ok(self)
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn free(&self) -> &[String] {
        &self.free
    }
    pub fn paid(&self) -> &[String] {
        &self.paid
    }
    pub fn paid_caps(&self) -> Option<ModelPrice> {
        self.paid_caps
    }
    pub fn limits(&self) -> &RunLimits {
        &self.limits
    }
    pub fn allowed_providers(&self) -> &[String] {
        &self.allowed_providers
    }
    pub fn validate(&self) -> AiResult<()> {
        self.limits.validate()?;
        let mut seen = std::collections::BTreeSet::new();
        if self.version == 0
            || self.free.len() + self.paid.len() > 256
            || self.allowed_providers.len() > 64
            || self
                .free
                .iter()
                .chain(&self.paid)
                .any(|id| !valid_name(id, 256) || !seen.insert(id))
            || self.allowed_providers.iter().any(|id| !valid_name(id, 128))
            || self
                .budget_reference
                .as_ref()
                .is_some_and(|id| !valid_name(id, 160))
            || (!self.paid.is_empty() && self.paid_caps.is_none())
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
