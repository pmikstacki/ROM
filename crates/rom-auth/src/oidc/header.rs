use crate::AuthError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::Algorithm;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: Algorithm,
    kid: String,
    #[serde(default, deserialize_with = "crate::present_claim")]
    typ: Option<String>,
}
pub(super) fn key_id(token: &str) -> Result<String, AuthError> {
    // A closed header denies embedded trust, URLs, certificates, critical and
    // unencoded-payload extensions before contacting the host's key source.
    let bytes = URL_SAFE_NO_PAD
        .decode(token.split('.').next().ok_or(AuthError::Invalid)?)
        .map_err(|_| AuthError::Invalid)?;
    if !serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| AuthError::WrongProfile)?
        .is_object()
    {
        return Err(AuthError::WrongProfile);
    }
    let header: Header = serde_json::from_slice(&bytes).map_err(|_| AuthError::WrongProfile)?;
    if header.alg != Algorithm::RS256 || header.typ.as_deref().is_some_and(|typ| typ != "JWT") {
        return Err(AuthError::WrongProfile);
    }
    if header.kid.is_empty() || header.kid.len() > 64 {
        return Err(AuthError::UnknownKey);
    }
    Ok(header.kid)
}
