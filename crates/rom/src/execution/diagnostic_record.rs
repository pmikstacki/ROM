//! Fixed diagnostic observations. Records publish only at explicit post-guard boundaries.
use crate::{
    DiagnosticOutcome, DiagnosticStage, Error, Key, Result, Row, Storage,
    diagnostics::OperationRecord,
};

pub(crate) fn load(
    storage: &dyn Storage,
    key: &Key,
    recording: &mut Option<OperationRecord>,
) -> Result<Option<Row>> {
    stage(
        recording,
        DiagnosticStage::StorageRead,
        DiagnosticOutcome::Started,
        0,
    );
    let result = storage.load(key);
    if result.is_ok() {
        stage(
            recording,
            DiagnosticStage::StorageRead,
            DiagnosticOutcome::Succeeded,
            0,
        );
    }
    result
}

pub(crate) fn stage(
    recording: &mut Option<OperationRecord>,
    stage: DiagnosticStage,
    outcome: DiagnosticOutcome,
    event_count: u32,
) {
    if let Some(recording) = recording {
        recording.stage(stage, outcome, recording.elapsed_ns(), event_count);
    }
}

pub(crate) fn outcome(error: &Error) -> DiagnosticOutcome {
    match error {
        Error::Denied => DiagnosticOutcome::Denied,
        Error::Conflict | Error::IdentityMismatch => DiagnosticOutcome::Conflict,
        Error::NotCommitted => DiagnosticOutcome::NotCommitted,
        Error::Unknown | Error::Storage => DiagnosticOutcome::Unknown,
        Error::Panicked => DiagnosticOutcome::Panicked,
        Error::Overloaded => DiagnosticOutcome::Retryable,
        Error::Closed => DiagnosticOutcome::Stopped,
        Error::Unsupported(_) => DiagnosticOutcome::Permanent,
        Error::Invalid { .. }
        | Error::Duplicate(_)
        | Error::Unregistered
        | Error::IdentityExpired
        | Error::Missing
        | Error::TooLarge
        | Error::HistoryGap => DiagnosticOutcome::Invalid,
    }
}

pub(crate) fn publish(recording: Option<OperationRecord>) {
    if let Some(recording) = recording {
        recording.publish();
    }
}
