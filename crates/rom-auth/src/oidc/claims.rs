use super::OidcTokenBindings;
use crate::{AuthError, claims::Audience};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

#[derive(Deserialize)]
pub(super) struct IdClaims {
    iss: String,
    aud: Audience,
    pub(super) sub: String,
    pub(super) exp: u64,
    iat: u64,
    nonce: String,
    #[serde(default, deserialize_with = "crate::present_claim")]
    azp: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    nbf: Option<u64>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    at_hash: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    c_hash: Option<String>,
}
impl IdClaims {
    pub(super) fn validate(
        &self,
        issuer: &str,
        client: &str,
        allowed: &[String],
        nonce: &str,
        bindings: OidcTokenBindings<'_>,
        now: u64,
    ) -> Result<(), AuthError> {
        if self.iss != issuer {
            return Err(AuthError::Binding);
        }
        let audiences = match &self.aud {
            Audience::One(value) => std::slice::from_ref(value),
            Audience::Many(values) => values.as_slice(),
        };
        if audiences.is_empty()
            || audiences.len() > 8
            || !audiences.iter().any(|value| value == client)
            || audiences.iter().enumerate().any(|(index, value)| {
                !crate::valid_name(value)
                    || !allowed.contains(value)
                    || audiences[..index].contains(value)
            })
            || self.azp.as_deref().is_some_and(|value| value != client)
            || (audiences.len() > 1 && self.azp.as_deref() != Some(client))
        {
            return Err(AuthError::Binding);
        }
        if self.exp <= now
            || self.iat > now
            || self.nbf.is_some_and(|value| value > now)
            || self.exp.saturating_sub(self.iat) > 3600
        {
            return Err(AuthError::Expired);
        }
        if !opaque(&self.sub, 255)
            || !opaque(&self.nonce, 2048)
            || !bool::from(self.nonce.as_bytes().ct_eq(nonce.as_bytes()))
        {
            return Err(AuthError::Binding);
        }
        hash(self.at_hash.as_deref(), bindings.access_token)?;
        hash(self.c_hash.as_deref(), bindings.authorization_code)?;
        Ok(())
    }
}
pub(super) fn opaque(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}
fn hash(claim: Option<&str>, original: Option<&str>) -> Result<(), AuthError> {
    let Some(claim) = claim else {
        return Ok(());
    };
    let original = original
        .filter(|value| opaque(value, 16_384))
        .ok_or(AuthError::Binding)?;
    let digest = Sha256::digest(original.as_bytes());
    let expected = URL_SAFE_NO_PAD.encode(&digest[..16]);
    if !bool::from(claim.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(AuthError::Binding);
    }
    Ok(())
}
