//! Raw endpoint facts are cached; eligibility is always recomputed for the current request and policy.
use rom_ai::{
    AiError, AiResult, CatalogModel, CompletionRequest, Deadline, ModelPrice, RoutingPolicy,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Deserialize)]
pub(crate) struct Endpoint {
    tag: String,
    model_id: String,
    context_length: u64,
    max_prompt_tokens: Option<u64>,
    max_completion_tokens: Option<u64>,
    supported_parameters: Vec<String>,
    pricing: Pricing,
}
#[derive(Clone, Deserialize)]
struct Pricing {
    prompt: String,
    completion: String,
    request: String,
}
#[derive(Deserialize)]
struct Envelope {
    data: ModelEndpoints,
}
#[derive(Deserialize)]
struct ModelEndpoints {
    id: String,
    endpoints: Vec<Endpoint>,
}
struct Cached {
    records: Arc<Vec<Endpoint>>,
    expires: Instant,
}
type Slot = Arc<tokio::sync::Mutex<Option<Cached>>>;
pub(crate) struct Cache {
    entries: Mutex<BTreeMap<String, Slot>>,
    permits: tokio::sync::Semaphore,
}
impl Cache {
    pub(crate) fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            permits: tokio::sync::Semaphore::new(4),
        }
    }
    pub(crate) async fn get(
        &self,
        adapter: &crate::OpenRouter,
        model: &str,
        deadline: Deadline,
    ) -> AiResult<Arc<Vec<Endpoint>>> {
        let slot = {
            let mut entries = self.entries.lock().map_err(|_| AiError::Closed)?;
            if let Some(slot) = entries.get(model) {
                slot.clone()
            } else {
                if entries.len() >= 256 {
                    let old = entries
                        .iter()
                        .find(|(_, slot)| Arc::strong_count(slot) == 1)
                        .map(|(id, _)| id.clone())
                        .ok_or(AiError::BudgetExhausted)?;
                    entries.remove(&old);
                }
                let slot = Arc::new(tokio::sync::Mutex::new(None));
                entries.insert(model.into(), slot.clone());
                slot
            }
        };
        let remaining = deadline
            .expires_at_unix_ms()
            .checked_sub(crate::errors::now_ms()?)
            .filter(|remaining| *remaining > 0)
            .ok_or(AiError::DeadlineExceeded)?
            .min(deadline.remaining_ms());
        let until = tokio::time::Instant::now()
            .checked_add(Duration::from_millis(remaining))
            .ok_or(AiError::DeadlineExceeded)?;
        let mut cached = tokio::time::timeout_at(until, slot.lock())
            .await
            .map_err(|_| AiError::DeadlineExceeded)?;
        if let Some(cached) = cached.as_ref()
            && cached.expires > Instant::now()
        {
            return Ok(cached.records.clone());
        }
        let _permit = tokio::time::timeout_at(until, self.permits.acquire())
            .await
            .map_err(|_| AiError::DeadlineExceeded)?
            .map_err(|_| AiError::Closed)?;
        let reply = adapter
            .exchange(
                deadline,
                &format!("models/{model}/endpoints"),
                None,
                256 * 1024,
            )
            .await?;
        if reply.status != 200 {
            return Err(crate::errors::http_error(reply.status));
        }
        let envelope: Envelope =
            serde_json::from_slice(&reply.body?).map_err(|_| AiError::InvalidOutput)?;
        if envelope.data.id != model || envelope.data.endpoints.len() > 64 {
            return Err(AiError::InvalidOutput);
        }
        for endpoint in &envelope.data.endpoints {
            if endpoint.model_id != model
                || endpoint.tag.is_empty()
                || endpoint.tag.len() > 128
                || !endpoint
                    .tag
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:/".contains(&byte))
                || endpoint.supported_parameters.len() > 128
                || endpoint
                    .supported_parameters
                    .iter()
                    .any(|parameter| parameter.len() > 128)
            {
                return Err(AiError::InvalidOutput);
            }
            endpoint.price()?;
        }
        let records = Arc::new(envelope.data.endpoints);
        *cached = Some(Cached {
            records: records.clone(),
            expires: Instant::now()
                .checked_add(Duration::from_secs(adapter.config.catalog_ttl_seconds))
                .ok_or(AiError::Closed)?,
        });
        Ok(records)
    }
}
impl Endpoint {
    fn price(&self) -> AiResult<ModelPrice> {
        Ok(ModelPrice::new(
            crate::money::scaled(&self.pricing.prompt, 15)?,
            crate::money::scaled(&self.pricing.completion, 15)?,
            crate::money::scaled(&self.pricing.request, 9)?,
        ))
    }
    fn supports(&self, parameter: &str) -> bool {
        self.supported_parameters
            .iter()
            .any(|supported| supported == parameter)
    }
    fn provider_matches(&self, allowed: &str) -> bool {
        self.tag == allowed
            || (!allowed.contains('/') && self.tag.split('/').next() == Some(allowed))
    }
}
pub(crate) fn providers(records: &[Endpoint]) -> AiResult<Vec<String>> {
    let mut tags = std::collections::BTreeSet::new();
    for record in records {
        tags.insert(record.tag.clone());
        if let Some((base, _)) = record.tag.split_once('/') {
            tags.insert(base.into());
        }
    }
    if tags.len() > 64 {
        return Err(AiError::InvalidOutput);
    }
    Ok(tags.into_iter().collect())
}
pub(crate) fn eligible(
    model: &CatalogModel,
    records: &[Endpoint],
    request: &CompletionRequest,
    policy: &RoutingPolicy,
) -> AiResult<Option<CatalogModel>> {
    let caps = if policy.free().contains(&model.id) {
        ModelPrice::free()
    } else if policy.paid().contains(&model.id) {
        policy.paid_caps().ok_or(AiError::InvalidRequest)?
    } else {
        return Ok(None);
    };
    let input = request.input_bound()?;
    let output = u64::from(request.max_output_tokens());
    let context = input.checked_add(output).ok_or(AiError::InvalidRequest)?;
    let mut valid = Vec::new();
    for endpoint in records {
        let price = endpoint.price()?;
        if model.input_text
            && model.output_text
            && endpoint.context_length >= context
            && endpoint
                .max_prompt_tokens
                .is_none_or(|maximum| maximum >= input)
            && endpoint
                .max_completion_tokens
                .is_none_or(|maximum| maximum >= output)
            && endpoint.supports("max_tokens")
            && (request.schema().is_none() || endpoint.supports("structured_outputs"))
            && (request.tools().is_empty() || endpoint.supports("tools"))
            && (policy.allowed_providers().is_empty()
                || policy
                    .allowed_providers()
                    .iter()
                    .any(|allowed| endpoint.provider_matches(allowed)))
            && price.prompt_per_million <= caps.prompt_per_million
            && price.completion_per_million <= caps.completion_per_million
            && price.request <= caps.request
        {
            valid.push(endpoint.clone());
        }
    }
    if valid.is_empty() {
        return Ok(None);
    }
    let mut candidate = model.clone();
    candidate.providers = providers(&valid)?;
    candidate.context_tokens = valid
        .iter()
        .map(|endpoint| endpoint.context_length)
        .min()
        .unwrap_or(0);
    candidate.supports_schema = valid
        .iter()
        .all(|endpoint| endpoint.supports("structured_outputs"));
    candidate.supports_tools = valid.iter().all(|endpoint| endpoint.supports("tools"));
    candidate.price = ModelPrice::free();
    for endpoint in valid {
        let price = endpoint.price()?;
        candidate.price.prompt_per_million = candidate
            .price
            .prompt_per_million
            .max(price.prompt_per_million);
        candidate.price.completion_per_million = candidate
            .price
            .completion_per_million
            .max(price.completion_per_million);
        candidate.price.request = candidate.price.request.max(price.request);
    }
    Ok(Some(candidate))
}
