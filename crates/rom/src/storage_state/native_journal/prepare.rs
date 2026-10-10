use super::*;
use crate::storage_support::work::{WorkEdit, WorkRead};
pub(in crate::storage_state) fn new_edit<'tx, R: WorkRead>(
    epochs: RetryEpochs,
    work: &'tx R,
) -> Result<WorkEdit<'tx, R>> {
    if work.header()?.retry_epochs != epochs {
        return Err(Error::Storage);
    }
    WorkEdit::new(work)
}
pub(in crate::storage_state) fn check_new_epoch<R: WorkRead>(
    epochs: RetryEpochs,
    epoch: u64,
    completed: Option<(&ClaimKey, u64)>,
    edit: &mut WorkEdit<'_, R>,
) -> Result<()> {
    let causal = if let Some((claim, now)) = completed {
        if edit.claim_retry_epoch(claim, now)? != epoch {
            return Err(Error::Conflict);
        }
        true
    } else {
        false
    };
    epochs.check(epoch, false, causal)
}
/// Prepare after receipt/revision arbitration; no method publishes either delta.
/// Arbitration must establish that the incoming receipt identity is new.
/// Untouched identity uniqueness relies on validated import and native constraints.
pub fn prepare_native_bundle(
    journal: &impl JournalRead,
    work: &impl WorkRead,
    bundle: &Bundle,
) -> Result<NativeBundleDelta> {
    let fence = journal.coherence();
    let before = journal.header()?;
    before
        .retry_epochs()
        .check(bundle.receipt.retry_epoch, true, false)?;
    let mut edit = new_edit(before.retry_epochs(), work)?;
    check_new_epoch(
        before.retry_epochs(),
        bundle.receipt.retry_epoch,
        bundle
            .completed_work
            .as_ref()
            .map(|(claim, now)| (claim, *now)),
        &mut edit,
    )?;
    if bundle
        .reactions
        .iter()
        .any(|w| w.cause.retry_epoch != bundle.receipt.retry_epoch)
    {
        return Err(Error::Conflict);
    }
    if let Some((claim, now)) = &bundle.completed_work {
        edit.complete(claim.clone(), *now)?;
    }
    let mut parts = before.parts();
    // Share the established receipt/effect overflow and overload precedence.
    let mut counters = StorageMetadata {
        retry_epochs: parts.retry_epochs,
        limits: parts.limits.clone(),
        generation: parts.generation.clone(),
        head: parts.head,
        floor: parts.head,
        receipts: parts.receipts,
        effects: parts.effects,
        events: vec![],
    }
    .reassemble(WorkLedger::default(), OperatorLedger::default());
    counters.count_bundle(bundle)?;
    parts.receipts = counters.receipts;
    parts.effects = counters.effects;
    if !bundle.reactions.is_empty() {
        if !bundle.changed {
            return Err(Error::NotCommitted);
        }
        edit.enqueue(
            bundle.reaction_limits.as_ref().ok_or(Error::NotCommitted)?,
            bundle.reactions.clone(),
        )?;
    }
    let mut facts = BTreeMap::new();
    let mut retired = vec![];
    let mut appended = None;
    if bundle.changed {
        parts.head = parts.head.checked_add(1).ok_or(Error::TooLarge)?;
        let event = JournalEvent {
            position: parts.head,
            identity: bundle.receipt.identity.clone(),
            row: bundle.receipt.row.clone(),
        };
        let encoded_bytes = serde_json::to_vec(&event)
            .map_err(|_| Error::Storage)?
            .len();
        if encoded_bytes > parts.limits.journal_bytes {
            return Err(Error::TooLarge);
        }
        let prior = journal.entry(parts.head)?;
        if prior.is_some() {
            return Err(Error::Storage);
        }
        facts.insert(parts.head, prior);
        parts.journal_records = parts
            .journal_records
            .checked_add(1)
            .ok_or(Error::TooLarge)?;
        parts.journal_bytes = parts
            .journal_bytes
            .checked_add(encoded_bytes)
            .ok_or(Error::TooLarge)?;
        let mut identities = BTreeSet::new();
        while parts.journal_records > parts.limits.journal_rows
            || parts.journal_bytes > parts.limits.journal_bytes
        {
            let position = parts.floor.checked_add(1).ok_or(Error::TooLarge)?;
            let entry = journal.entry(position)?.ok_or(Error::Storage)?;
            read::validate_entry(&before, position, &entry)?;
            if !identities.insert(entry.event.identity.clone())
                || entry.event.identity == event.identity
            {
                return Err(Error::Storage);
            }
            parts.journal_bytes = parts
                .journal_bytes
                .checked_sub(entry.encoded_bytes)
                .ok_or(Error::Storage)?;
            parts.journal_records = parts.journal_records.checked_sub(1).ok_or(Error::Storage)?;
            parts.floor = position;
            facts.insert(position, Some(entry.clone()));
            retired.push(entry);
        }
        appended = Some(JournalEntry {
            event,
            encoded_bytes,
        });
    }
    let after = MetadataHeader::from_parts(parts)?;
    let work_delta = edit.finish()?;
    if !journal.coherence().same_context(&fence)
        || !work.coherence().same_context(work_delta.coherence())
    {
        return Err(Error::Conflict);
    }
    Ok(NativeBundleDelta {
        journal: JournalDelta {
            coherence: fence,
            before,
            after,
            appended,
            retired,
            facts,
        },
        work: work_delta,
    })
}
