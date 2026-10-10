//! Explicit metadata grants belong to this disposable test host only.
use crate::{control::Fixture, note, storage};
use rom::{Actor, PrincipalKind};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn fixture_discovery_requires_current_human_authority_and_keeps_row_grants_separate() {
    for adapter in ["sqlite", "redb"] {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rom-recovery-discovery-{}-{adapter}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        let storage = Arc::new(storage::open(adapter, &path.join("database")).unwrap());
        let revoked = Arc::new(AtomicBool::new(false));
        let fixture = Fixture {
            storage: storage.clone(),
            generation: Arc::new(AtomicU64::new(0)),
            revoked: revoked.clone(),
        };
        let runtime = note::declarations()
            .actor_gate(fixture.gate())
            .build(
                storage.store.clone(),
                rom::Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        let actor = Actor::trusted("recovery-fixture", "alice")
            .with_kind(PrincipalKind::Human)
            .with_host_stamp("fixture-session-0");
        let discovery = runtime.discover(&actor).await.unwrap();
        assert_eq!(discovery.resources.len(), 1);
        assert_eq!(discovery.resources[0].kind, "recovery-notes");
        assert_eq!(discovery.resources[0].fields.len(), 5);
        assert!(
            discovery.resources[0]
                .actions
                .iter()
                .any(|name| name == "save")
        );
        let other = Actor::trusted("other", "alice")
            .with_kind(PrincipalKind::Human)
            .with_host_stamp("fixture-session-0");
        assert!(runtime.discover(&other).await.is_err());
        assert!(
            runtime
                .discover(&Actor::trusted("recovery-fixture", "alice"))
                .await
                .is_err()
        );
        revoked.store(true, Ordering::SeqCst);
        assert!(runtime.discover(&actor).await.is_err());
        runtime.shutdown().await.unwrap();
    }
}
