//! Shared dependency selection. Native adapters own storage and publication.
use crate::{BackupLimits, RetentionPolicy, RetentionReport, Snapshot, codec, maintenance_limits};
use rom::{Error, Key, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Build a bounded candidate without altering the source or executing callbacks.
/// Publish through a native adapter to fence old cursors and work claims.
pub fn retain_snapshot(
    mut snapshot: Snapshot,
    policy: &RetentionPolicy,
    limits: BackupLimits,
) -> Result<(Snapshot, RetentionReport)> {
    maintenance_limits::check_snapshot(&snapshot, limits)?;
    snapshot.validate()?;
    codec::encode(policy, limits.max_bytes)?;
    if policy
        .settled_effects
        .len()
        .checked_add(policy.tombstones.len())
        .ok_or(Error::TooLarge)?
        > limits.max_records
    {
        return Err(Error::TooLarge);
    }
    let before = (
        snapshot.receipts.len(),
        snapshot.events.len(),
        snapshot.effects.len(),
        snapshot.rows.len(),
    );
    let through = policy
        .journal_through
        .unwrap_or(snapshot.state.journal_floor());
    let state_report =
        snapshot
            .state
            .apply_retention(policy.epochs, through, before.0, before.2)?;
    let removed: BTreeSet<_> = state_report.journal_identities.into_iter().collect();
    snapshot.events.retain(|(id, _)| !removed.contains(id));
    settle_effects(&mut snapshot, policy)?;
    purge_tombstones(&mut snapshot, policy)?;
    retain_receipts(&mut snapshot, policy);
    snapshot.state.apply_retention(
        policy.epochs,
        through,
        snapshot.receipts.len(),
        snapshot.effects.len(),
    )?;
    snapshot.validate()?;
    maintenance_limits::check_snapshot(&snapshot, limits)?;
    let report = RetentionReport {
        retry_epochs: policy.epochs,
        receipts_removed: before.0 - snapshot.receipts.len(),
        receipts_retained: snapshot.receipts.len(),
        events_removed: before.1 - snapshot.events.len(),
        effects_removed: before.2 - snapshot.effects.len(),
        work_records_removed: state_report.work_records,
        work_roots_removed: state_report.work_roots,
        tombstones_removed: before.3 - snapshot.rows.len(),
    };
    Ok((snapshot, report))
}

fn settle_effects(snapshot: &mut Snapshot, policy: &RetentionPolicy) -> Result<()> {
    let expired: BTreeSet<_> = snapshot
        .receipts
        .iter()
        .filter(|r| r.retry_epoch < policy.epochs.replay_floor)
        .map(|r| r.identity.as_str())
        .collect();
    let effects: BTreeSet<_> = snapshot
        .effects
        .iter()
        .map(|e| e.identity.as_str())
        .collect();
    if policy
        .settled_effects
        .iter()
        .any(|id| !expired.contains(id.as_str()) || !effects.contains(id.as_str()))
    {
        return Err(Error::Conflict);
    }
    snapshot
        .effects
        .retain(|e| !policy.settled_effects.contains(&e.identity));
    Ok(())
}

fn purge_tombstones(snapshot: &mut Snapshot, policy: &RetentionPolicy) -> Result<()> {
    if policy.tombstones.is_empty() {
        return Ok(());
    }
    // Opaque invocation/notification payloads can depend on rows other than their
    // direct target. Do not infer their data dependencies from arbitrary JSON.
    if !snapshot.state.work.records().is_empty() {
        return Err(Error::Conflict);
    }
    let rows: BTreeMap<_, _> = snapshot.rows.iter().map(|r| (&r.key, r)).collect();
    let event_ids: BTreeSet<_> = snapshot.events.iter().map(|(id, _)| id.as_str()).collect();
    let effect_ids: BTreeSet<_> = snapshot
        .effects
        .iter()
        .map(|e| e.identity.as_str())
        .collect();
    let blocked: BTreeSet<_> = snapshot
        .references
        .iter()
        .flat_map(|e| [&e.source, &e.target])
        .chain(
            snapshot
                .receipts
                .iter()
                .filter(|r| {
                    r.retry_epoch >= policy.epochs.replay_floor
                        || event_ids.contains(r.identity.as_str())
                        || effect_ids.contains(r.identity.as_str())
                })
                .map(|r| &r.row.key),
        )
        .collect();
    for key in &policy.tombstones {
        if rows.get(key).is_none_or(|r| r.value.is_some()) || blocked.contains(key) {
            return Err(Error::Conflict);
        }
    }
    snapshot
        .rows
        .retain(|r| !policy.tombstones.contains(&r.key));
    snapshot
        .receipts
        .retain(|r| !policy.tombstones.contains(&r.row.key));
    Ok(())
}

fn retain_receipts(snapshot: &mut Snapshot, policy: &RetentionPolicy) {
    let mut pinned: BTreeSet<String> = snapshot
        .events
        .iter()
        .map(|(id, _)| id.clone())
        .chain(snapshot.effects.iter().map(|e| e.identity.clone()))
        .chain(
            snapshot
                .receipts
                .iter()
                .filter(|r| r.retry_epoch >= policy.epochs.replay_floor)
                .map(|r| r.identity.clone()),
        )
        .collect();
    let rows: BTreeMap<_, _> = snapshot.rows.iter().map(|r| (&r.key, r)).collect();
    let mut proofs: BTreeMap<&Key, (&str, bool)> = BTreeMap::new();
    for receipt in &snapshot.receipts {
        if rows
            .get(&receipt.row.key)
            .is_some_and(|r| **r == receipt.row)
        {
            let is_pinned = pinned.contains(&receipt.identity);
            proofs
                .entry(&receipt.row.key)
                .and_modify(|(id, covered)| {
                    if receipt.identity.as_str() < *id {
                        *id = &receipt.identity;
                    }
                    *covered |= is_pinned;
                })
                .or_insert((&receipt.identity, is_pinned));
        }
    }
    for (id, covered) in proofs.into_values() {
        if !covered {
            pinned.insert(id.to_owned());
        }
    }
    snapshot.receipts.retain(|r| pinned.contains(&r.identity));
}
