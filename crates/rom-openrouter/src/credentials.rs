//! Credentials exist only in a bounded trusted host callback and private HTTP headers.
use rom_ai::{AiError, AiFuture, AiResult};
use std::fmt;

pub trait CredentialSource: Send + Sync {
    fn resolve<'a>(&'a self, reference: &'a str) -> AiFuture<'a, SecretToken>;
}
pub struct SecretToken(pub(crate) String);
impl SecretToken {
    pub fn new(value: impl Into<String>) -> AiResult<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 4096
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self(value))
    }
}
impl fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretToken { .. }")
    }
}
