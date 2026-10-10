//! Admission margin for handing short-lived identity evidence to ordinary work.
use crate::{AuthOperation, HostConfig};
use rom::{Error, Result};
use std::time::Duration;

pub(crate) fn validate(config: &HostConfig) -> Result<()> {
    if config.approved.is_empty() {
        return Ok(());
    }
    if config
        .proof_handoff_budget
        .is_some_and(|budget| budget.is_zero())
    {
        return Err(invalid());
    }
    budget(config, AuthOperation::GenericHttpResolver)?;
    budget(config, AuthOperation::Blob)?;
    Ok(())
}

pub(crate) fn budget(config: &HostConfig, operation: AuthOperation) -> Result<u64> {
    if config.approved.is_empty() {
        return Ok(0);
    }
    let duration = match operation {
        AuthOperation::GenericHttpResolver => config.http_limits.body_timeout,
        AuthOperation::Blob => {
            config
                .blobs
                .as_ref()
                .map_or(config.http_limits.body_timeout, |service| {
                    config
                        .http_limits
                        .body_timeout
                        .max(service.limits().staging_timeout)
                })
        }
        // Session checks and the middleware's captured stream actor preserve their
        // existing lifetime. Current streams never acquire new evidence here.
        _ => return Ok(0),
    };
    let duration = duration.max(config.proof_handoff_budget.unwrap_or(Duration::ZERO));
    let seconds = duration
        .as_secs()
        .checked_add(u64::from(duration.subsec_nanos() != 0))
        .ok_or_else(invalid)?;
    if seconds == 0 || seconds >= rom_auth::oidc::MAX_PROOF_SECONDS {
        return Err(invalid());
    }
    Ok(seconds)
}

pub(crate) fn short(now: u64, until: u64, budget: u64) -> bool {
    // Expiry is exclusive. Exact equality does not leave room beyond the budget.
    until.saturating_sub(now) <= budget
}

fn invalid() -> Error {
    Error::Invalid {
        kind: "studio-host".into(),
        field: "proof handoff budget must be positive and below the OIDC proof ceiling".into(),
    }
}
