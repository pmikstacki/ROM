use super::{StorageFixture, scenario::Record};
use crate::{
    ConformanceResult,
    error::{check, observe},
};
use rom::{Actor, Command, Resource, Runtime};

fn runtime(storage: std::sync::Arc<dyn rom::Storage>) -> rom::Result<Runtime> {
    Runtime::builder()
        .resource(
            Record::definition()
                .allow_all_fields()
                .policy(|_, _, _| true),
        )
        .build(storage, Runtime::shared_cpu_pool(2)?)
}
pub(super) async fn run(fixture: &mut dyn StorageFixture) -> ConformanceResult {
    let storage = fixture.storage();
    super::assertions::fresh(fixture)?;
    let runtime = observe(runtime(storage.clone()), "storage.runtime.build")?;
    let actor = Actor::trusted("host", "owner");
    let create = Command::create("one", Record { done: false }).idempotency("create");
    let update = Command::replace("one", Record { done: true })
        .at_revision(1)
        .idempotency("update");
    let result = async {
        check(
            observe(
                runtime.execute(&actor, create.clone()).await,
                "storage.runtime.create",
            )?
            .revision
                == 1,
            "storage.runtime.create.revision",
        )?;
        check(
            observe(
                runtime.execute(&actor, create).await,
                "storage.runtime.replay",
            )?
            .revision
                == 1,
            "storage.runtime.replay.revision",
        )?;
        check(
            observe(
                runtime.execute(&actor, update.clone()).await,
                "storage.runtime.update",
            )?
            .revision
                == 2,
            "storage.runtime.update.revision",
        )?;
        check(
            observe(fixture.facts(), "storage.facts")?.counts == [1, 2, 2, 0],
            "storage.runtime.counts",
        )
    }
    .await;
    let shutdown = observe(runtime.shutdown().await, "storage.runtime.shutdown");
    drop(runtime);
    drop(storage);
    result?;
    shutdown?;
    let before = observe(fixture.facts(), "storage.facts")?;
    observe(fixture.reopen(), "storage.runtime.reopen")?;
    let runtime = observe(self::runtime(fixture.storage()), "storage.runtime.rebuild")?;
    let result = async {
        check(
            observe(
                runtime.execute(&actor, update).await,
                "storage.runtime.reopen.replay",
            )?
            .revision
                == 2,
            "storage.runtime.reopen.revision",
        )?;
        check(
            observe(fixture.facts(), "storage.facts")? == before,
            "storage.runtime.reopen.unchanged",
        )
    }
    .await;
    let shutdown = observe(runtime.shutdown().await, "storage.runtime.shutdown");
    drop(runtime);
    result?;
    shutdown
}
