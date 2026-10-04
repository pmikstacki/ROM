use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rom::{Error, Result};
use rom_auth::jwt::DecodingKey;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeySet {
    keys: Vec<Jwk>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    key_ops: Option<Vec<String>>,
}
pub(crate) fn parse_jwks(bytes: &[u8]) -> Result<BTreeMap<String, DecodingKey>> {
    if bytes.len() > 65_536 {
        return Err(Error::TooLarge);
    }
    let set: KeySet = serde_json::from_slice(bytes).map_err(|_| Error::Denied)?;
    if set.keys.is_empty() || set.keys.len() > 8 {
        return Err(Error::Denied);
    }
    let mut keys = BTreeMap::new();
    for jwk in set.keys {
        if jwk.kid.is_empty()
            || jwk.kid.len() > 64
            || jwk.kty != "RSA"
            || jwk.alg.as_deref().is_some_and(|alg| alg != "RS256")
            || jwk.usage.as_deref().is_some_and(|usage| usage != "sig")
            || jwk
                .key_ops
                .as_ref()
                .is_some_and(|ops| ops.as_slice() != ["verify"])
            || keys.contains_key(&jwk.kid)
        {
            return Err(Error::Denied);
        }
        let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|_| Error::Denied)?;
        keys.insert(jwk.kid, key);
    }
    Ok(keys)
}
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
        self.cached.lock().await.proof.bind(&shared.runtime).await
    }
    pub(crate) async fn actor(&self, shared: &Arc<Shared>) -> Result<Actor> {
        let now = shared.config.clock.now();
        if now >= self.expiry {
            return Err(Error::Denied);
        }
        let mut cached = self.cached.lock().await;
        if now >= cached.until {
            cached.proof = verify(
                shared,
                &self.activation,
                &self.token,
                &self.nonce,
                &self.access_token,
                &self.code,
            )
            .await?;
            cached.until = cached
                .proof
                .bind(&shared.runtime)
                .await?
                .valid_until()
                .ok_or(Error::Denied)?;
        }
        cached.proof.bind(&shared.runtime).await
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
) -> Result<ActivatedIdentity> {
    let config = shared
        .config
        .approved
        .iter()
        .find(|config| config.authority == activation.authority())
        .ok_or(Error::Denied)?;
    let keys = parse_jwks(
        &bounded(
            shared.client.get(shared.config.jwks_endpoint(config)),
            shared.config.limits.acquisition_bytes,
        )
        .await?,
    )?;
    let now = shared.config.clock.now();
    let activation = activation.clone();
    let token = token.to_owned();
    let nonce = nonce.to_owned();
    let access = access.to_owned();
    let code = code.to_owned();
    tokio::task::spawn_blocking(move || {
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
    .map_err(|_| Error::Panicked)?
}
async fn bounded(request: reqwest::RequestBuilder, limit: usize) -> Result<Vec<u8>> {
    let response = request.send().await.map_err(|_| Error::Denied)?;
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
        let chunk = chunk.map_err(|_| Error::Denied)?;
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
