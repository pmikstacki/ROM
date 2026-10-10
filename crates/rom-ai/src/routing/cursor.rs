//! Continuation belongs to a run, never a process-global endpoint/schema cache.
use crate::{AiError, AiResult, request::valid_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingTier {
    Free,
    Paid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCursor {
    policy_version: u32,
    catalog_identity: String,
    tier: RoutingTier,
    next_candidate: usize,
    rejected: BTreeSet<String>,
}
impl RouteCursor {
    pub fn new(policy_version: u32, catalog_identity: impl Into<String>) -> AiResult<Self> {
        let value = Self {
            policy_version,
            catalog_identity: catalog_identity.into(),
            tier: RoutingTier::Free,
            next_candidate: 0,
            rejected: BTreeSet::new(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn reject(&self, model: &str) -> AiResult<Self> {
        let mut next = self.clone();
        next.rejected.insert(model.into());
        next.validate()?;
        Ok(next)
    }
    pub fn policy_version(&self) -> u32 {
        self.policy_version
    }
    pub fn catalog_identity(&self) -> &str {
        &self.catalog_identity
    }
    pub fn tier(&self) -> RoutingTier {
        self.tier
    }
    pub fn next_candidate(&self) -> usize {
        self.next_candidate
    }
    pub fn is_rejected(&self, model: &str) -> bool {
        self.rejected.contains(model)
    }
    pub(crate) fn advanced(&self, tier: RoutingTier, index: usize) -> Self {
        let mut next = self.clone();
        next.tier = tier;
        next.next_candidate = index;
        next
    }
    pub fn validate(&self) -> AiResult<()> {
        if self.policy_version == 0
            || !valid_name(&self.catalog_identity, 128)
            || self.next_candidate > 256
            || self.rejected.len() > 256
            || self.rejected.iter().any(|id| !valid_name(id, 256))
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
