//! Supervised authorized inspection and atomic unchanged-intent recovery.
use super::*;
use crate::*;
impl Runtime {
    pub async fn operator_capabilities(&self, actor: &Actor) -> Result<OperatorCapabilities> {
        let actor = actor.clone();
        let a = actor.clone();
        self.operator_observe(&actor, move |runtime| {
            let mut capabilities = OperatorCapabilities::default();
            if runtime.0.storage.supports_operator() {
                for (access, flag) in [
                    (OperatorAccess::Inspect, &mut capabilities.inspect),
                    (OperatorAccess::Retry, &mut capabilities.retry),
                    (OperatorAccess::Reconcile, &mut capabilities.reconcile),
                ] {
                    match runtime.authorize_operator(&a, access, None) {
                        Ok(()) => *flag = true,
                        Err(Error::Denied) => {}
                        Err(error) => return Err(error),
                    }
                }
            }
            capabilities.validate(&runtime.0.operator_limits.responses)?;
            Ok(capabilities)
        })
        .await
        .map_err(super::authorization::public_error)
    }
    pub async fn work_list(&self, actor: &Actor, query: WorkQuery) -> Result<WorkPage> {
        query.validate(&self.0.operator_limits.responses)?;
        let a = actor.clone();
        self.operator_observe(actor, move |runtime| {
            runtime.authorize_operator(&a, OperatorAccess::Inspect, None)?;
            runtime.require_operator_storage()?;
            runtime.operator_page(&a, &runtime.operator_snapshot()?, &query)
        })
        .await
        .map_err(super::authorization::public_error)
    }
    pub async fn work_read(&self, actor: &Actor, handle: WorkHandle) -> Result<WorkView> {
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            runtime.authorize_operator(&a, OperatorAccess::Inspect, None)?;
            runtime.require_operator_storage()?;
            let snapshot = runtime.operator_snapshot()?;
            let record = snapshot
                .records
                .iter()
                .find(|record| WorkHandle::from_work_id(&record.pending.id) == handle)
                .ok_or(Error::Denied)?;
            let scope = WorkScope::from_record(record)?;
            runtime.authorize_operator(&a, OperatorAccess::Inspect, Some(&scope))?;
            let view = super::projection::view(&snapshot, record, scope);
            view.validate(&runtime.0.operator_limits.responses)?;
            Ok(view)
        })
        .await
        .map_err(super::authorization::public_error)
    }
    /// Admission errors precede commit. Unknown requires an exact request retry;
    /// it can mean a committed control whose reply failed validation or disclosure.
    pub async fn work_control(
        &self,
        actor: &Actor,
        request: WorkControlRequest,
    ) -> Result<WorkControlResult> {
        self.check_actor(actor)?;
        request.validate()?;
        let a = actor.clone();
        self.io(move |runtime| runtime.execute_operator_control(&a, &request))
            .await
            .map_err(super::authorization::public_error)
    }
    async fn operator_observe<
        T: Send + 'static,
        F: Fn(&Runtime) -> Result<T> + Send + Sync + 'static,
    >(
        &self,
        actor: &Actor,
        operation: F,
    ) -> Result<T> {
        // Wrapping operation errors keeps authority diagnostics separate from safe API errors.
        self.observe(actor, move |runtime| Ok(operation(runtime)))
            .await
            .map_err(super::authorization::authority_error)?
            .map_err(super::authorization::public_error)
    }
    pub(super) fn require_operator_storage(&self) -> Result<()> {
        if self.0.storage.supports_operator() {
            Ok(())
        } else {
            Err(Error::Unsupported("atomic operator work recovery".into()))
        }
    }
}
