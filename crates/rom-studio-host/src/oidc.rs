use crate::auth_diagnostics::{AuthOperation, AuthOutcome, AuthStage};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rom::{Error, Result};
use rom_auth::jwt::DecodingKey;
use serde::Deserialize;
use std::collections::BTreeMap;

pub(crate) use crate::jwks::parse_jwks;

pub(crate) fn original_expiry(token: &str) -> Result<u64> {
    #[derive(Deserialize)]
    struct Expiry {
        exp: u64,
    }
    if token.len() > 16_384 {
        return Err(Error::TooLarge);
    }
    let mut parts = token.split('.');
    parts.next().ok_or(Error::Denied)?;
    let body = parts.next().ok_or(Error::Denied)?;
    if parts.next().is_none() || parts.next().is_some() {
        return Err(Error::Denied);
    }
    let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| Error::Denied)?;
    Ok(serde_json::from_slice::<Expiry>(&bytes)
        .map_err(|_| Error::Denied)?
        .exp)
}

use crate::{OidcProviderConfig, router::Shared};
use futures_util::StreamExt;
use rom::Actor;
use rom_auth::{AuthError, OidcIdTokenAdapter, jwt::TrustedKeys, oidc::OidcTokenBindings};
use rom_identity::{ActivatedIdentity, ProviderActivation};
use std::sync::Arc;

struct PinnedKeys(BTreeMap<String, DecodingKey>);
impl TrustedKeys for PinnedKeys {
    fn fetch(&mut self) -> std::result::Result<BTreeMap<String, DecodingKey>, AuthError> {
        Ok(self.0.clone())
    }
}
struct Cached {
    proof: ActivatedIdentity,
    until: u64,
}
pub(crate) struct Credentials {
    activation: ProviderActivation,
    token: String,
    nonce: String,
    access_token: String,
    code: String,
    pub(crate) expiry: u64,
    cached: tokio::sync::Mutex<Cached>,
}
impl Credentials {
    pub(crate) async fn exchange(
        shared: &Arc<Shared>,
        config: &OidcProviderConfig,
        attempt: crate::login::Attempt,
        code: String,
    ) -> Result<(Self, Actor)> {
        let activation = attempt.activation.ok_or(Error::Denied)?;
        let mut request = shared
            .client
            .post(shared.config.token_endpoint(config))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("code_verifier", attempt.verifier.as_str()),
                (
                    "redirect_uri",
                    shared.config.callback(&config.authority).as_str(),
                ),
            ]);
        if let Some(secret) = &config.client_secret {
            request = client_secret_basic(request, &config.client_id, secret);
        } else {
            request = request.form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("code_verifier", attempt.verifier.as_str()),
                (
                    "redirect_uri",
                    shared.config.callback(&config.authority).as_str(),
                ),
                ("client_id", config.client_id.as_str()),
            ]);
        }
        let bytes = bounded(request, shared.config.limits.acquisition_bytes).await?;
        #[derive(Deserialize)]
        struct Tokens {
            id_token: String,
            access_token: String,
            token_type: String,
        }
        let tokens: Tokens = serde_json::from_slice(&bytes).map_err(|_| Error::Denied)?;
        if !tokens.token_type.eq_ignore_ascii_case("Bearer")
            || tokens.id_token.len() > 16_384
            || tokens.access_token.len() > 16_384
        {
            return Err(Error::Denied);
        }
        let proof = verify(
            shared,
            &activation,
            &tokens.id_token,
            &attempt.nonce,
            &tokens.access_token,
            &code,
            AuthOperation::Session,
        )
        .await?;
        let expiry = original_expiry(&tokens.id_token)?;
        let actor = proof.bind(&shared.runtime).await?;
        let until = actor.valid_until().ok_or(Error::Denied)?;
        Ok((
            Self {
                activation,
                token: tokens.id_token,
                nonce: attempt.nonce,
                access_token: tokens.access_token,
                code,
                expiry,
                cached: tokio::sync::Mutex::new(Cached { proof, until }),
            },
            actor,
        ))
    }
    pub(crate) async fn current(&self, shared: &Arc<Shared>) -> Result<Actor> {
        let cached = self.cached.lock().await;
        shared.auth.record(
            AuthOperation::CurrentStream,
            AuthStage::CredentialWait,
            AuthOutcome::Succeeded,
        );
        let outcome = cached.proof.bind(&shared.runtime).await;
        shared.auth.record(
            AuthOperation::CurrentStream,
            AuthStage::CurrentBind,
            AuthOutcome::result(&outcome),
        );
        outcome
    }
    #[cfg(test)]
    pub(crate) async fn actor(&self, shared: &Arc<Shared>) -> Result<Actor> {
        self.actor_observed(shared, AuthOperation::Unspecified)
            .await
    }
    pub(crate) async fn actor_observed(
        &self,
        shared: &Arc<Shared>,
        operation: AuthOperation,
    ) -> Result<Actor> {
        let mut cached = self.cached.lock().await;
        shared
            .auth
            .record(operation, AuthStage::CredentialWait, AuthOutcome::Succeeded);
        // Recheck time after waiting: cached evidence can expire while the lock is held.
        let now = shared.config.clock.now();
        if now >= self.expiry {
            shared
                .auth
                .record(operation, AuthStage::OriginalExpiry, AuthOutcome::Denied);
            return Err(Error::Denied);
        }
        let budget = crate::proof_handoff::budget(&shared.config, operation)?;
        if crate::proof_handoff::short(now, cached.until, budget) {
            shared
                .auth
                .record(operation, AuthStage::RenewDue, AuthOutcome::Succeeded);
            return self.renew(shared, &mut cached, operation, budget).await;
        }
        let outcome = cached.proof.bind(&shared.runtime).await;
        shared.auth.record(
            operation,
            AuthStage::CachedBind,
            AuthOutcome::result(&outcome),
        );
        let now = shared.config.clock.now();
        if now >= self.expiry {
            shared
                .auth
                .record(operation, AuthStage::OriginalExpiry, AuthOutcome::Denied);
            return Err(Error::Denied);
        }
        // Authoritative IO can cross the cached expiry after the initial clock check.
        // Renew once, retaining actual signature and current identity checks.
        if matches!(&outcome, Ok(_) | Err(Error::Denied)) && now >= cached.until
            || outcome.is_ok() && crate::proof_handoff::short(now, cached.until, budget)
        {
            shared
                .auth
                .record(operation, AuthStage::RenewAfterBind, AuthOutcome::Succeeded);
            return self.renew(shared, &mut cached, operation, budget).await;
        }
        outcome
    }
    async fn renew(
        &self,
        shared: &Arc<Shared>,
        cached: &mut Cached,
        operation: AuthOperation,
        budget: u64,
    ) -> Result<Actor> {
        let verified = verify(
            shared,
            &self.activation,
            &self.token,
            &self.nonce,
            &self.access_token,
            &self.code,
            operation,
        )
        .await;
        let proof = verified?;
        let bound = proof.bind(&shared.runtime).await;
        shared
            .auth
            .record(operation, AuthStage::FreshBind, AuthOutcome::result(&bound));
        let actor = bound?;
        let until = actor.valid_until().ok_or(Error::Denied)?;
        let now = shared.config.clock.now();
        if now >= self.expiry || now >= until {
            shared.auth.record(
                operation,
                if now >= self.expiry {
                    AuthStage::OriginalExpiry
                } else {
                    AuthStage::FreshProofExpiry
                },
                AuthOutcome::Denied,
            );
            return Err(Error::Denied);
        }
        if crate::proof_handoff::short(now, until, budget) {
            // Fresh evidence cannot reserve this operation margin. Do not loop,
            // extend its deadline, or fall back to the old cached proof.
            return Err(Error::Overloaded);
        }
        // Publish the verified proof and its deadline together only after successful binding.
        *cached = Cached { proof, until };
        Ok(actor)
    }
}

// RFC 6749 section 2.3.1 requires form encoding before HTTP Basic encoding.
pub(crate) fn client_secret_basic(
    request: reqwest::RequestBuilder,
    client: &str,
    secret: &str,
) -> reqwest::RequestBuilder {
    fn component(value: &str) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("", value)
            .finish()[1..]
            .to_owned()
    }
    request.basic_auth(component(client), Some(component(secret)))
}
async fn verify(
    shared: &Arc<Shared>,
    activation: &ProviderActivation,
    token: &str,
    nonce: &str,
    access: &str,
    code: &str,
    operation: AuthOperation,
) -> Result<ActivatedIdentity> {
    let config = shared
        .config
        .approved
        .iter()
        .find(|config| config.authority == activation.authority())
        .ok_or(Error::Denied)?;
    let acquisition = async {
        parse_jwks(
            &bounded(
                shared.client.get(shared.config.jwks_endpoint(config)),
                shared.config.limits.acquisition_bytes,
            )
            .await?,
        )
    }
    .await;
    shared.auth.record(
        operation,
        AuthStage::KeyAcquisition,
        AuthOutcome::result(&acquisition),
    );
    let keys = acquisition?;
    let now = shared.config.clock.now();
    let activation = activation.clone();
    let token = token.to_owned();
    let nonce = nonce.to_owned();
    let access = access.to_owned();
    let code = code.to_owned();
    let verified = tokio::task::spawn_blocking(move || {
        activation.verify(|authority, provider| {
            OidcIdTokenAdapter::configured(
                authority,
                &provider.issuer,
                &provider.audience,
                PinnedKeys(keys),
            )?
            .authenticate(
                &token,
                &nonce,
                OidcTokenBindings {
                    access_token: Some(&access),
                    authorization_code: Some(&code),
                },
                now,
            )
        })
    })
    .await
    .map_err(|_| Error::Panicked)
    .and_then(|result| result);
    shared.auth.record(
        operation,
        AuthStage::OriginalTokenVerification,
        AuthOutcome::result(&verified),
    );
    verified
}
// Retain session context only for a bounded, known acquisition outage. No proof is returned.
fn acquisition_error(error: reqwest::Error) -> Error {
    use std::error::Error as StdError;
    use std::io::ErrorKind;
    let mut temporary = error.is_timeout();
    let mut source: Option<&(dyn StdError + 'static)> = Some(&error);
    for _ in 0..16 {
        let Some(cause) = source else {
            return if temporary {
                Error::Overloaded
            } else {
                Error::Denied
            };
        };
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            match io.kind() {
                ErrorKind::InvalidData | ErrorKind::InvalidInput => return Error::Denied,
                ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::TimedOut
                | ErrorKind::NetworkDown
                | ErrorKind::NetworkUnreachable
                | ErrorKind::HostUnreachable => temporary = true,
                _ => {}
            }
        }
        source = cause.source();
    }
    Error::Denied
}

pub(crate) async fn bounded(request: reqwest::RequestBuilder, limit: usize) -> Result<Vec<u8>> {
    let response = request.send().await.map_err(acquisition_error)?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        || response.status().is_server_error()
    {
        return Err(Error::Overloaded);
    }
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > limit as u64)
    {
        return Err(Error::Denied);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(acquisition_error)?;
        if bytes
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > limit)
        {
            return Err(Error::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod credentials_tests;

#[cfg(test)]
mod credentials_concurrency_tests;

#[cfg(test)]
mod auth_diagnostics_tests;

#[cfg(test)]
mod auth_pipeline_diagnostics_tests;

#[cfg(test)]
mod session_expiry_tests;

#[cfg(test)]
#[path = "oidc/proof_handoff_tests.rs"]
mod proof_handoff_tests;

#[cfg(test)]
#[path = "oidc/proof_handoff_downstream_tests.rs"]
mod proof_handoff_downstream_tests;
