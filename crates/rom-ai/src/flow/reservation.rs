//! Immutable exact attempt identities; unknown outcomes retain the ceiling charge.
use super::codec::{Validated, input};
use crate::{AiError, AiResult, PreparedAttempt, RoutingTier, UsdNanos, request::valid_name};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationKey {
    account_window: String,
    run_id: String,
    step: u32,
    attempt_ordinal: u32,
    policy_version: u32,
}
impl ReservationKey {
    pub fn new(
        account_window: impl Into<String>,
        run_id: impl Into<String>,
        step: u32,
        attempt_ordinal: u32,
        policy_version: u32,
    ) -> AiResult<Self> {
        let value = Self {
            account_window: account_window.into(),
            run_id: run_id.into(),
            step,
            attempt_ordinal,
            policy_version,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn account_window(&self) -> &str {
        &self.account_window
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn step(&self) -> u32 {
        self.step
    }
    pub fn attempt_ordinal(&self) -> u32 {
        self.attempt_ordinal
    }
    pub fn policy_version(&self) -> u32 {
        self.policy_version
    }
    pub fn attempt_identity(&self) -> String {
        format!("{}:{}:{}", self.run_id, self.step, self.attempt_ordinal)
    }
    pub(crate) fn same_dispatch(&self, other: &Self) -> bool {
        self.account_window == other.account_window
            && self.run_id == other.run_id
            && self.step == other.step
            && self.attempt_ordinal == other.attempt_ordinal
    }
}
impl Validated for ReservationKey {
    fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.account_window, 64)
            || !valid_name(&self.run_id, 64)
            || !(1..=32).contains(&self.step)
            || !(1..=8).contains(&self.attempt_ordinal)
            || self.policy_version == 0
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
impl fmt::Debug for ReservationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReservationKey")
            .field("step", &self.step)
            .field("ordinal", &self.attempt_ordinal)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ReservationStatus {
    Reserved,
    Unknown,
    Settled { actual_cost: UsdNanos },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationEntry {
    key: ReservationKey,
    prepared: PreparedAttempt,
    pub(crate) status: ReservationStatus,
}
impl ReservationEntry {
    pub fn key(&self) -> &ReservationKey {
        &self.key
    }
    pub fn prepared(&self) -> &PreparedAttempt {
        &self.prepared
    }
    pub fn maximum_cost(&self) -> UsdNanos {
        self.prepared.route().maximum_cost()
    }
    pub fn status(&self) -> &ReservationStatus {
        &self.status
    }
    pub(crate) fn same_reservation(&self, other: &Self) -> bool {
        self.key == other.key && self.prepared == other.prepared
    }
}
impl Validated for ReservationEntry {
    fn validate(&self) -> AiResult<()> {
        self.key.validate()?;
        self.prepared.validate()?;
        if self.prepared.identity() != self.key.attempt_identity()
            || self.prepared.policy().version() != self.key.policy_version
        {
            return Err(AiError::InvalidRequest);
        }
        match self.prepared.policy().budget_reference() {
            Some(window) if window == self.key.account_window() => {}
            None if self.prepared.route().tier() == RoutingTier::Free
                && self.maximum_cost() == UsdNanos(0) => {}
            _ => return Err(AiError::Denied),
        }
        if let ReservationStatus::Settled { actual_cost } = self.status
            && actual_cost > self.maximum_cost()
        {
            return Err(AiError::InvalidOutput);
        }
        Ok(())
    }
}
impl fmt::Debug for ReservationEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReservationEntry")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveBudget {
    entry: ReservationEntry,
}
impl ReserveBudget {
    pub fn new(key: ReservationKey, prepared: PreparedAttempt) -> AiResult<Self> {
        let value = Self {
            entry: ReservationEntry {
                key,
                prepared,
                status: ReservationStatus::Reserved,
            },
        };
        value.validate()?;
        Ok(value)
    }
    pub fn entry(&self) -> &ReservationEntry {
        &self.entry
    }
}
impl Validated for ReserveBudget {
    fn validate(&self) -> AiResult<()> {
        self.entry.validate()?;
        if self.entry.status != ReservationStatus::Reserved {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(ReserveBudget);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettleBudget {
    pub(crate) key: ReservationKey,
    pub(crate) actual_cost: Option<UsdNanos>,
}
impl SettleBudget {
    pub fn unknown(key: ReservationKey) -> AiResult<Self> {
        let value = Self {
            key,
            actual_cost: None,
        };
        value.validate()?;
        Ok(value)
    }
    /// The trusted host must authenticate usage evidence before constructing settlement.
    pub fn confirmed(key: ReservationKey, actual_cost: UsdNanos) -> AiResult<Self> {
        let value = Self {
            key,
            actual_cost: Some(actual_cost),
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for SettleBudget {
    fn validate(&self) -> AiResult<()> {
        self.key.validate()
    }
}
input!(SettleBudget);
