use rom::{Actor, Command, Runtime};
use rom_consumer::{COMPLETE, ENABLE, Setting, Task, declarations};
use rom_sqlite::Sqlite;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(Sqlite::open(":memory:")?);
    let rom = declarations().build(store.clone(), Runtime::shared_cpu_pool(2)?)?;
    let actor = Actor::trusted("local", "alice");
    let mut open = rom.live(&actor, Task::done_field().equals(false))?;
    assert!(open.changed().await?.is_empty());
    rom.execute(
        &actor,
        Command::create(
            "one",
            Task {
                owner: "alice".into(),
                title: "Use one declaration".into(),
                done: false,
                note: None,
            },
        )
        .idempotency("create-task"),
    )
    .await?;
    println!("open task membership: {}", open.changed().await?.len());
    rom.execute(
        &actor,
        Command::action("one", COMPLETE, ())
            .at_revision(1)
            .idempotency("complete-task"),
    )
    .await?;
    println!("after complete: {}", open.changed().await?.len());
    rom.execute(
        &actor,
        Command::create(
            "preferences",
            Setting {
                owner: "alice".into(),
                enabled: false,
                attempts: 0,
            },
        )
        .idempotency("create-setting"),
    )
    .await?;
    rom.execute(
        &actor,
        Command::action("preferences", ENABLE, true)
            .at_revision(1)
            .idempotency("enable-setting"),
    )
    .await?;
    println!(
        "settings: {:?}",
        rom.query(&actor, &Setting::enabled_field().equals(true))?
    );
    println!("state/event/receipt/effect counts: {:?}", store.counts()?);
    rom.shutdown().await?;
    Ok(())
}
