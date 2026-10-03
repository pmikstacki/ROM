use super::{OidcTokenBindings, claims::IdClaims, header};
use crate::{AuthError, IdentityProfile, VerifiedIdentity, jwt::TrustedKeys, keys::KeyCache};
use jsonwebtoken::{Algorithm, Validation, decode};

/// Host-configured RS256 human ID-token verifier.
///
/// Accepts only signed authorization-code-flow ID tokens, with no clock leeway.
/// Issued lifetime is at most one hour; evidence and cached keys last at most
/// 30 seconds. The key source has the same eight-key, five-second refresh bounds
/// as [`crate::jwt::JwtAdapter`]. Trust-source approval belongs to the host.
pub struct OidcIdTokenAdapter<K> {
    authority: String,
    issuer: String,
    client_id: String,
    audiences: Vec<String>,
    validation: Validation,
    keys: KeyCache<K>,
}
impl<K: TrustedKeys> OidcIdTokenAdapter<K> {
    /// Configure the exact authority, issuer, client ID and issuer-bound key source.
    /// Names must be nonempty, at most 2048 bytes, with no surrounding whitespace.
    pub fn configured(
        authority: &str,
        issuer: &str,
        client_id: &str,
        source: K,
    ) -> Result<Self, AuthError> {
        if ![authority, issuer, client_id]
            .into_iter()
            .all(crate::valid_name)
        {
            return Err(AuthError::InvalidConfiguration);
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[client_id]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp"]);
        validation.leeway = 0;
        // Use one explicit trusted clock for token, key and proof deadlines.
        validation.validate_exp = false;
        validation.validate_nbf = false;
        Ok(Self {
            authority: authority.into(),
            issuer: issuer.into(),
            client_id: client_id.into(),
            audiences: vec![client_id.into()],
            validation,
            keys: KeyCache::new(source),
        })
    }
    /// Approve up to seven additional exact audiences. The client remains mandatory.
    /// Empty, oversized, duplicate and client-repeated entries are rejected.
    /// This replaces any previous additional audience configuration.
    pub fn with_trusted_audiences(mut self, additional: &[&str]) -> Result<Self, AuthError> {
        if additional.len() > 7 {
            return Err(AuthError::InvalidConfiguration);
        }
        let mut audiences = vec![self.client_id.clone()];
        for audience in additional {
            if !crate::valid_name(audience) || audiences.iter().any(|value| value == audience) {
                return Err(AuthError::InvalidConfiguration);
            }
            audiences.push((*audience).into());
        }
        self.audiences = audiences;
        Ok(self)
    }
    /// Verify an ID token with the retained nonce and trusted host Unix time.
    ///
    /// The token is at most 16 KiB. The independently retained nonce must be
    /// nonempty printable ASCII, at most 2048 bytes. Present `at_hash` or `c_hash`
    /// requires a matching original input. Hash absence is permitted for this
    /// authorization-code profile. No UserInfo, refresh or hybrid-flow profile
    /// is supported. A successful call does not consume the host's login attempt;
    /// the host must atomically enforce one-use state, code and nonce itself.
    pub fn authenticate(
        &mut self,
        token: &str,
        expected_nonce: &str,
        bindings: OidcTokenBindings<'_>,
        now: u64,
    ) -> Result<VerifiedIdentity, AuthError> {
        if token.len() > 16_384 {
            return Err(AuthError::TooLarge);
        }
        if !super::claims::opaque(expected_nonce, 2048) {
            return Err(AuthError::InvalidConfiguration);
        }
        let kid = header::key_id(token)?;
        let key = self.keys.get(&kid, now)?;
        let claims = decode::<IdClaims>(token, key, &self.validation)
            .map_err(|_| AuthError::Invalid)?
            .claims;
        claims.validate(
            &self.issuer,
            &self.client_id,
            &self.audiences,
            expected_nonce,
            bindings,
            now,
        )?;
        Ok(VerifiedIdentity::verified(
            &self.authority,
            claims.sub,
            IdentityProfile::OidcRs256Human,
            claims
                .exp
                .min(self.keys.until())
                .min(now.saturating_add(30)),
        ))
    }
}
