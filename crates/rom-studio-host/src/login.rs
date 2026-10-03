use crate::session::secret;
use rom::{Error, Result};
use std::{collections::BTreeMap, sync::Mutex};
use subtle::ConstantTimeEq;

pub(crate) struct Attempt {
    pub(crate) provider: String,
    pub(crate) browser: String,
    pub(crate) nonce: String,
    pub(crate) verifier: String,
    pub(crate) expires: u64,
    pub(crate) activation: Option<rom_identity::ProviderActivation>,
}
struct State {
    closed: bool,
    entries: BTreeMap<String, Attempt>,
}
pub(crate) struct Attempts {
    state: Mutex<State>,
    capacity: usize,
}
impl Attempts {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(State {
                closed: false,
                entries: BTreeMap::new(),
            }),
            capacity,
        }
    }
    pub(crate) fn insert(&self, attempt: Attempt, now: u64) -> Result<String> {
        let mut state = self.state.lock().map_err(|_| Error::Panicked)?;
        state.entries.retain(|_, attempt| attempt.expires > now);
        if state.closed {
            return Err(Error::Closed);
        }
        if state.entries.len() >= self.capacity {
            return Err(Error::Overloaded);
        }
        if attempt.expires <= now {
            return Err(Error::Denied);
        }
        let token = secret()?;
        state.entries.insert(token.clone(), attempt);
        Ok(token)
    }
    pub(crate) fn consume(
        &self,
        token: &str,
        provider: &str,
        browser: &str,
        now: u64,
    ) -> Result<Attempt> {
        let mut state = self.state.lock().map_err(|_| Error::Panicked)?;
        if state.closed {
            return Err(Error::Closed);
        }
        let attempt = state.entries.remove(token).ok_or(Error::Denied)?;
        if attempt.expires <= now
            || attempt.provider != provider
            || !bool::from(attempt.browser.as_bytes().ct_eq(browser.as_bytes()))
        {
            return Err(Error::Denied);
        }
        Ok(attempt)
    }
    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.entries.clear();
        }
    }
}
