//! Public Resource boundary tests precede the two complete durable tool journeys.
use rom::{Actor, Command, Runtime, Storage};
use rom_ai_flows_consumer::publication::{
    Draft, Edition, Head, PREPARE_EDITION, PUBLISH_HEAD, PrepareEdition,
};
use rom_ai_flows_consumer::triage::{CLASSIFY, Classification, Classify, Ticket};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
fn owner() -> Actor {
    Actor::trusted("external-ai-consumer", "author")
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_publication_retains_immutable_revision_before_head_commit() {
    publication(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_publication_retains_immutable_revision_before_head_commit() {
    publication(true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_unrelated_triage_rejects_invalid_classification_and_replays_exact_action() {
    triage(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_unrelated_triage_rejects_invalid_classification_and_replays_exact_action() {
    triage(true).await;
}
fn storage(redb: bool) -> (std::path::PathBuf, Arc<dyn Storage>) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-ai-public-consumer-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    (dir, storage)
}
async fn publication(redb: bool) {
    let (dir, store) = storage(redb);
    let runtime = rom_ai_flows_consumer::publication::register(Runtime::builder())
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create(
                "draft",
                Draft {
                    owner: "author".into(),
                    text: "First historical article".into(),
                },
            )
            .idempotency("create-draft"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create(
                "edition",
                Edition {
                    owner: "author".into(),
                    draft: rom::ResourceRef::<Draft>::new("draft").unwrap(),
                    draft_revision: 1,
                    text: "First historical article".into(),
                    prepared: false,
                },
            )
            .idempotency("create-edition"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create(
                "head",
                Head {
                    owner: "author".into(),
                    edition: None,
                },
            )
            .idempotency("create-head"),
        )
        .await
        .unwrap();
    let prepare = Command::action(
        "edition",
        PREPARE_EDITION,
        PrepareEdition {
            draft_revision: 1,
            text: "First historical article".into(),
        },
    )
    .at_revision(1)
    .idempotency("prepare-publication");
    runtime.execute(&owner(), prepare.clone()).await.unwrap();
    assert!(
        runtime
            .read::<Head>(&owner(), "head")
            .await
            .unwrap()
            .value
            .unwrap()
            .edition
            .is_none(),
        "immutable edition preparation must not pretend to publish the head"
    );
    runtime.execute(&owner(), prepare).await.unwrap();
    assert_eq!(
        runtime
            .read::<Edition>(&owner(), "edition")
            .await
            .unwrap()
            .revision,
        2
    );
    assert!(
        runtime
            .execute(
                &owner(),
                Command::action(
                    "edition",
                    PREPARE_EDITION,
                    PrepareEdition {
                        draft_revision: 2,
                        text: "Rewritten history".into()
                    }
                )
                .at_revision(2)
                .idempotency("rewrite-edition")
            )
            .await
            .is_err()
    );
    runtime
        .execute(
            &owner(),
            Command::action(
                "head",
                PUBLISH_HEAD,
                rom::ResourceRef::<Edition>::new("edition").unwrap(),
            )
            .at_revision(1)
            .idempotency("publish-head"),
        )
        .await
        .unwrap();
    let citation = runtime
        .read::<Head>(&owner(), "head")
        .await
        .unwrap()
        .value
        .unwrap()
        .edition
        .unwrap();
    assert_eq!(citation.id(), "edition");
    assert_eq!(
        runtime
            .read::<Edition>(&owner(), citation.id())
            .await
            .unwrap()
            .value
            .unwrap()
            .text,
        "First historical article"
    );
    drop(runtime);
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}
async fn triage(redb: bool) {
    let (dir, store) = storage(redb);
    let runtime = rom_ai_flows_consumer::triage::register(Runtime::builder())
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create(
                "ticket",
                Ticket {
                    owner: "author".into(),
                    body: "Pump requires inspection".into(),
                    classification: Classification::new("unclassified").unwrap(),
                },
            )
            .idempotency("create-ticket"),
        )
        .await
        .unwrap();
    assert!(
        runtime
            .execute(
                &owner(),
                Command::action(
                    "ticket",
                    CLASSIFY,
                    Classify {
                        classification: "invented".into()
                    }
                )
                .at_revision(1)
                .idempotency("invalid-classification")
            )
            .await
            .is_err()
    );
    assert_eq!(
        runtime
            .read::<Ticket>(&owner(), "ticket")
            .await
            .unwrap()
            .revision,
        1
    );
    let classify = Command::action(
        "ticket",
        CLASSIFY,
        Classify {
            classification: "urgent".into(),
        },
    )
    .at_revision(1)
    .idempotency("classify-ticket");
    runtime.execute(&owner(), classify.clone()).await.unwrap();
    runtime.execute(&owner(), classify).await.unwrap();
    assert_eq!(
        runtime
            .read::<Ticket>(&owner(), "ticket")
            .await
            .unwrap()
            .revision,
        2
    );
    assert!(
        runtime
            .read::<Ticket>(&Actor::trusted("external-ai-consumer", "other"), "ticket")
            .await
            .is_err()
    );
    drop(runtime);
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn external_custom_classification_field_uses_public_codec_and_rejects_unknown_values() {
    use rom::Field;
    let urgent = Classification::new("urgent").unwrap();
    assert_eq!(urgent.encode(), rom::json!("urgent"));
    assert_eq!(
        Classification::decode(urgent.encode()).unwrap().as_str(),
        "urgent"
    );
    assert!(Classification::decode(rom::json!("invented")).is_err());
    assert!(Classification::decode(rom::json!(null)).is_err());
    assert_eq!(
        Classification::shape(),
        rom::Shape::Enum(vec![
            "unclassified".into(),
            "routine".into(),
            "urgent".into()
        ])
    );
}
