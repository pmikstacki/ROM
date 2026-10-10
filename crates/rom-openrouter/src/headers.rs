//! Bounded header knowledge; ambiguity cannot prove provider nonacceptance.
use reqwest::header::{HeaderMap, RETRY_AFTER};

pub(crate) struct Knowledge {
    pub(crate) generation: Option<String>,
    pub(crate) retry: Option<String>,
    pub(crate) ambiguous: bool,
}
pub(crate) fn capture(headers: &HeaderMap) -> Knowledge {
    let generations: Vec<_> = headers.get_all("x-generation-id").iter().collect();
    let retries: Vec<_> = headers.get_all(RETRY_AFTER).iter().collect();
    let generation = generations
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| crate::transport::valid_generation(value))
        .map(str::to_owned);
    let retry = retries
        .first()
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let ambiguous = generations.len() > 1
        || generations
            .iter()
            .any(|value| !value.to_str().is_ok_and(crate::transport::valid_generation))
        || retries.len() > 1
        || retries.iter().any(|value| value.to_str().is_err());
    Knowledge {
        generation,
        retry,
        ambiguous,
    }
}
