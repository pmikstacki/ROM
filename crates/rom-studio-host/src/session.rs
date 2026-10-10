use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rom::{Actor, Error, Result};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use subtle::ConstantTimeEq;
use tokio::sync::watch;

pub(crate) fn secret() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Storage)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
pub(crate) struct SessionEvidence {
    pub(crate) actor: Actor,
    pub(crate) user_id: String,
    pub(crate) token_expiry: u64,
    pub(crate) credentials: Option<Arc<crate::oidc::Credentials>>,
}
pub(crate) struct Session {
    cookie: String,
    csrf: String,
    generation: String,
    expiry: u64,
    pub(crate) evidence: SessionEvidence,
    cancelled: watch::Sender<bool>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionValidity {
    Current,
    Expired,
    Cancelled,
}
impl Session {
    pub(crate) fn cookie(&self) -> &str {
        &self.cookie
    }
    pub(crate) fn csrf(&self) -> &str {
        &self.csrf
    }
    pub(crate) fn generation(&self) -> &str {
        &self.generation
    }
    pub(crate) fn expires_at(&self) -> u64 {
        self.expiry
    }
    pub(crate) fn validity(&self, now: u64) -> SessionValidity {
        if *self.cancelled.borrow() {
            SessionValidity::Cancelled
        } else if now >= self.expiry {
            SessionValidity::Expired
        } else {
            SessionValidity::Current
        }
    }
    pub(crate) fn cancellation(&self) -> watch::Receiver<bool> {
        self.cancelled.subscribe()
    }
    pub(crate) fn check_csrf(&self, value: &str) -> bool {
        bool::from(self.csrf.as_bytes().ct_eq(value.as_bytes()))
    }
    fn cancel(&self) {
        self.cancelled.send_replace(true);
    }
}
struct Entries {
    closed: bool,
    sessions: BTreeMap<String, Arc<Session>>,
}
pub(crate) struct SessionStore {
    entries: Mutex<Entries>,
    capacity: usize,
    ttl: u64,
}
impl SessionStore {
    pub(crate) fn new(capacity: usize, ttl: u64) -> Self {
        Self {
            entries: Mutex::new(Entries {
                closed: false,
                sessions: BTreeMap::new(),
            }),
            capacity,
            ttl,
        }
    }
    pub(crate) fn insert(&self, evidence: SessionEvidence, now: u64) -> Result<Arc<Session>> {
        let mut entries = self.entries.lock().map_err(|_| Error::Panicked)?;
        expire(&mut entries.sessions, now);
        if entries.closed {
            return Err(Error::Closed);
        }
        if entries.sessions.len() >= self.capacity {
            return Err(Error::Overloaded);
        }
        let expiry = evidence.token_expiry.min(now.saturating_add(self.ttl));
        if expiry <= now || evidence.actor.valid_until().is_none_or(|end| end <= now) {
            return Err(Error::Denied);
        }
        let session = Arc::new(Session {
            cookie: secret()?,
            csrf: secret()?,
            generation: secret()?,
            expiry,
            evidence,
            cancelled: watch::channel(false).0,
        });
        entries
            .sessions
            .insert(session.cookie.clone(), session.clone());
        Ok(session)
    }
    pub(crate) fn lookup(&self, cookie: &str, now: u64) -> Option<Arc<Session>> {
        if cookie.len() != 43 {
            return None;
        }
        let mut entries = self.entries.lock().ok()?;
        expire(&mut entries.sessions, now);
        if entries.closed {
            return None;
        }
        entries.sessions.get(cookie).cloned()
    }
    pub(crate) fn failed(&self, cookie: &str, error: &Error) -> bool {
        matches!(error, Error::Denied | Error::Panicked) && self.remove(cookie)
    }
    pub(crate) fn remove(&self, cookie: &str) -> bool {
        if let Ok(mut entries) = self.entries.lock()
            && let Some(session) = entries.sessions.remove(cookie)
        {
            session.cancel();
            true
        } else {
            false
        }
    }
    pub(crate) fn close(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.closed = true;
            for session in entries.sessions.values() {
                session.cancel();
            }
            entries.sessions.clear();
        }
    }
}
fn expire(sessions: &mut BTreeMap<String, Arc<Session>>, now: u64) {
    sessions.retain(|_, session| {
        if now >= session.expiry {
            session.cancel();
            false
        } else {
            true
        }
    });
}
