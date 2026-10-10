//! Exact canonical JSON contributions for affected work and root entries.
//! Prior contributions must come from the caller's coherent transaction read.
//! Byte totals alone cannot authenticate a supplied key or prior record.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryBytes {
    pub current: usize,
    pub reserved: usize,
}
impl EntryBytes {
    fn validate(self) -> Result<()> {
        if self.current == 0 || self.reserved == 0 {
            return Err(Error::Storage);
        }
        Ok(())
    }
}

fn entry_length(id: &str, value: &impl serde::Serialize) -> Result<usize> {
    let key_bytes = serde_json::to_vec(id).map_err(|_| Error::Storage)?.len();
    let value_bytes = serde_json::to_vec(value).map_err(|_| Error::Storage)?.len();
    key_bytes
        .checked_add(1)
        .and_then(|n| n.checked_add(value_bytes))
        .ok_or(Error::TooLarge)
}

pub fn entry_for_root(id: &str, used: u32) -> Result<EntryBytes> {
    Ok(EntryBytes {
        current: entry_length(id, &used)?,
        reserved: entry_length(id, &u32::MAX)?,
    })
}

pub fn entry_for_work(id: &str, record: &WorkRecord) -> Result<EntryBytes> {
    let current = entry_length(id, record)?;
    let mut reserved = record.clone();
    reserved.attempts = u32::MAX;
    reserved.generation = u64::MAX;
    reserved.revision = u64::MAX;
    reserved.due = u64::MAX;
    reserved.state = WorkState::Leased {
        until: u64::MAX,
        generation: u64::MAX,
        resolution_only: Some(StopReason::DefinitionChanged),
    };
    reserved.delivery = Some(DeliveryOutcome::Retryable);
    Ok(EntryBytes {
        current,
        reserved: entry_length(id, &reserved)?,
    })
}

/// Persisted numerical map contributions. Reconstruction does not authorize work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAccounting {
    pub entries: usize,
    pub current_entries_bytes: usize,
    pub reserved_entries_bytes: usize,
}
/// Native header DTO; compare with canonical records during import/integrity checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkAccounting {
    pub work: MapAccounting,
    pub roots: MapAccounting,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MapBytes {
    entries: usize,
    current: usize,
    reserved: usize,
}
impl MapBytes {
    fn replace(&mut self, before: Option<EntryBytes>, after: Option<EntryBytes>) -> Result<()> {
        if let Some(before) = before {
            before.validate()?;
            self.entries = self.entries.checked_sub(1).ok_or(Error::Storage)?;
            self.current = self
                .current
                .checked_sub(before.current)
                .ok_or(Error::Storage)?;
            self.reserved = self
                .reserved
                .checked_sub(before.reserved)
                .ok_or(Error::Storage)?;
        }
        if self.entries == 0 && (self.current != 0 || self.reserved != 0) {
            return Err(Error::Storage);
        }
        if let Some(after) = after {
            after.validate()?;
            self.entries = self.entries.checked_add(1).ok_or(Error::TooLarge)?;
            self.current = self
                .current
                .checked_add(after.current)
                .ok_or(Error::TooLarge)?;
            self.reserved = self
                .reserved
                .checked_add(after.reserved)
                .ok_or(Error::TooLarge)?;
        }
        Ok(())
    }
    fn totals(self) -> Result<EntryBytes> {
        let commas = self.entries.saturating_sub(1);
        Ok(EntryBytes {
            current: self.current.checked_add(commas).ok_or(Error::TooLarge)?,
            reserved: self.reserved.checked_add(commas).ok_or(Error::TooLarge)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerBytes {
    envelope: usize,
    policy: Option<ReactionLimits>,
    work: MapBytes,
    roots: MapBytes,
}
impl LedgerBytes {
    pub fn from_parts(limits: Option<&ReactionLimits>, parts: WorkAccounting) -> Result<Self> {
        fn map(parts: MapAccounting) -> Result<MapBytes> {
            if parts.entries == 0 {
                if parts.current_entries_bytes != 0 || parts.reserved_entries_bytes != 0 {
                    return Err(Error::Storage);
                }
            } else {
                let minimum = parts.entries.checked_mul(4).ok_or(Error::TooLarge)?;
                if parts.current_entries_bytes < minimum || parts.reserved_entries_bytes < minimum {
                    return Err(Error::Storage);
                }
            }
            Ok(MapBytes {
                entries: parts.entries,
                current: parts.current_entries_bytes,
                reserved: parts.reserved_entries_bytes,
            })
        }
        if let Some(limits) = limits {
            limits.validate()?;
        }
        let mut result = Self::empty(limits)?;
        result.work = map(parts.work)?;
        result.roots = map(parts.roots)?;
        result.totals()?;
        Ok(result)
    }
    pub fn parts(&self) -> WorkAccounting {
        fn map(bytes: MapBytes) -> MapAccounting {
            MapAccounting {
                entries: bytes.entries,
                current_entries_bytes: bytes.current,
                reserved_entries_bytes: bytes.reserved,
            }
        }
        WorkAccounting {
            work: map(self.work),
            roots: map(self.roots),
        }
    }
    pub fn validate_policy(&self, limits: Option<&ReactionLimits>) -> Result<()> {
        if self.policy.as_ref() != limits {
            return Err(Error::Unsupported(
                "accounting policy differs from frozen envelope".into(),
            ));
        }
        Ok(())
    }
    pub fn empty(limits: Option<&ReactionLimits>) -> Result<Self> {
        let ledger = WorkLedger {
            limits: limits.cloned(),
            ..WorkLedger::default()
        };
        Ok(Self {
            envelope: serde_json::to_vec(&ledger)
                .map_err(|_| Error::Storage)?
                .len(),
            policy: limits.cloned(),
            work: MapBytes::default(),
            roots: MapBytes::default(),
        })
    }
    pub fn replace_work(
        &mut self,
        before: Option<EntryBytes>,
        after: Option<EntryBytes>,
    ) -> Result<()> {
        let mut next = self.clone();
        next.work.replace(before, after)?;
        next.totals()?;
        *self = next;
        Ok(())
    }
    pub fn replace_root(
        &mut self,
        before: Option<EntryBytes>,
        after: Option<EntryBytes>,
    ) -> Result<()> {
        let mut next = self.clone();
        next.roots.replace(before, after)?;
        next.totals()?;
        *self = next;
        Ok(())
    }
    pub fn check_bounds(&self, limits: &ReactionLimits) -> Result<()> {
        self.validate_policy(Some(limits))?;
        let totals = self.totals()?;
        if self.work.entries > limits.max_records
            || totals.current > limits.max_bytes
            || totals.reserved > limits.max_bytes
        {
            return Err(Error::Overloaded);
        }
        Ok(())
    }
    pub fn totals(&self) -> Result<EntryBytes> {
        let work = self.work.totals()?;
        let roots = self.roots.totals()?;
        Ok(EntryBytes {
            current: self
                .envelope
                .checked_add(work.current)
                .and_then(|n| n.checked_add(roots.current))
                .ok_or(Error::TooLarge)?,
            reserved: self
                .envelope
                .checked_add(work.reserved)
                .and_then(|n| n.checked_add(roots.reserved))
                .ok_or(Error::TooLarge)?,
        })
    }
}
