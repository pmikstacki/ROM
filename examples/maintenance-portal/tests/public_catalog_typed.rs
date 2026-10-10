use rom::{Actor, Command, Error, PrincipalKind, Runtime, Storage};
use rom_maintenance_portal::{MaintenanceGuide, declarations, public_guest};
use std::sync::Arc;

async fn journey(store: Arc<dyn Storage>) {
    let runtime = declarations()
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let publisher = Actor::trusted("maintenance-portal", "alice").with_kind(PrincipalKind::Human);
    runtime
        .execute(
            &publisher,
            Command::create(
                "electrical",
                MaintenanceGuide {
                    title: "Electrical inspection".into(),
                    nominal_voltage: 230,
                    updated_at: rom_fields::DateTime::new("2026-10-08T12:00:00Z").unwrap(),
                    internal_notes: "PRIVATE PUBLISHER NOTE".into(),
                },
            )
            .idempotency("publish-guide"),
        )
        .await
        .unwrap();
    let guest = public_guest();
    assert!(matches!(
        runtime.read::<MaintenanceGuide>(&guest, "electrical").await,
        Err(Error::Denied)
    ));
    let before = runtime
        .read_projected(&guest, "maintenance-guides", "electrical")
        .await
        .unwrap();
    assert_eq!(before.value.as_ref().unwrap().len(), 3);
    let mut subscription = runtime
        .live_spec_projected(
            &guest,
            "maintenance-guides",
            rom::QuerySpec::equal("title", rom::json!("Electrical inspection")),
        )
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), subscription.changed())
            .await
            .unwrap()
            .unwrap(),
        vec![before]
    );
    let original = runtime
        .read::<MaintenanceGuide>(&publisher, "electrical")
        .await
        .unwrap();
    let mut changed = original.value.unwrap();
    changed.nominal_voltage = 240;
    runtime
        .execute(
            &publisher,
            Command::replace("electrical", changed)
                .at_revision(1)
                .idempotency("publish-new-reference"),
        )
        .await
        .unwrap();
    let next = tokio::time::timeout(std::time::Duration::from_secs(2), subscription.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].revision, 2);
    assert_eq!(
        next[0].value.as_ref().unwrap()["nominal_voltage"],
        rom::json!(240)
    );
    assert!(
        next[0]
            .value
            .as_ref()
            .unwrap()
            .get("internal_notes")
            .is_none()
    );
    drop(subscription);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn sqlite_public_projection_live_updates_never_become_complete_private_values() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_sqlite::Sqlite::open(directory.path().join("live.sqlite")).unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn redb_public_projection_live_updates_never_become_complete_private_values() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_redb::Redb::open(directory.path().join("live.redb")).unwrap(),
    ))
    .await;
}
