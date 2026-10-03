use crate::{
    AuthError,
    jwt::{DecodingKey, TrustedKeys},
};
use std::collections::BTreeMap;

/// Shared bounded, issuer-selected key cache. Acquisition remains a host concern.
pub(crate) struct KeyCache<K> {
    source: K,
    keys: BTreeMap<String, DecodingKey>,
    until: u64,
    last_refresh: Option<u64>,
}
impl<K: TrustedKeys> KeyCache<K> {
    pub(crate) fn new(source: K) -> Self {
        Self {
            source,
            keys: BTreeMap::new(),
            until: 0,
            last_refresh: None,
        }
    }
    pub(crate) fn get(&mut self, kid: &str, now: u64) -> Result<&DecodingKey, AuthError> {
        if kid.is_empty() || kid.len() > 64 {
            return Err(AuthError::UnknownKey);
        }
        if now >= self.until || !self.keys.contains_key(kid) {
            if self
                .last_refresh
                .is_some_and(|last| now < last.saturating_add(5))
            {
                return Err(AuthError::RefreshLimited);
            }
            self.last_refresh = Some(now);
            let fresh = self.source.fetch()?;
            if fresh.is_empty()
                || fresh.len() > 8
                || fresh.keys().any(|key| key.is_empty() || key.len() > 64)
            {
                return Err(AuthError::Invalid);
            }
            self.keys = fresh;
            self.until = now.saturating_add(30);
        }
        self.keys.get(kid).ok_or(AuthError::UnknownKey)
    }
    pub(crate) fn until(&self) -> u64 {
        self.until
    }
}
