use rom::*;
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Resource)]
#[resource(name = "migration-runtime-notes")]
struct NoteV1 {
    title: String,
    owner: String,
}
#[derive(Clone, Resource)]
#[resource(name = "migration-runtime-notes", version = 2)]
struct NoteV2 {
    label: String,
    owner: String,
    active: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "migration-runtime-totals")]
struct Total {
    count: u64,
}
const ADD: Action<Total, u64> = Action::new("add", |v, n| {
    v.count += n;
    Ok(vec![])
});
fn owner() -> Actor {
    Actor::trusted("migrations", "owner")
}
fn worker() -> Actor {
    Actor::trusted("migrations", "worker").with_kind(PrincipalKind::Service)
}
fn total() -> Definition<Total> {
    Total::definition()
        .policy(|_, _, _| true)
        .allow_all_fields()
        .action(ADD)
}
fn before() -> Builder {
    Runtime::builder()
        .resource(
            NoteV1::definition()
                .policy(|a, _, v| a.subject == v.owner || a.subject == "worker")
                .allow_all_fields(),
        )
        .resource(total())
        .reaction(Reaction::new(
            "note-size",
            1,
            worker(),
            ADD,
            |s: &Snapshot<NoteV1>| {
                Ok(vec![Target::new(
                    "total",
                    s.value.as_ref().unwrap().title.len() as u64,
                )])
            },
        ))
}
fn after() -> Builder {
    Runtime::builder()
        .resource(
            NoteV2::definition()
                .replay_from::<NoteV1>()
                .policy(|a, _, v| a.subject == v.owner || a.subject == "worker")
                .allow_all_fields(),
        )
        .resource(total())
        .reaction(Reaction::new(
            "note-size",
            1,
            worker(),
            ADD,
            |s: &Snapshot<NoteV2>| {
                Ok(vec![Target::new(
                    "total",
                    s.value.as_ref().unwrap().label.len() as u64,
                )])
            },
        ))
}
fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<NoteV1, NoteV2>(|v| {
            Ok(NoteV2 {
                label: v.title,
                owner: v.owner,
                active: true,
            })
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|w| {
        if w.definition != "note-size" || w.version != 1 {
            return Err(Error::Unregistered);
        }
        match &w.payload {
            WorkPayload::Source(row) => {
                NoteV2::decode(row.value.clone().ok_or(Error::Missing)?).map(|_| ())
            }
            WorkPayload::Action(value) => {
                let input: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                match input.operation {
                    Operation::Action { name, input: value }
                        if input.kind == Total::KIND && name == "add" =>
                    {
                        <u64 as Input>::decode(value).map(|_| ())
                    }
                    _ => Err(Error::Unsupported("work contract".into())),
                }
            }
            _ => Err(Error::Unsupported("work contract".into())),
        }
    })
}
struct Paths(PathBuf);
impl Paths {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "rom-migration-runtime-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn at(&self, n: &str) -> PathBuf {
        self.0.join(n)
    }
}
impl Drop for Paths {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn open(redb: bool, p: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(p).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(p).unwrap())
    }
}
fn migrate(redb: bool, source: &Path, target: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(
            rom_redb::Redb::migrate_from(source, target, &plan(), BackupLimits::default()).unwrap(),
        )
    } else {
        Arc::new(
            rom_sqlite::Sqlite::migrate_from(source, target, &plan(), BackupLimits::default())
                .unwrap(),
        )
    }
}
#[tokio::test]
async fn migrated_runtime_replays_old_input_and_resumes_both_reaction_phases() {
    for redb in [false, true] {
        for materialize in [false, true] {
            let paths = Paths::new();
            let source = paths.at("source");
            let destination = paths.at("destination");
            let storage = open(redb, &source);
            let runtime = before()
                .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            runtime
                .execute(
                    &owner(),
                    Command::create("total", Total { count: 0 }).idempotency("total"),
                )
                .await
                .unwrap();
            let original: Invocation = Command::create(
                "note",
                NoteV1 {
                    title: "hello".into(),
                    owner: "owner".into(),
                },
            )
            .idempotency("note")
            .into();
            runtime.invoke(&owner(), original.clone()).await.unwrap();
            let cursor = runtime.journal_head(&owner(), NoteV1::KIND).await.unwrap();
            if materialize {
                assert_eq!(runtime.process_reactions(1).await.unwrap(), 1);
            }
            runtime.shutdown().await.unwrap();
            drop(runtime);
            drop(storage);
            let source_bytes = std::fs::read(&source).unwrap();
            let storage = migrate(redb, &source, &destination);
            assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
            let runtime = after()
                .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            let replay = runtime.invoke(&owner(), original.clone()).await.unwrap();
            assert_eq!(replay.revision, 1);
            assert_eq!(
                replay.value,
                Some(json!({"label":"hello","owner":"owner","active":true}))
            );
            assert_eq!(
                runtime
                    .journal(&owner(), NoteV2::KIND, Some(&cursor))
                    .await
                    .unwrap_err(),
                Error::HistoryGap
            );
            assert_eq!(
                runtime
                    .journal(&owner(), NoteV2::KIND, None)
                    .await
                    .unwrap()
                    .events
                    .len(),
                1
            );
            let mut wrong = original.clone();
            wrong.operation = Operation::Create(json!({"title":"other","owner":"owner"}));
            assert_eq!(
                runtime.invoke(&owner(), wrong).await.unwrap_err(),
                Error::IdentityMismatch
            );
            let mut fresh = original.clone();
            fresh.id = "fresh".into();
            fresh.idempotency = "fresh".into();
            assert!(matches!(
                runtime.invoke(&owner(), fresh).await,
                Err(Error::Invalid { .. })
            ));
            runtime.process_reactions(8).await.unwrap();
            assert_eq!(
                runtime
                    .read::<Total>(&owner(), "total")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .count,
                5
            );
            assert!(
                storage
                    .reaction_records()
                    .unwrap()
                    .iter()
                    .all(|w| w.state == WorkState::Done)
            );
            runtime.invoke(&owner(), original.clone()).await.unwrap();
            assert_eq!(runtime.process_reactions(8).await.unwrap(), 0);
            assert_eq!(
                runtime
                    .read::<Total>(&owner(), "total")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .count,
                5
            );
            runtime
                .execute(
                    &worker(),
                    Command::patch(
                        "note",
                        Patch::new().set(NoteV2::owner_field(), "someone-else".into()),
                    )
                    .at_revision(1)
                    .idempotency("transfer"),
                )
                .await
                .unwrap();
            assert_eq!(
                runtime.invoke(&owner(), original).await.unwrap_err(),
                Error::Denied
            );
            runtime.shutdown().await.unwrap();
            drop(runtime);
            drop(storage);
        }
    }
}
#[tokio::test]
async fn migrated_tombstone_keeps_current_policy_context_and_delete_replay() {
    for redb in [false, true] {
        let paths = Paths::new();
        let source = paths.at("source");
        let destination = paths.at("destination");
        let storage = open(redb, &source);
        let runtime = Runtime::builder()
            .resource(
                NoteV1::definition()
                    .policy(|a, _, v| a.subject == v.owner)
                    .allow_all_fields(),
            )
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        runtime
            .execute(
                &owner(),
                Command::create(
                    "gone",
                    NoteV1 {
                        title: "removed".into(),
                        owner: "owner".into(),
                    },
                )
                .idempotency("create"),
            )
            .await
            .unwrap();
        let deletion: Invocation = Command::<NoteV1>::delete("gone")
            .at_revision(1)
            .idempotency("delete")
            .into();
        runtime.invoke(&owner(), deletion.clone()).await.unwrap();
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(storage);
        let storage = migrate(redb, &source, &destination);
        let runtime = Runtime::builder()
            .resource(
                NoteV2::definition()
                    .replay_from::<NoteV1>()
                    .policy(|a, _, v| a.subject == v.owner)
                    .allow_all_fields(),
            )
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let outcome = runtime.invoke(&owner(), deletion).await.unwrap();
        assert!(outcome.value.is_none());
        assert!(outcome.protected.is_empty());
        let history = runtime.journal(&owner(), NoteV2::KIND, None).await.unwrap();
        assert_eq!(history.events.len(), 1);
        assert!(history.events[0].view.value.is_none());
        let retained = storage.journal(NoteV2::KIND, None, 10, 100_000).unwrap();
        assert_eq!(
            retained.events[0].row.value.as_ref().unwrap()["label"],
            "removed"
        );
        assert!(
            runtime
                .journal(
                    &Actor::trusted("migrations", "stranger"),
                    NoteV2::KIND,
                    None
                )
                .await
                .unwrap()
                .events
                .is_empty()
        );
        runtime.shutdown().await.unwrap();
    }
}
