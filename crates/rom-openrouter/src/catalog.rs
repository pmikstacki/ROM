//! Native model catalog; global records do not certify provider membership.
use rom_ai::{AiError, AiResult, CatalogModel, CatalogSnapshot, ModelPrice};
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) struct CachedSnapshot {
    pub(crate) snapshot: CatalogSnapshot,
    pub(crate) expires: std::time::Instant,
}
#[derive(Deserialize)]
struct Catalog {
    data: Vec<Model>,
}
#[derive(Deserialize)]
struct Model {
    id: String,
    context_length: u64,
    architecture: Architecture,
    #[serde(default)]
    supported_parameters: Vec<String>,
    pricing: Pricing,
}
#[derive(Deserialize)]
struct Architecture {
    input_modalities: Vec<String>,
    output_modalities: Vec<String>,
}
#[derive(Deserialize)]
struct Pricing {
    prompt: String,
    completion: String,
    #[serde(default = "zero")]
    request: String,
}
fn zero() -> String {
    "0".into()
}
static NEXT_SNAPSHOT: AtomicU64 = AtomicU64::new(1);
pub(crate) fn valid_model_id(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 256
        && model.contains('/')
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:/".contains(&byte))
        && model
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}
pub(crate) fn decode(bytes: &[u8]) -> AiResult<CatalogSnapshot> {
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|_| AiError::InvalidOutput)?;
    if catalog.data.len() > 4096 {
        return Err(AiError::InvalidOutput);
    }
    let mut models = Vec::with_capacity(catalog.data.len());
    for model in catalog.data {
        if model.supported_parameters.len() > 128
            || model.architecture.input_modalities.len() > 16
            || model.architecture.output_modalities.len() > 16
        {
            return Err(AiError::InvalidOutput);
        }
        models.push(CatalogModel {
            id: model.id,
            input_text: model
                .architecture
                .input_modalities
                .iter()
                .any(|value| value == "text"),
            output_text: model
                .architecture
                .output_modalities
                .iter()
                .any(|value| value == "text"),
            context_tokens: model.context_length,
            supports_schema: model
                .supported_parameters
                .iter()
                .any(|value| value == "structured_outputs"),
            supports_tools: model
                .supported_parameters
                .iter()
                .any(|value| value == "tools"),
            price: ModelPrice::new(
                crate::money::scaled(&model.pricing.prompt, 15)?,
                crate::money::scaled(&model.pricing.completion, 15)?,
                crate::money::scaled(&model.pricing.request, 9)?,
            ),
            providers: Vec::new(),
        });
    }
    CatalogSnapshot::new(identity()?, models).map_err(|_| AiError::InvalidOutput)
}
pub(crate) fn identity() -> AiResult<String> {
    let ordinal = NEXT_SNAPSHOT
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| AiError::Closed)?;
    Ok(format!("openrouter:{}:{ordinal}", crate::errors::now_ms()?))
}
