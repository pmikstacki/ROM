//! Persisted read ordinals and bounded physical callback ownership are separate facts.
use crate::{AiError, AiResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex, Weak},
};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ReadPhase {
    Scheduled,
    Started,
    Unresolved,
    Completed,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadAttempt {
    pub ordinal: u32,
    pub phase: ReadPhase,
    pub operation_key: Option<String>,
}
impl ReadAttempt {
    pub fn validate(&self) -> AiResult<()> {
        if !(1..=32).contains(&self.ordinal)
            || self
                .operation_key
                .as_ref()
                .is_some_and(|key| !crate::request::valid_name(key, 128))
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReadKey {
    run: String,
    source: String,
    call: String,
}
impl ReadKey {
    fn new(run: &str, source: &str, call: &str) -> Self {
        Self {
            run: run.into(),
            source: source.into(),
            call: call.into(),
        }
    }
}
/// A descendant retains this guard until its actual synchronous execution finishes.
pub(crate) struct ReadLease {
    _key: ReadKey,
}
impl fmt::Debug for ReadLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReadLease { .. }")
    }
}
#[derive(Default)]
pub(crate) struct ReadActivity {
    active: Mutex<BTreeMap<ReadKey, Weak<ReadLease>>>,
}
impl ReadActivity {
    pub fn acquire(&self, run: &str, source: &str, call: &str) -> AiResult<Arc<ReadLease>> {
        let key = ReadKey::new(run, source, call);
        let mut active = self.active.lock().map_err(|_| AiError::Closed)?;
        active.retain(|_, lease| lease.strong_count() > 0);
        if active.contains_key(&key) {
            return Err(AiError::Conflict);
        }
        if active.len() >= 32 {
            return Err(AiError::BudgetExhausted);
        }
        let lease = Arc::new(ReadLease { _key: key.clone() });
        active.insert(key, Arc::downgrade(&lease));
        Ok(lease)
    }
    pub fn is_active(&self, run: &str, source: &str, call: &str) -> AiResult<bool> {
        let active = self.active.lock().map_err(|_| AiError::Closed)?;
        Ok(active
            .get(&ReadKey::new(run, source, call))
            .is_some_and(|lease| lease.strong_count() > 0))
    }
}
