//! Durable proof and account tombstone for a revision-fenced, never-started attempt.
use super::{
    AiBudget, AiRun, HostState, ReservationEntry, ReservationStatus, RunRecord, RunState,
    codec::{self, Validated, input},
    projection::map_error,
};
use crate::{AiError, AiResult, UsdNanos};
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnstartedProof {
    version: u32,
    prepared: bool,
    key: super::ReservationKey,
}
fn terminal(record: &RunRecord) -> bool {
    matches!(record.state(), RunState::Failed | RunState::Cancelled)
}
fn selected(before: &RunRecord) -> Option<(bool, ReservationEntry)> {
    if !super::route_continuation::enabled(before) {
        return None;
    }
    match before.state() {
        RunState::Prepared => before.active_attempt().cloned().map(|e| (true, e)),
        RunState::Waiting { .. } => super::route_continuation::pending(before)
            .and_then(|c| c.staged.as_ref())
            .map(|p| (false, p.entry.clone())),
        _ => None,
    }
}
pub(crate) fn expected(before: &RunRecord, after: &RunRecord) -> Option<UnstartedProof> {
    if before.unstarted.is_some() {
        return before.unstarted.clone();
    }
    if !terminal(after) {
        return None;
    }
    selected(before).map(|(prepared, entry)| UnstartedProof {
        version: 1,
        prepared,
        key: entry.key().clone(),
    })
}
pub(crate) fn retain(before: &RunRecord, after: &mut RunRecord) {
    after.unstarted = expected(before, after);
}
fn proof_entry<'a>(
    record: &'a RunRecord,
    proof: &UnstartedProof,
) -> AiResult<&'a ReservationEntry> {
    let entry = if proof.prepared {
        record.active_attempt()
    } else {
        super::route_continuation::pending(record)
            .and_then(|c| c.staged.as_ref())
            .map(|p| &p.entry)
    }
    .ok_or(AiError::Conflict)?;
    if entry.key() != &proof.key {
        return Err(AiError::Conflict);
    }
    Ok(entry)
}
pub(crate) fn validate(record: &RunRecord) -> AiResult<()> {
    let Some(proof) = &record.unstarted else {
        return Ok(());
    };
    let entry = proof_entry(record, proof)?;
    entry.validate()?;
    if proof.version != 1
        || !terminal(record)
        || !super::route_continuation::enabled(record)
        || entry.status() != &ReservationStatus::Reserved
        || entry.key().run_id() != record.run_id()
    {
        return Err(AiError::Conflict);
    }
    Ok(())
}
/// Called after the ordinary owner-resolution boundary. No provider I/O occurs here.
pub(crate) async fn settle(
    state: &HostState,
    runtime: &rom::Runtime,
    record: &RunRecord,
) -> AiResult<()> {
    validate(record)?;
    let Some(proof) = &record.unstarted else {
        return Ok(());
    };
    // Re-read the actual terminal record. A Prepared snapshot or a guessed entry is not proof.
    let actual = runtime
        .read::<AiRun>(&state.service, record.run_id())
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Conflict)?
        .record()?;
    if actual.unstarted != record.unstarted || !terminal(&actual) {
        return Err(AiError::Conflict);
    }
    let entry = proof_entry(&actual, proof)?;
    let snapshot = runtime
        .read::<AiBudget>(&state.service, entry.key().account_window())
        .await
        .map_err(map_error)?;
    let account = snapshot.value.ok_or(AiError::Denied)?.record()?;
    if account.entries().iter().any(|e| {
        e.same_reservation(entry)
            && e.status()
                == &ReservationStatus::Settled {
                    actual_cost: UsdNanos(0),
                }
    }) {
        return Ok(());
    }
    let command = rom::Command::action(
        entry.key().account_window(),
        TOMBSTONE_UNSTARTED,
        TombstoneUnstarted {
            entry: entry.clone(),
        },
    )
    .at_revision(snapshot.revision)
    .idempotency(&format!(
        "ai-unstarted-account-v1-{}-{}",
        entry.prepared().identity(),
        snapshot.revision
    ));
    // The exact account CAS either excludes all older reservations or races and retries
    // through the durable terminal record. Absence never means cleanup is complete.
    for retry in 0..2 {
        match runtime.execute(&state.service, command.clone()).await {
            Ok(_) => return Ok(()),
            Err(rom::Error::Unknown) if retry == 0 => continue,
            Err(error) => return Err(map_error(error)),
        }
    }
    Err(AiError::UnknownOutcome)
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TombstoneUnstarted {
    pub(crate) entry: ReservationEntry,
}
impl Validated for TombstoneUnstarted {
    fn validate(&self) -> AiResult<()> {
        self.entry.validate()?;
        if self.entry.status() != &ReservationStatus::Reserved {
            return Err(AiError::Conflict);
        }
        Ok(())
    }
}
input!(TombstoneUnstarted);
pub(crate) const TOMBSTONE_UNSTARTED: rom::Action<AiBudget, TombstoneUnstarted> =
    rom::Action::new("tombstone_unstarted", |budget, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = budget.record().map_err(codec::rom_error)?;
        record
            .tombstone_unstarted(input.entry)
            .map_err(codec::rom_error)?;
        budget.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
