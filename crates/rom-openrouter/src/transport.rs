//! Bounded provider transport; no scheduler or receipt store.
use crate::{CredentialSource, OpenRouterConfig};
use rom_ai::{
    AiError, AiFuture, AiResult, AttemptEvidence, CatalogSnapshot, Completion, Deadline,
    DispatchOutcome, PreparedAttempt, Provider, Reconciliation, Usage,
};
use std::{
    fmt,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct OpenRouter {
    pub(crate) config: OpenRouterConfig,
    pub(crate) credentials: Arc<dyn CredentialSource>,
    pub(crate) base: reqwest::Url,
    client: reqwest::Client,
    catalog_cache: Mutex<Option<crate::catalog::CachedSnapshot>>,
    endpoint_models: Vec<String>,
    endpoint_cache: crate::endpoints::Cache,
}
impl OpenRouter {
    pub fn new(config: OpenRouterConfig, credentials: Arc<dyn CredentialSource>) -> AiResult<Self> {
        Self::configured(config, credentials, false)
    }
    #[cfg(feature = "test-support")]
    pub fn for_loopback(
        config: OpenRouterConfig,
        credentials: Arc<dyn CredentialSource>,
    ) -> AiResult<Self> {
        Self::configured(config, credentials, true)
    }
    fn configured(
        config: OpenRouterConfig,
        credentials: Arc<dyn CredentialSource>,
        loopback: bool,
    ) -> AiResult<Self> {
        let base = config.validate(loopback)?;
        let client = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .build()
            .map_err(|_| AiError::Closed)?;
        Ok(Self {
            config,
            credentials,
            base,
            client,
            catalog_cache: Mutex::new(None),
            endpoint_models: Vec::new(),
            endpoint_cache: crate::endpoints::Cache::new(),
        })
    }
    /// Configure bounded discovery identities, not permission to incur model charges.
    pub fn with_endpoint_models(mut self, models: Vec<String>) -> AiResult<Self> {
        let mut names = std::collections::BTreeSet::new();
        if models.len() > 256
            || models.iter().map(String::len).sum::<usize>() > 64 * 1024
            || models
                .iter()
                .any(|model| !crate::catalog::valid_model_id(model) || !names.insert(model))
        {
            return Err(AiError::InvalidRequest);
        }
        self.endpoint_models = models;
        Ok(self)
    }
    async fn observed_completion(
        &self,
        attempt: &PreparedAttempt,
        deadline: Deadline,
    ) -> AiResult<DispatchOutcome> {
        let body = crate::request::encode(attempt)?;
        let reply = self
            .exchange(
                deadline,
                "chat/completions",
                Some(body),
                self.config.response_bytes,
            )
            .await?;
        let mut generation = reply.generation;
        let mut contradictory_identity = false;
        if let Ok(body) = &reply.body
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(body)
            && let Some(id) = value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|id| valid_generation(id))
        {
            if generation.as_deref().is_some_and(|header| header != id) {
                contradictory_identity = true;
            } else {
                generation = Some(id.into());
            }
        }
        let evidence = AttemptEvidence::new(attempt.identity(), None, generation)?;
        if reply.status == 429 {
            return crate::nonacceptance::classify(
                attempt,
                evidence,
                &reply.body,
                reply.retry.as_deref(),
                reply.sse,
                contradictory_identity || reply.ambiguous_headers,
            );
        }
        if contradictory_identity {
            return Ok(DispatchOutcome::Uncertain {
                evidence,
                usage: Usage::default(),
                cause: AiError::InvalidOutput,
            });
        }
        let cause = if reply.status != 200 {
            crate::errors::http_error(reply.status)
        } else if reply.sse {
            AiError::UnknownOutcome
        } else {
            match reply.body {
                Ok(bytes) => match crate::response::decode(&bytes, attempt, evidence.clone()) {
                    Ok(completion) => return Ok(DispatchOutcome::Completed(completion)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        };
        Ok(DispatchOutcome::Uncertain {
            evidence,
            usage: Usage::default(),
            cause,
        })
    }
    async fn observed_reconciliation(
        &self,
        prepared: &PreparedAttempt,
        evidence: &AttemptEvidence,
        deadline: Deadline,
    ) -> AiResult<rom_ai::ReconciliationObservation> {
        prepared.validate()?;
        evidence.validate()?;
        if evidence.attempt_id() != prepared.identity() {
            return Err(AiError::InvalidOutput);
        }
        let Some(bytes) = self.generation_metadata(evidence, deadline).await? else {
            return Ok(rom_ai::ReconciliationObservation::Resolved(
                Reconciliation::Unresolved,
            ));
        };
        crate::reconcile::observed(&bytes, prepared, evidence)
    }
    async fn generation_metadata(
        &self,
        evidence: &AttemptEvidence,
        deadline: Deadline,
    ) -> AiResult<Option<Vec<u8>>> {
        evidence.validate()?;
        let Some(generation) = evidence.generation_id().filter(|id| valid_generation(id)) else {
            return Ok(None);
        };
        let reply = self
            .exchange(
                deadline,
                &format!("generation?id={generation}"),
                None,
                self.config.response_bytes,
            )
            .await?;
        if reply.status == 404 {
            return Ok(None);
        }
        if reply.status != 200 {
            return Err(crate::errors::http_error(reply.status));
        }
        Ok(Some(reply.body?))
    }
    async fn raw_catalog(&self, deadline: Deadline) -> AiResult<CatalogSnapshot> {
        if deadline.remaining_ms() == 0 || deadline.expires_at_unix_ms() <= crate::errors::now_ms()?
        {
            return Err(AiError::DeadlineExceeded);
        }
        if let Some(cache) = self
            .catalog_cache
            .lock()
            .map_err(|_| AiError::Closed)?
            .as_ref()
            && cache.expires > std::time::Instant::now()
        {
            return Ok(cache.snapshot.clone());
        }
        let reply = self
            .exchange(deadline, "models", None, 4 * 1024 * 1024)
            .await?;
        if reply.status != 200 {
            return Err(crate::errors::http_error(reply.status));
        }
        let snapshot = crate::catalog::decode(&reply.body?)?;
        if deadline.expires_at_unix_ms() <= crate::errors::now_ms()? {
            return Err(AiError::DeadlineExceeded);
        }
        let expires = std::time::Instant::now()
            .checked_add(Duration::from_secs(self.config.catalog_ttl_seconds))
            .ok_or(AiError::Closed)?;
        *self.catalog_cache.lock().map_err(|_| AiError::Closed)? =
            Some(crate::catalog::CachedSnapshot {
                snapshot: snapshot.clone(),
                expires,
            });
        Ok(snapshot)
    }
    async fn records(
        &self,
        models: &[String],
        deadline: Deadline,
    ) -> AiResult<std::collections::BTreeMap<String, Arc<Vec<crate::endpoints::Endpoint>>>> {
        let mut records = std::collections::BTreeMap::new();
        for group in models.chunks(4) {
            let fetch = |index: usize| async move {
                match group.get(index) {
                    Some(model) => self
                        .endpoint_cache
                        .get(self, model, deadline)
                        .await
                        .map(|records| Some((model.clone(), records))),
                    None => Ok(None),
                }
            };
            let results = tokio::join!(fetch(0), fetch(1), fetch(2), fetch(3));
            for result in [results.0, results.1, results.2, results.3] {
                if let Some((model, proof)) = result? {
                    records.insert(model, proof);
                }
            }
        }
        if deadline.expires_at_unix_ms() <= crate::errors::now_ms()? {
            return Err(AiError::DeadlineExceeded);
        }
        Ok(records)
    }
    pub(crate) async fn exchange(
        &self,
        deadline: Deadline,
        path: &str,
        body: Option<Vec<u8>>,
        limit: usize,
    ) -> AiResult<Reply> {
        let wall_remaining = deadline
            .expires_at_unix_ms()
            .checked_sub(crate::errors::now_ms()?)
            .filter(|value| *value > 0)
            .ok_or(AiError::DeadlineExceeded)?;
        let until = tokio::time::Instant::now()
            .checked_add(Duration::from_millis(
                deadline.remaining_ms().min(wall_remaining),
            ))
            .ok_or(AiError::DeadlineExceeded)?;
        let token =
            tokio::time::timeout_at(until, self.credentials.resolve(&self.config.credential_ref))
                .await
                .map_err(|_| AiError::DeadlineExceeded)??;
        let mut authorization =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token.0))
                .map_err(|_| AiError::InvalidRequest)?;
        authorization.set_sensitive(true);
        let mut url = self.base.clone();
        let (path, query) = path
            .split_once('?')
            .map_or((path, None), |(path, query)| (path, Some(query)));
        url.set_path(&format!("{}/{path}", self.base.path()));
        url.set_query(query);
        let mut request = self
            .client
            .request(
                if body.is_some() {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::GET
                },
                url,
            )
            .header(reqwest::header::AUTHORIZATION, authorization);
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header("x-openrouter-cache", "false")
                .body(body);
        }
        let mut response = tokio::time::timeout_at(until, request.send())
            .await
            .map_err(|_| AiError::DeadlineExceeded)?
            .map_err(|_| AiError::UnknownOutcome)?;
        if response.headers().len() > 128
            || response
                .headers()
                .iter()
                .map(|(key, value)| key.as_str().len() + value.as_bytes().len())
                .sum::<usize>()
                > 16 * 1024
        {
            return Err(AiError::UnknownOutcome);
        }
        let status = response.status().as_u16();
        let knowledge = crate::headers::capture(response.headers());
        let sse = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));
        let body = tokio::time::timeout_at(until, async {
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| AiError::UnknownOutcome)?
            {
                if bytes
                    .len()
                    .checked_add(chunk.len())
                    .is_none_or(|length| length > limit)
                {
                    return Err(AiError::UnknownOutcome);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
        .await
        .unwrap_or(Err(AiError::DeadlineExceeded));
        Ok(Reply {
            status,
            generation: knowledge.generation,
            retry: knowledge.retry,
            ambiguous_headers: knowledge.ambiguous,
            sse,
            body,
        })
    }
}
pub(crate) fn valid_generation(id: &str) -> bool {
    id.len() <= 128
        && id.starts_with("gen-")
        && id.len() > 4
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
pub(crate) struct Reply {
    pub(crate) status: u16,
    generation: Option<String>,
    retry: Option<String>,
    ambiguous_headers: bool,
    sse: bool,
    pub(crate) body: AiResult<Vec<u8>>,
}
impl Provider for OpenRouter {
    fn preflight<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, ()> {
        Box::pin(async move {
            crate::request::encode(attempt)?;
            Ok(())
        })
    }
    fn catalog<'a>(&'a self, deadline: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async move {
            let snapshot = self.raw_catalog(deadline).await?;
            if self.endpoint_models.is_empty() {
                return Ok(snapshot);
            }
            let proofs = self.records(&self.endpoint_models, deadline).await?;
            let mut models = snapshot.models().to_vec();
            for model in &mut models {
                if let Some(records) = proofs.get(&model.id) {
                    model.providers = crate::endpoints::providers(records)?;
                }
            }
            CatalogSnapshot::new(crate::catalog::identity()?, models)
                .map_err(|_| AiError::InvalidOutput)
        })
    }
    fn catalog_for<'a>(
        &'a self,
        request: &'a rom_ai::CompletionRequest,
        policy: &'a rom_ai::RoutingPolicy,
        deadline: Deadline,
    ) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async move {
            request.validate()?;
            policy.validate()?;
            let snapshot = self.raw_catalog(deadline).await?;
            let models: Vec<_> = policy
                .free()
                .iter()
                .chain(policy.paid())
                .filter(|model| {
                    self.endpoint_models.is_empty() || self.endpoint_models.contains(model)
                })
                .filter(|model| snapshot.models().iter().any(|record| &record.id == *model))
                .cloned()
                .collect();
            if models
                .iter()
                .any(|model| !crate::catalog::valid_model_id(model))
            {
                return Err(AiError::InvalidRequest);
            }
            let proofs = self.records(&models, deadline).await?;
            let mut eligible = Vec::new();
            for model in snapshot.models() {
                if let Some(records) = proofs.get(&model.id)
                    && let Some(model) =
                        crate::endpoints::eligible(model, records, request, policy)?
                {
                    eligible.push(model);
                }
            }
            CatalogSnapshot::new(crate::catalog::identity()?, eligible)
                .map_err(|_| AiError::InvalidOutput)
        })
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            match self.complete_observed(attempt).await? {
                DispatchOutcome::Completed(completion) => Ok(completion),
                DispatchOutcome::NotAccepted { retry_after_ms, .. } => {
                    Err(AiError::RateLimited { retry_after_ms })
                }
                DispatchOutcome::Uncertain { cause, .. } => Err(cause),
            }
        })
    }
    fn complete_observed<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
    ) -> AiFuture<'a, DispatchOutcome> {
        Box::pin(self.observed_completion(attempt, attempt.deadline()))
    }
    fn complete_observed_within<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
        execution: rom_ai::ExecutionDeadline,
    ) -> AiFuture<'a, DispatchOutcome> {
        Box::pin(async move {
            let deadline = crate::execution::deadline(
                &execution,
                Some(attempt.deadline().expires_at_unix_ms()),
            )?;
            self.observed_completion(attempt, deadline).await
        })
    }
    fn reconcile<'a>(&'a self, evidence: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async move {
            let Some(bytes) = self
                .generation_metadata(evidence, crate::reconcile::deadline()?)
                .await?
            else {
                return Ok(Reconciliation::Unresolved);
            };
            crate::reconcile::metadata(&bytes, evidence)
        })
    }
    fn reconcile_observed<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
        evidence: &'a AttemptEvidence,
    ) -> AiFuture<'a, rom_ai::ReconciliationObservation> {
        Box::pin(async move {
            self.observed_reconciliation(prepared, evidence, crate::reconcile::deadline()?)
                .await
        })
    }
    fn reconcile_observed_within<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
        evidence: &'a AttemptEvidence,
        execution: rom_ai::ExecutionDeadline,
    ) -> AiFuture<'a, rom_ai::ReconciliationObservation> {
        Box::pin(async move {
            // Lookup can reconcile an expired generation; only the current lookup budget applies.
            let deadline = crate::execution::deadline(&execution, None)?;
            self.observed_reconciliation(prepared, evidence, deadline)
                .await
        })
    }
}
impl fmt::Debug for OpenRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpenRouter { .. }")
    }
}
