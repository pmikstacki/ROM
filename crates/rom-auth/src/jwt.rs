//! RS256 access-token verification with explicit host-selected trust.
//!
//! A host supplies issuer-bound public keys and consumes the proof only after verification:
//! ```no_run
//! use std::collections::BTreeMap;
//! use rom_auth::{AuthError, VerifiedIdentity};
//! use rom_auth::jwt::{DecodingKey, JwtAdapter, TrustedKeys};
//! struct Pinned(BTreeMap<String, DecodingKey>);
//! impl TrustedKeys for Pinned {
//!     fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
//!         Ok(self.0.clone())
//!     }
//! }
//! fn verify(key: DecodingKey, token: &str, now: u64) -> Result<VerifiedIdentity, AuthError> {
//!     let mut adapter = JwtAdapter::configured("employees", "https://issuer.example",
//!         "rom-api", Pinned(BTreeMap::from([("signing-key".into(), key)])))?;
//!     adapter.authenticate(token, now)
//! }
//! ```
//! Request deserialization cannot create verified evidence:
//! ```compile_fail
//! let forged = serde_json::from_str::<rom_auth::VerifiedIdentity>("{}");
//! ```
use crate::{AuthError, IdentityProfile, VerifiedIdentity, claims::Audience, keys::KeyCache};
/// Public-key representation used by the selected cryptographic backend.
pub use jsonwebtoken::DecodingKey;
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct JwtClaims {
    iss: String,
    aud: Audience,
    sub: String,
    exp: u64,
    iat: u64,
    jti: String,
    client_id: String,
    principal_kind: String,
    #[serde(default, deserialize_with = "crate::present_claim")]
    nbf: Option<u64>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    cnf: Option<serde_json::Value>,
}

/// Host-selected public-key source; token headers cannot select the source.
///
/// Implementations must bound acquisition time and bytes, bind keys to the configured
/// issuer, and reject ambiguous key IDs before creating the map. Called synchronously;
/// remote discovery, network timeouts and concurrent fetch coalescing are host concerns.
/// Key IDs are opaque, case-sensitive strings of 1 through 256 UTF-8 bytes, without
/// Unicode control characters. This ROM profile bound is not a JOSE requirement.
/// IDs are matched exactly, without normalization or truncation.
pub trait TrustedKeys {
    /// Fetch one complete key set; a successful refresh replaces the previous set.
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError>;
}
/// Bounded RS256 human access-token verifier with a host-owned key source.
///
/// Refresh interval: 5 seconds; key and proof validity: at most 30 seconds;
/// at most 8 keys. These profile limits are fixed, not production-wide ROM policy.
pub struct JwtAdapter<K> {
    authority: String,
    expected_issuer: String,
    audience: String,
    validation: Validation,
    keys: KeyCache<K>,
}
impl<K: TrustedKeys> JwtAdapter<K> {
    /// Configure exact issuer/audience binding and a nonempty authority namespace.
    pub fn configured(
        authority: &str,
        expected_issuer: &str,
        audience: &str,
        source: K,
    ) -> Result<Self, AuthError> {
        if ![authority, expected_issuer, audience]
            .into_iter()
            .all(crate::valid_name)
        {
            return Err(AuthError::InvalidConfiguration);
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[expected_issuer]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp"]);
        validation.leeway = 0;
        // Evaluate temporal claims below against the same trusted host clock used for cache expiry.
        validation.validate_exp = false;
        validation.validate_nbf = false;
        Ok(Self {
            authority: authority.into(),
            expected_issuer: expected_issuer.into(),
            audience: audience.into(),
            validation,
            keys: KeyCache::new(source),
        })
    }
    /// Verify a bearer using trusted host Unix time, never a time supplied by a request.
    ///
    /// Rejects ID-token types, non-RS256, key URLs, embedded keys, critical extensions
    /// and sender constraints. Requires `principal_kind=human`, `client_id`, `jti`,
    /// `iat`, `exp`, `sub`, issuer and audience. Issued lifetime is at most one hour.
    pub fn authenticate(&mut self, token: &str, now: u64) -> Result<VerifiedIdentity, AuthError> {
        if token.len() > 16_384 {
            return Err(AuthError::TooLarge);
        }
        let header = decode_header(token).map_err(|_| AuthError::Invalid)?;
        // Unverified header can select only a key from the configured source; it grants no trust.
        if header.alg != Algorithm::RS256
            || !matches!(header.typ.as_deref(), Some("at+jwt" | "application/at+jwt"))
        {
            return Err(AuthError::WrongProfile);
        }
        if header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
            || header.crit.as_ref().is_some_and(|c| !c.is_empty())
        {
            return Err(AuthError::WrongProfile);
        }
        let kid = header
            .kid
            .as_deref()
            .filter(|x| crate::key_id::valid_key_id(x))
            .ok_or(AuthError::UnknownKey)?;
        let key = self.keys.get(kid, now)?;
        let claims = decode::<JwtClaims>(token, key, &self.validation)
            .map_err(|_| AuthError::Invalid)?
            .claims;
        // Require an exact string issuer independently of the JWT library
        // (its validation model also permits issuer arrays via set intersection).
        if claims.iss != self.expected_issuer || !claims.aud.contains(&self.audience) {
            return Err(AuthError::Binding);
        }
        if claims.exp <= now
            || claims.nbf.is_some_and(|nbf| nbf > now)
            || claims.iat > now
            || claims.exp.saturating_sub(claims.iat) > 3600
        {
            return Err(AuthError::Expired);
        }
        if claims.sub.is_empty()
            || claims.sub.len() > 256
            || claims.jti.is_empty()
            || claims.client_id.is_empty()
            || claims.principal_kind != "human"
            || claims.cnf.is_some()
        {
            return Err(AuthError::WrongProfile);
        }
        Ok(VerifiedIdentity::verified(
            &self.authority,
            claims.sub,
            IdentityProfile::JwtRs256Human,
            claims
                .exp
                .min(self.keys.until())
                .min(now.saturating_add(30)),
        ))
    }
}
