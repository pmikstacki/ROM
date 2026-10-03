//! Current host authority for each independent operator capability and scope.
use super::*;
use crate::{Actor, AuthorizationRead, Error, Result, Runtime, policy};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorAccess {
    Inspect,
    Retry,
    Reconcile,
}
/// `None` checks coarse capability; `Some` checks one redacted work scope.
/// Authorizers run in bounded supervised I/O, outside native storage guards.
pub trait OperatorAuthorizer: Send + Sync + 'static {
    fn authorize(
        &self,
        actor: &Actor,
        access: OperatorAccess,
        scope: Option<&WorkScope>,
        reads: &mut dyn AuthorizationRead,
    ) -> Result<()>;
}
impl<F> OperatorAuthorizer for F
where
    F: Fn(&Actor, OperatorAccess, Option<&WorkScope>, &mut dyn AuthorizationRead) -> Result<()>
        + Send
        + Sync
        + 'static,
{
    fn authorize(
        &self,
        actor: &Actor,
        access: OperatorAccess,
        scope: Option<&WorkScope>,
        reads: &mut dyn AuthorizationRead,
    ) -> Result<()> {
        self(actor, access, scope, reads)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorLimits {
    pub responses: WorkResponseLimits,
    pub max_snapshot_records: usize,
    pub max_snapshot_bytes: usize,
}
impl Default for OperatorLimits {
    fn default() -> Self {
        Self {
            responses: WorkResponseLimits::default(),
            max_snapshot_records: 2048,
            max_snapshot_bytes: 4 * 1024 * 1024,
        }
    }
}
impl OperatorLimits {
    pub fn validate(&self) -> Result<()> {
        self.responses.validate()?;
        if self.max_snapshot_records == 0 || self.max_snapshot_bytes == 0 {
            return Err(Error::TooLarge);
        }
        self.max_snapshot_records
            .checked_mul(std::mem::size_of::<crate::WorkRecord>())
            .ok_or(Error::TooLarge)?;
        Ok(())
    }
}
impl Runtime {
    pub(crate) fn authorize_operator(
        &self,
        actor: &Actor,
        access: OperatorAccess,
        scope: Option<&WorkScope>,
    ) -> Result<()> {
        self.check_authority(actor).map_err(authority_error)?;
        let policy = self.0.operator_authorizer.as_ref().ok_or(Error::Denied)?;
        let result = catch_unwind(AssertUnwindSafe(|| {
            policy.authorize(
                actor,
                access,
                scope,
                &mut policy::GateRead {
                    storage: self.0.storage.as_ref(),
                    reads: 8,
                    bytes: self.0.limits.command_bytes,
                },
            )
        }))
        .map_err(|_| Error::Panicked)?;
        result.map_err(|_| Error::Denied)?;
        self.check_authority(actor).map_err(authority_error)
    }
}
pub(crate) type Authorizer = Option<Arc<dyn OperatorAuthorizer>>;
/// Host callback diagnostics never become operator response bodies.
pub(super) fn public_error(error: Error) -> Error {
    match error {
        Error::Invalid { kind, field }
            if kind == "retry" && (field == "retry epoch" || field == "epoch floors") =>
        {
            Error::Invalid { kind, field }
        }
        Error::Invalid { .. } | Error::Duplicate(_) => Error::Denied,
        Error::Unsupported(_) => Error::Unsupported("operator operation unavailable".into()),
        error => error,
    }
}
pub(super) fn authority_error(error: Error) -> Error {
    match error {
        Error::Closed | Error::Overloaded | Error::Panicked => error,
        _ => Error::Denied,
    }
}
