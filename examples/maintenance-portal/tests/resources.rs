use rom::{Actor, Command, Runtime, Storage};
use rom_maintenance_portal::{
    COMPLETE, Equipment, Inspection, PortalSettings, WorkOrder, declarations,
};
use std::sync::Arc;

async fn journey(store: Arc<dyn Storage>) {
    let runtime = declarations()
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let alice = Actor::trusted("maintenance-portal", "alice").with_kind(rom::PrincipalKind::Human);
    let bob = Actor::trusted("maintenance-portal", "bob").with_kind(rom::PrincipalKind::Human);
    runtime
        .execute(
            &alice,
            Command::create(
                "pump-1",
                Equipment {
                    owner: "alice".into(),
                    title: "Feed pump".into(),
                    active: true,
                },
            )
            .idempotency("create-equipment"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &alice,
            Command::create(
                "check-1",
                Inspection {
                    owner: "alice".into(),
                    equipment: rom::ResourceRef::new("pump-1").unwrap(),
                    notes: "Valve inspected".into(),
                    inspected_at: rom_fields::DateTime::new("2026-10-08T12:00:00Z").unwrap(),
                },
            )
            .idempotency("create-inspection"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &alice,
            Command::create(
                "repair-1",
                WorkOrder {
                    owner: "alice".into(),
                    equipment: rom::ResourceRef::new("pump-1").unwrap(),
                    title: "Replace seal".into(),
                    completed: false,
                },
            )
            .idempotency("create-work"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &alice,
            Command::create(
                "workspace",
                PortalSettings {
                    owner: "alice".into(),
                    show_history: true,
                    columns: 4,
                    layout: rom_maintenance_portal::WorkspaceLayout::initial().unwrap(),
                },
            )
            .idempotency("create-settings"),
        )
        .await
        .unwrap();
    let open = WorkOrder::completed_field().equals(false);
    assert_eq!(runtime.query(&alice, &open).await.unwrap().len(), 1);
    assert!(runtime.query(&bob, &open).await.unwrap().is_empty());
    let denied = runtime
        .execute(
            &bob,
            Command::action("repair-1", COMPLETE, ())
                .at_revision(1)
                .idempotency("denied-completion"),
        )
        .await;
    assert!(matches!(denied, Err(rom::Error::Denied)));
    assert_eq!(runtime.query(&alice, &open).await.unwrap().len(), 1);
    let command = Command::action("repair-1", COMPLETE, ())
        .at_revision(1)
        .idempotency("complete-work");
    runtime.execute(&alice, command.clone()).await.unwrap();
    runtime.execute(&alice, command).await.unwrap();
    assert!(runtime.query(&alice, &open).await.unwrap().is_empty());
    assert_eq!(
        runtime
            .read::<WorkOrder>(&alice, "repair-1")
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        store
            .journal("work-orders", None, 100, 1048576)
            .unwrap()
            .events
            .len(),
        2
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn public_declarations_share_the_sqlite_pipeline() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_sqlite::Sqlite::open(directory.path().join("portal.sqlite")).unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn public_declarations_share_the_redb_pipeline() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_redb::Redb::open(directory.path().join("portal.redb")).unwrap(),
    ))
    .await;
}
