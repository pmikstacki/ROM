//! Shared redacted work state for inspection and retained control evidence.
use super::WorkStatus;
use crate::WorkState;

pub(crate) fn work_status(state: &WorkState) -> WorkStatus {
    match state {
        WorkState::Pending => WorkStatus::Pending,
        WorkState::Leased { .. } => WorkStatus::Leased,
        WorkState::AwaitingReconciliation => WorkStatus::AwaitingReconciliation,
        WorkState::Done => WorkStatus::Done,
        WorkState::Stopped(reason) => WorkStatus::Stopped(reason.clone()),
    }
}
