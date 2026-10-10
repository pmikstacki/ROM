//! Versioned account ledger with checked totals and conservative replay.
use super::{
    codec::{self, Validated},
    reservation::{ReservationEntry, ReservationStatus, ReserveBudget, SettleBudget},
    resource::OwnerIdentity,
};
use crate::{AiError, AiResult, UsdNanos, request::valid_name};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_RESERVATIONS: usize = 1024;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetRecord {
    version: u32,
    pub(crate) service: OwnerIdentity,
    account_window: String,
    limit: UsdNanos,
    reserved: UsdNanos,
    settled: UsdNanos,
    entries: Vec<ReservationEntry>,
}
impl BudgetRecord {
    pub fn limit(&self) -> UsdNanos {
        self.limit
    }
    pub fn reserved(&self) -> UsdNanos {
        self.reserved
    }
    pub fn settled(&self) -> UsdNanos {
        self.settled
    }
    pub fn entries(&self) -> &[ReservationEntry] {
        &self.entries
    }
    fn totals(&self) -> AiResult<(UsdNanos, UsdNanos)> {
        let (mut reserved, mut settled) = (0_u128, 0_u128);
        for entry in &self.entries {
            match entry.status() {
                ReservationStatus::Reserved | ReservationStatus::Unknown => {
                    reserved += u128::from(entry.maximum_cost().0)
                }
                ReservationStatus::Settled { actual_cost } => settled += u128::from(actual_cost.0),
            }
        }
        Ok((
            UsdNanos(u64::try_from(reserved).map_err(|_| AiError::BudgetExhausted)?),
            UsdNanos(u64::try_from(settled).map_err(|_| AiError::BudgetExhausted)?),
        ))
    }
    fn refresh(&mut self) -> AiResult<()> {
        (self.reserved, self.settled) = self.totals()?;
        self.validate()
    }
    pub(crate) fn reserve(&mut self, input: ReserveBudget) -> AiResult<()> {
        input.validate()?;
        if input.entry().key().account_window() != self.account_window {
            return Err(AiError::InvalidRequest);
        }
        if let Some(existing) = self
            .entries
            .iter()
            .find(|entry| entry.key().same_dispatch(input.entry().key()))
        {
            return if existing.same_reservation(input.entry()) {
                Ok(())
            } else {
                Err(AiError::Conflict)
            };
        }
        if self.entries.len() >= MAX_RESERVATIONS {
            return Err(AiError::BudgetExhausted);
        }
        self.entries.push(input.entry().clone());
        self.refresh()
    }
    pub(crate) fn tombstone_unstarted(&mut self, mut entry: ReservationEntry) -> AiResult<()> {
        entry.validate()?;
        if entry.key().account_window() != self.account_window
            || entry.status() != &ReservationStatus::Reserved
        {
            return Err(AiError::Conflict);
        }
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.key().same_dispatch(entry.key()))
        {
            if !existing.same_reservation(&entry) {
                return Err(AiError::Conflict);
            }
            match existing.status() {
                ReservationStatus::Reserved => {
                    existing.status = ReservationStatus::Settled {
                        actual_cost: UsdNanos(0),
                    }
                }
                ReservationStatus::Settled {
                    actual_cost: UsdNanos(0),
                } => return Ok(()),
                _ => return Err(AiError::Conflict),
            }
        } else {
            if self.entries.len() >= MAX_RESERVATIONS {
                return Err(AiError::BudgetExhausted);
            }
            entry.status = ReservationStatus::Settled {
                actual_cost: UsdNanos(0),
            };
            self.entries.push(entry);
        }
        self.refresh()
    }
    pub(crate) fn settle(&mut self, input: SettleBudget) -> AiResult<()> {
        input.validate()?;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.key() == &input.key)
            .ok_or(AiError::Conflict)?;
        let status = match input.actual_cost {
            Some(actual_cost) if actual_cost <= entry.maximum_cost() => {
                ReservationStatus::Settled { actual_cost }
            }
            Some(_) => return Err(AiError::InvalidOutput),
            None => ReservationStatus::Unknown,
        };
        if matches!(entry.status(), ReservationStatus::Settled { .. }) {
            return if entry.status() == &status {
                Ok(())
            } else {
                Err(AiError::Conflict)
            };
        }
        entry.status = status;
        self.refresh()
    }
    pub(crate) fn transition_from(&self, before: &Self) -> AiResult<()> {
        if self.service != before.service
            || self.account_window != before.account_window
            || self.limit != before.limit
            || self.entries.len() < before.entries.len()
            || self.entries.len() > before.entries.len() + 1
        {
            return Err(AiError::Conflict);
        }
        for (old, new) in before.entries.iter().zip(&self.entries) {
            if !old.same_reservation(new)
                || (matches!(old.status(), ReservationStatus::Settled { .. })
                    && old.status() != new.status())
                || (old.status() == &ReservationStatus::Unknown
                    && new.status() == &ReservationStatus::Reserved)
            {
                return Err(AiError::Conflict);
            }
        }
        Ok(())
    }
}
impl Validated for BudgetRecord {
    fn validate(&self) -> AiResult<()> {
        self.service.validate_service()?;
        if self.version != 1
            || !valid_name(&self.account_window, 64)
            || self.entries.len() > MAX_RESERVATIONS
        {
            return Err(AiError::InvalidRequest);
        }
        for (index, entry) in self.entries.iter().enumerate() {
            entry.validate()?;
            if entry.key().account_window() != self.account_window
                || self.entries[..index]
                    .iter()
                    .any(|old| old.key().same_dispatch(entry.key()))
            {
                return Err(AiError::InvalidRequest);
            }
        }
        if self.totals()? != (self.reserved, self.settled)
            || u128::from(self.reserved.0) + u128::from(self.settled.0) > u128::from(self.limit.0)
        {
            return Err(AiError::BudgetExhausted);
        }
        Ok(())
    }
}
impl fmt::Debug for BudgetRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BudgetRecord")
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, rom::Resource)]
#[resource(name = "ai_budgets")]
pub struct AiBudget {
    encoded: String,
}
impl AiBudget {
    pub fn new(
        service: &rom::Actor,
        account_window: impl Into<String>,
        limit: UsdNanos,
    ) -> AiResult<Self> {
        let record = BudgetRecord {
            version: 1,
            service: OwnerIdentity::from_actor(service)?,
            account_window: account_window.into(),
            limit,
            reserved: UsdNanos(0),
            settled: UsdNanos(0),
            entries: Vec::new(),
        };
        Ok(Self {
            encoded: codec::encode(&record)?,
        })
    }
    pub fn record(&self) -> AiResult<BudgetRecord> {
        codec::decode(&self.encoded)
    }
    pub(crate) fn store(&mut self, record: BudgetRecord) -> AiResult<()> {
        self.encoded = codec::encode(&record)?;
        Ok(())
    }
    pub fn definition_for(service: &rom::Actor) -> AiResult<rom::Definition<Self>> {
        use rom::Resource;
        let service = OwnerIdentity::from_actor(service)?;
        service.validate_service()?;
        Ok(Self::definition()
            .policy(|actor, _, row| {
                row.record()
                    .is_ok_and(|record| record.service.matches(actor))
            })
            .allow_all_fields()
            .validate_transition(move |actor, before, after| {
                if !service.matches(actor) {
                    return Err(rom::Error::Denied);
                }
                let after = after
                    .ok_or(rom::Error::Denied)?
                    .record()
                    .map_err(codec::rom_error)?;
                if after.service != service {
                    return Err(rom::Error::Denied);
                }
                if let Some(before) = before {
                    after
                        .transition_from(&before.record().map_err(codec::rom_error)?)
                        .map_err(codec::rom_error)?;
                } else if !after.entries.is_empty() {
                    return Err(rom::Error::Denied);
                }
                Ok(())
            })
            .action(super::actions::RESERVE_BUDGET)
            .action(super::actions::SETTLE_BUDGET)
            .action(super::unstarted::TOMBSTONE_UNSTARTED))
    }
}
impl fmt::Debug for AiBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AiBudget { .. }")
    }
}
