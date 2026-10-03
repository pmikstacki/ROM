//! Explicit local operator recovery followed by an ordinary compensation action.
use crate::compensation::{RELEASE, RESERVE, ReserveInput, Stock};
use crate::{DISPLAY, Notices, bootstrap, bootstrap_actor, declarations, identity};
use rom::operator::*;
use rom::{Actor, AuthorizationRead, Command, Error, Resource, Result, Runtime, Storage};
use std::{path::Path, sync::Arc};

struct LocalOperator;
impl OperatorAuthorizer for LocalOperator {
    fn authorize(
        &self,
        actor: &Actor,
        _: OperatorAccess,
        _: Option<&WorkScope>,
        _: &mut dyn AuthorizationRead,
    ) -> Result<()> {
        if identity::same_principal(actor, &bootstrap_actor()) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
fn require(condition: bool, reason: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::invalid("operator journey", reason))
    }
}
fn open(redb: bool, path: &Path, notices: Notices) -> Result<Runtime> {
    let store: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path)?)
    };
    declarations(notices)?
        .operator_authorizer(Arc::new(LocalOperator))
        .build(store, Runtime::shared_cpu_pool(2)?)
}
fn query() -> WorkQuery {
    WorkQuery {
        state: None,
        category: Some(WorkCategory::Notification),
        definition: Some("local-completions".into()),
        limit: 8,
        cursor: None,
    }
}

/// Work authority is revoked in one process lifetime, then restored by explicit host restart.
/// This synthetic host policy does not grant operator access to normal demo sessions.
pub async fn run(redb: bool) -> crate::smoke::SmokeResult<()> {
    let scratch = crate::scratch::Scratch::new("operator")?;
    let path = scratch.0.join("db");
    let notices = Notices::default();
    let actor = bootstrap_actor();
    let runtime = open(redb, &path, notices.clone())?;
    bootstrap(&runtime).await?;
    crate::reference::drain(&runtime).await?;
    runtime
        .execute(
            &actor,
            Command::action(
                "workshop-stock",
                RESERVE,
                ReserveInput {
                    token: "operator-case".into(),
                    quantity: 2,
                },
            )
            .at_revision(1)
            .idempotency("operator-reserve"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::action("workshop", DISPLAY, "Operator recovery".into())
                .at_revision(1)
                .idempotency("operator-display"),
        )
        .await?;
    runtime.revoke(&identity::service());
    crate::reference::drain(&runtime).await?;
    let page = runtime.work_list(&actor, query()).await?;
    require(page.records.len() == 1, "one notification")?;
    require(
        page.records[0].state == WorkStatus::Stopped(rom::StopReason::Denied),
        "service revocation stops delivery",
    )?;
    require(
        notices.lock().map_err(|_| Error::Panicked)?.is_empty(),
        "denied work was not delivered",
    )?;
    let request = WorkControlRequest {
        handle: page.records[0].handle.clone(),
        expected: page.records[0].version.clone(),
        key: "operator-retry".into(),
        retry_epoch: 0,
        operation: WorkControlOperation::Retry,
    };
    runtime.shutdown().await?;
    drop(runtime);

    let runtime = open(redb, &path, notices.clone())?;
    require(
        matches!(
            runtime.work_list(&crate::session_actor(), query()).await,
            Err(Error::Denied)
        ),
        "ordinary session has no operator authority",
    )?;
    let before = runtime
        .journal(&actor, crate::Dashboard::KIND, None)
        .await?
        .events
        .len();
    let result = runtime.work_control(&actor, request.clone()).await?;
    require(
        result.outcome == WorkControlOutcome::Scheduled && !result.replayed,
        "unchanged work scheduled",
    )?;
    require(
        runtime
            .work_control(&actor, request.clone())
            .await?
            .replayed,
        "same request replays receipt",
    )?;
    crate::reference::drain(&runtime).await?;
    require(
        notices.lock().map_err(|_| Error::Panicked)?.len() == 1,
        "one recovered delivery",
    )?;
    require(
        runtime.work_read(&actor, request.handle).await?.state == WorkStatus::Done,
        "delivery completed",
    )?;
    require(
        runtime
            .journal(&actor, crate::Dashboard::KIND, None)
            .await?
            .events
            .len()
            == before,
        "operator control creates no Resource event",
    )?;

    // The host now explicitly chooses a domain compensation. Recovery did not infer it.
    let stock = runtime.read::<Stock>(&actor, "workshop-stock").await?;
    require(
        stock
            .value
            .as_ref()
            .is_some_and(|s| s.reservations.get("operator-case") == Some(&2)),
        "retry did not compensate automatically",
    )?;
    let compensation = Command::action("workshop-stock", RELEASE, "operator-case".into())
        .at_revision(stock.revision)
        .idempotency("operator-release");
    runtime.execute(&actor, compensation.clone()).await?;
    runtime.execute(&actor, compensation).await?;
    let after = runtime.read::<Stock>(&actor, "workshop-stock").await?;
    require(
        after.revision == stock.revision + 1
            && after.value.is_some_and(|s| s.reservations.is_empty()),
        "ordinary idempotent compensation",
    )?;
    runtime.shutdown().await?;
    Ok(())
}
