use rom::*;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Resource)]
#[resource(name = "reference-targets")]
struct Target {
    label: String,
}
#[derive(Clone, Resource)]
#[resource(name = "direct-references")]
struct Direct {
    target: ResourceRef<Target>,
}
#[derive(Clone, Resource)]
#[resource(name = "nullable-references")]
struct Nullable {
    target: Option<ResourceRef<Target>>,
}
#[derive(Clone, Resource)]
#[resource(name = "list-references")]
struct List {
    targets: Vec<ResourceRef<Target>>,
}
#[derive(Clone, Resource)]
#[resource(name = "map-references")]
struct Map {
    targets: BTreeMap<String, Option<ResourceRef<Target>>>,
}
#[derive(Clone, Resource)]
#[resource(name = "self-references")]
struct Node {
    next: Option<ResourceRef<Node>>,
}

fn actor() -> Actor {
    Actor::trusted("reference-tests", "owner")
}
fn definition<R: Resource>() -> Definition<R> {
    R::definition().policy(|_, _, _| true).allow_all_fields()
}
struct Fixture {
    runtime: Runtime,
    storage: Arc<dyn Storage>,
    path: PathBuf,
}
impl Fixture {
    fn new(redb: bool) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-references-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = Runtime::builder()
            .resource(definition::<Target>())
            .resource(definition::<Direct>())
            .resource(definition::<Nullable>())
            .resource(definition::<List>())
            .resource(definition::<Map>())
            .resource(definition::<Node>())
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self {
            runtime,
            storage,
            path,
        }
    }
    async fn create_target(&self) {
        self.runtime
            .execute(
                &actor(),
                Command::create(
                    "target",
                    Target {
                        label: "live".into(),
                    },
                )
                .idempotency("create-target"),
            )
            .await
            .unwrap();
    }
    async fn assert_target_unchanged(&self) {
        assert_eq!(
            self.runtime
                .read::<Target>(&actor(), "target")
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            self.runtime
                .journal(&actor(), Target::KIND, None)
                .await
                .unwrap()
                .events
                .len(),
            1
        );
    }
    async fn reject_missing<R: Resource>(&self, value: R) {
        let command = || Command::create("source", value.clone()).idempotency("create-source");
        let result = self.runtime.execute(&actor(), command()).await;
        assert!(
            matches!(result, Err(Error::Conflict)),
            "missing reference target must conflict"
        );
        assert!(matches!(
            self.runtime.read::<R>(&actor(), "source").await,
            Err(Error::Missing)
        ));
        assert!(
            self.runtime
                .journal(&actor(), R::KIND, None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        // If rejection wrote a receipt, retry after target creation cannot create the source.
        self.create_target().await;
        let accepted = self.runtime.execute(&actor(), command()).await.unwrap();
        assert_eq!(accepted.revision, 1);
        assert_eq!(
            self.runtime
                .journal(&actor(), R::KIND, None)
                .await
                .unwrap()
                .events
                .len(),
            1
        );
        self.runtime.shutdown().await.unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

async fn source_blocks_target_deletion(redb: bool) {
    let f = Fixture::new(redb);
    f.create_target().await;
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "source",
                Direct {
                    target: ResourceRef::new("target").unwrap(),
                },
            )
            .idempotency("create-source"),
        )
        .await
        .unwrap();
    let result = f
        .runtime
        .execute(
            &actor(),
            Command::<Target>::delete("target")
                .at_revision(1)
                .idempotency("delete-target"),
        )
        .await;
    assert!(
        matches!(result, Err(Error::Conflict)),
        "incoming reference must restrict deletion"
    );
    f.assert_target_unchanged().await;
    assert_eq!(
        f.runtime
            .read::<Direct>(&actor(), "source")
            .await
            .unwrap()
            .revision,
        1
    );
    f.runtime.shutdown().await.unwrap();
}

async fn unlink_all_shapes_allows_deletion(redb: bool) {
    let f = Fixture::new(redb);
    f.create_target().await;
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "nullable",
                Nullable {
                    target: Some(ResourceRef::new("target").unwrap()),
                },
            )
            .idempotency("nullable"),
        )
        .await
        .unwrap();
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "list",
                List {
                    targets: vec![
                        ResourceRef::new("target").unwrap(),
                        ResourceRef::new("target").unwrap(),
                    ],
                },
            )
            .idempotency("list"),
        )
        .await
        .unwrap();
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "map",
                Map {
                    targets: BTreeMap::from([
                        ("none".into(), None),
                        ("target".into(), Some(ResourceRef::new("target").unwrap())),
                    ]),
                },
            )
            .idempotency("map"),
        )
        .await
        .unwrap();
    let delete = || {
        Command::<Target>::delete("target")
            .at_revision(1)
            .idempotency("delete-target")
    };
    assert!(matches!(
        f.runtime.execute(&actor(), delete()).await,
        Err(Error::Conflict)
    ));
    f.runtime
        .execute(
            &actor(),
            Command::patch("nullable", Patch::new().set(Nullable::target_field(), None))
                .at_revision(1)
                .idempotency("unlink-nullable"),
        )
        .await
        .unwrap();
    assert!(
        matches!(
            f.runtime.execute(&actor(), delete()).await,
            Err(Error::Conflict)
        ),
        "list/map links still block deletion"
    );
    f.runtime
        .execute(
            &actor(),
            Command::patch("list", Patch::new().set(List::targets_field(), vec![]))
                .at_revision(1)
                .idempotency("unlink-list"),
        )
        .await
        .unwrap();
    assert!(
        matches!(
            f.runtime.execute(&actor(), delete()).await,
            Err(Error::Conflict)
        ),
        "map link still blocks deletion"
    );
    f.runtime
        .execute(
            &actor(),
            Command::patch(
                "map",
                Patch::new().set(
                    Map::targets_field(),
                    BTreeMap::from([("none".into(), None)]),
                ),
            )
            .at_revision(1)
            .idempotency("unlink-map"),
        )
        .await
        .unwrap();
    f.assert_target_unchanged().await;
    let removed = f.runtime.execute(&actor(), delete()).await.unwrap();
    assert_eq!(removed.revision, 2);
    assert!(removed.value.is_none());
    assert_eq!(
        f.storage
            .journal(Target::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    f.runtime.shutdown().await.unwrap();
}

async fn self_reference_can_be_created_and_deleted(redb: bool) {
    let f = Fixture::new(redb);
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "self",
                Node {
                    next: Some(ResourceRef::new("self").unwrap()),
                },
            )
            .idempotency("create-self"),
        )
        .await
        .unwrap();
    let removed = f
        .runtime
        .execute(
            &actor(),
            Command::<Node>::delete("self")
                .at_revision(1)
                .idempotency("delete-self"),
        )
        .await
        .unwrap();
    assert_eq!(removed.revision, 2);
    assert!(removed.value.is_none());
    assert_eq!(
        f.storage
            .journal(Node::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    f.runtime.shutdown().await.unwrap();
}

async fn source_delete_releases_target(redb: bool) {
    let f = Fixture::new(redb);
    f.create_target().await;
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "source",
                Direct {
                    target: ResourceRef::new("target").unwrap(),
                },
            )
            .idempotency("original"),
        )
        .await
        .unwrap();
    f.runtime
        .execute(
            &actor(),
            Command::<Direct>::delete("source")
                .at_revision(1)
                .idempotency("delete-source"),
        )
        .await
        .unwrap();
    let removed = f
        .runtime
        .execute(
            &actor(),
            Command::<Target>::delete("target")
                .at_revision(1)
                .idempotency("delete-target"),
        )
        .await
        .unwrap();
    assert_eq!(removed.revision, 2);
    assert!(removed.value.is_none());
    assert_eq!(
        f.storage
            .journal(Direct::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    assert_eq!(
        f.storage
            .journal(Target::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    f.runtime.shutdown().await.unwrap();
}

async fn historical_replay_after_unlink_is_not_a_new_edge(redb: bool) {
    let f = Fixture::new(redb);
    f.create_target().await;
    let original = || {
        Command::create(
            "source",
            Nullable {
                target: Some(ResourceRef::new("target").unwrap()),
            },
        )
        .idempotency("original")
    };
    f.runtime.execute(&actor(), original()).await.unwrap();
    f.runtime
        .execute(
            &actor(),
            Command::patch("source", Patch::new().set(Nullable::target_field(), None))
                .at_revision(1)
                .idempotency("unlink"),
        )
        .await
        .unwrap();
    f.runtime
        .execute(
            &actor(),
            Command::<Target>::delete("target")
                .at_revision(1)
                .idempotency("delete-target"),
        )
        .await
        .unwrap();
    let replay = f.runtime.execute(&actor(), original()).await.unwrap();
    assert_eq!(replay.revision, 1);
    assert_eq!(replay.value.unwrap().target.unwrap().id(), "target");
    assert_eq!(
        f.storage
            .journal(Nullable::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    assert_eq!(
        f.storage
            .journal(Target::KIND, None, 100, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    assert!(
        f.runtime
            .read::<Nullable>(&actor(), "source")
            .await
            .unwrap()
            .value
            .unwrap()
            .target
            .is_none()
    );
    f.runtime.shutdown().await.unwrap();
}

async fn omitted_source_kind_after_reopen_cannot_erase_restrict(redb: bool) {
    let mut f = Fixture::new(redb);
    f.create_target().await;
    f.runtime
        .execute(
            &actor(),
            Command::create(
                "source",
                Direct {
                    target: ResourceRef::new("target").unwrap(),
                },
            )
            .idempotency("source"),
        )
        .await
        .unwrap();
    f.runtime.shutdown().await.unwrap();
    let path = std::mem::take(&mut f.path);
    drop(f);
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    let runtime = Runtime::builder()
        .resource(definition::<Target>())
        .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    assert!(matches!(
        runtime
            .execute(
                &actor(),
                Command::<Target>::delete("target")
                    .at_revision(1)
                    .idempotency("delete-after-reopen")
            )
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        runtime
            .read::<Target>(&actor(), "target")
            .await
            .unwrap()
            .revision,
        1
    );
    // Retained catalog is authority; a new registration cannot change persisted shape.
    let mut incompatible = Direct::descriptor();
    incompatible.fields[0].shape = Shape::String;
    assert!(storage.register(&[incompatible]).is_err());
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    std::fs::remove_file(path).unwrap();
}

async fn competing_runtime_is_rejected(redb: bool) {
    let f = Fixture::new(redb);
    let other = Runtime::builder()
        .resource(definition::<Target>())
        .resource(definition::<Direct>())
        .build(f.storage.clone(), Runtime::shared_cpu_pool(2).unwrap());
    assert!(matches!(other, Err(Error::Conflict)));
    f.runtime.shutdown().await.unwrap();
}

fn race_bundle(row: Row, expected: Option<u64>, identity: &str) -> Bundle {
    Bundle {
        expected,
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: identity.into(),
            fingerprint: identity.into(),
            row,
        },
    }
}

async fn concurrent_adapter_create_delete_race_preserves_integrity(redb: bool) {
    let f = Fixture::new(redb);
    f.create_target().await;
    // Direct trusted adapter commits preserve the independent atomic-integrity race.
    // Runtime exclusion must not become its only protection against dangling references.
    let create = race_bundle(
        Row {
            key: Key {
                kind: Direct::KIND.into(),
                id: "source".into(),
            },
            revision: 1,
            value: Some(
                Direct {
                    target: ResourceRef::new("target").unwrap(),
                }
                .encode(),
            ),
            protected: Default::default(),
        },
        None,
        "racing-create",
    );
    let mut target = f
        .storage
        .load(&Key {
            kind: Target::KIND.into(),
            id: "target".into(),
        })
        .unwrap()
        .unwrap();
    target.revision = 2;
    target.value = None;
    let delete = race_bundle(target, Some(1), "racing-delete");
    let barrier = std::sync::Barrier::new(2);
    let (created, deleted) = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            f.storage.commit(&create)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            f.storage.commit(&delete)
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    let a = actor();
    match (created, deleted) {
        (Ok(_), Err(Error::Conflict)) => {
            assert_eq!(
                f.runtime
                    .read::<Direct>(&a, "source")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .target
                    .id(),
                "target"
            );
            f.assert_target_unchanged().await;
        }
        (Err(Error::Conflict), Ok(_)) => {
            assert!(matches!(
                f.runtime.read::<Direct>(&a, "source").await,
                Err(Error::Missing)
            ));
            assert!(
                f.storage
                    .journal(Direct::KIND, None, 100, 100_000)
                    .unwrap()
                    .events
                    .is_empty()
            );
        }
        _ => panic!("exactly one competing mutation must commit; the other must conflict"),
    }
    f.runtime.shutdown().await.unwrap();
}

macro_rules! backend_tests {
    ($name:ident, $redb:expr) => {
        mod $name {
            use super::*;
            #[tokio::test]
            async fn source_deletion_releases_target() {
                source_delete_releases_target($redb).await;
            }
            #[tokio::test]
            async fn omitted_kind_and_changed_schema_preserve_persisted_integrity() {
                omitted_source_kind_after_reopen_cannot_erase_restrict($redb).await;
            }
            #[tokio::test]
            async fn competing_runtime_owner_is_rejected() {
                competing_runtime_is_rejected($redb).await;
            }
            #[tokio::test]
            async fn competing_adapter_create_delete_cannot_commit_dangling_reference() {
                concurrent_adapter_create_delete_race_preserves_integrity($redb).await;
            }
            #[tokio::test]
            async fn old_receipt_replay_after_unlink_does_not_add_an_edge() {
                historical_replay_after_unlink_is_not_a_new_edge($redb).await;
            }
            #[tokio::test]
            async fn missing_direct_target_conflicts_without_a_receipt() {
                Fixture::new($redb)
                    .reject_missing(Direct {
                        target: ResourceRef::new("target").unwrap(),
                    })
                    .await;
            }
            #[tokio::test]
            async fn missing_nullable_target_conflicts_without_a_receipt() {
                Fixture::new($redb)
                    .reject_missing(Nullable {
                        target: Some(ResourceRef::new("target").unwrap()),
                    })
                    .await;
            }
            #[tokio::test]
            async fn missing_list_target_conflicts_without_a_receipt() {
                Fixture::new($redb)
                    .reject_missing(List {
                        targets: vec![ResourceRef::new("target").unwrap()],
                    })
                    .await;
            }
            #[tokio::test]
            async fn missing_map_target_conflicts_without_a_receipt() {
                Fixture::new($redb)
                    .reject_missing(Map {
                        targets: BTreeMap::from([
                            ("absent".into(), None),
                            ("target".into(), Some(ResourceRef::new("target").unwrap())),
                        ]),
                    })
                    .await;
            }
            #[tokio::test]
            async fn incoming_source_restricts_deletion() {
                source_blocks_target_deletion($redb).await;
            }
            #[tokio::test]
            async fn clearing_nullable_list_and_map_links_allows_deletion() {
                unlink_all_shapes_allows_deletion($redb).await;
            }
            #[tokio::test]
            async fn own_reference_does_not_prevent_deletion() {
                self_reference_can_be_created_and_deleted($redb).await;
            }
        }
    };
}
backend_tests!(sqlite, false);
backend_tests!(redb, true);

#[tokio::test]
async fn sqlite_writes_only_changed_reference_edges() {
    #[derive(Clone, Resource)]
    #[resource(name = "labeled-references")]
    struct Labeled {
        label: String,
        targets: Vec<ResourceRef<Target>>,
    }
    let path = std::env::temp_dir().join(format!(
        "rom-reference-write-count-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let storage = Arc::new(rom_sqlite::Sqlite::open(&path).unwrap());
    let runtime = Runtime::builder()
        .resource(definition::<Target>())
        .resource(definition::<Labeled>())
        .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = writes.clone();
    storage.on_commit(Some(Arc::new(move |ordinal| {
        if ordinal != usize::MAX {
            observed.fetch_max(ordinal, Ordering::SeqCst);
        }
        Ok(())
    })));
    for id in ["a", "b", "c"] {
        runtime
            .execute(
                &actor(),
                Command::create(id, Target { label: id.into() }).idempotency(id),
            )
            .await
            .unwrap();
    }
    let bundle_writes = writes.swap(0, Ordering::SeqCst);
    runtime
        .execute(
            &actor(),
            Command::create(
                "source",
                Labeled {
                    label: "initial".into(),
                    targets: vec![
                        ResourceRef::new("a").unwrap(),
                        ResourceRef::new("b").unwrap(),
                    ],
                },
            )
            .idempotency("source"),
        )
        .await
        .unwrap();
    assert_eq!(writes.swap(0, Ordering::SeqCst), bundle_writes + 2);
    for (revision, label, targets, edge_writes) in [
        (1, "payload-only", vec!["a", "b"], 0),
        (2, "one-removed-one-added", vec!["b", "c"], 2),
        (3, "all-removed", vec![], 2),
    ] {
        runtime
            .execute(
                &actor(),
                Command::replace(
                    "source",
                    Labeled {
                        label: label.into(),
                        targets: targets
                            .into_iter()
                            .map(|id| ResourceRef::new(id).unwrap())
                            .collect(),
                    },
                )
                .at_revision(revision)
                .idempotency(label),
            )
            .await
            .unwrap();
        assert_eq!(
            writes.swap(0, Ordering::SeqCst),
            bundle_writes + edge_writes,
            "{label} must write only the changed edges"
        );
    }
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    std::fs::remove_file(path).unwrap();
}
