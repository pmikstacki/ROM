//! Opt-in bounded diagnostic records; exporters belong to the host.
mod channel;
mod core_overload;
mod correlation;

#[cfg(test)]
mod core_overload_tests;

pub(crate) use core_overload::CoreOverloadCounters;
pub use core_overload::CoreOverloadStats;
mod recording;
mod types;

pub use channel::{DiagnosticOptions, DiagnosticReader, DiagnosticSink, Diagnostics};
pub use correlation::DiagnosticKey;
pub(crate) use recording::OperationRecord;
pub use types::{
    DiagnosticEvent, DiagnosticOutcome, DiagnosticStage, DiagnosticStats, DiagnosticToken,
};

#[cfg(test)]
mod tests;
