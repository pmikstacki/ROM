//! Account settlement after the checkpoint future has completed and been dropped.
use super::{HostState, callback_phase::Settlement};
use crate::AiResult;

pub(super) async fn apply(
    state: &HostState,
    runtime: &rom::Runtime,
    input: Settlement,
) -> AiResult<()> {
    match input {
        Settlement::Record {
            record,
            confirmed_nonaccepted,
        } => super::worker::settle_record(state, runtime, &record, confirmed_nonaccepted).await,
        Settlement::Run {
            run,
            confirmed_nonaccepted,
        } => super::worker::settle_run(state, runtime, &run, confirmed_nonaccepted).await,
    }
}
