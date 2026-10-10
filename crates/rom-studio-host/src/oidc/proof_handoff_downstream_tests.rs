//! Deterministic downstream boundaries, including publication with an unknown acknowledgement.
use super::credentials_tests::{Fixture, bind_barrier::Controlled};
use crate::{AuthOperation, StudioHost, session::SessionEvidence};
use rom::{Command, Resource};
use rom_blob::{BlobStore, Metadata, ObjectKey, StoreFuture};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
const BOUND: Duration = Duration::from_secs(2);

#[derive(Clone, Resource)]
#[resource(name = "handoff_documents")]
pub(super) struct Document {
    pub(super) done: bool,
}

async fn command(storage: Arc<dyn rom::Storage>) {
    let controlled = Arc::new(Controlled::new(storage));
    let fixture = Fixture::with_diagnostics(200, false, controlled.clone(), None, true).await;
    fixture.clock.0.store(125, Ordering::SeqCst);
    let actor = fixture
        .credentials
        .actor_observed(&fixture.host.shared, AuthOperation::GenericHttpResolver)
        .await
        .unwrap();
    let until = actor.valid_until();
    controlled.arm();
    let runtime = fixture.host.shared.runtime.clone();
    let invocation = Command::create("one", Document { done: true }).idempotency("handoff-create");
    let pending = tokio::spawn(async move { runtime.execute(&actor, invocation).await });
    let entered = tokio::time::timeout(BOUND, controlled.started.notified()).await;
    if entered.is_ok() {
        fixture.clock.0.store(130, Ordering::SeqCst);
    }
    controlled.release();
    let result = tokio::time::timeout(BOUND, pending).await.unwrap().unwrap();
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    entered.expect("downstream current-identity read must reach the barrier before commit");
    assert_eq!(until, Some(155));
    let row = result.expect("early verified proof remains live across the old boundary");
    assert_eq!(row.revision, 1);
    assert!(row.value.unwrap().done);
    assert_eq!(requests, 1);
}

#[derive(Default)]
struct Publication {
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    block: AtomicBool,
    published: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl BlobStore for Publication {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            {
                let mut objects = self.objects.lock().unwrap();
                if objects.contains_key(key.as_str()) {
                    return Err(rom_blob::Error::Conflict);
                }
                objects.insert(key.as_str().into(), bytes);
            }
            if self.block.swap(false, Ordering::SeqCst) {
                self.published.notify_one();
                tokio::time::timeout(BOUND, self.release.notified())
                    .await
                    .map_err(|_| rom_blob::Error::Timeout)?;
            }
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let objects = self.objects.lock().unwrap();
            let bytes = objects.get(key.as_str()).ok_or(rom_blob::Error::Missing)?;
            if bytes.len() > max {
                return Err(rom_blob::Error::TooLarge);
            }
            Ok(bytes.clone())
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async move {
            Ok(Metadata {
                bytes: self.get(key, usize::MAX).await?.len() as u64,
            })
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.objects.lock().unwrap().remove(key.as_str());
            Ok(())
        })
    }
}

fn upload_request(session: &crate::session::Session) -> axum::extract::Request {
    axum::http::Request::builder()
        .method("POST")
        .uri("/rom-studio/blobs/upload?id=one")
        .header("origin", "http://127.0.0.1:43210")
        .header("cookie", format!("rom_session={}", session.cookie()))
        .header("x-rom-csrf", session.csrf())
        .body(axum::body::Body::from("hello"))
        .unwrap()
}
async fn publication(storage: Arc<dyn rom::Storage>, expire_fresh: bool) {
    let fixture = Fixture::with_diagnostics(200, false, storage, None, true).await;
    let store = Arc::new(Publication::default());
    let service = rom_blob::BlobService::builder(fixture.host.shared.runtime.clone())
        .store("objects", store.clone())
        .build()
        .unwrap();
    let host = StudioHost::new(
        fixture.host.shared.runtime.clone(),
        fixture.host.shared.config.clone().blobs(service.clone()),
    )
    .unwrap();
    fixture.clock.0.store(120, Ordering::SeqCst);
    let actor = fixture
        .credentials
        .actor_observed(&host.shared, AuthOperation::Blob)
        .await
        .unwrap();
    let until = actor.valid_until();
    let session = host
        .shared
        .sessions
        .insert(
            SessionEvidence {
                actor: actor.clone(),
                user_id: "alice-user".into(),
                token_expiry: 400,
                credentials: Some(fixture.credentials.clone()),
            },
            120,
        )
        .unwrap();
    service
        .reserve(
            &actor,
            "one",
            "objects",
            rom_blob::Digest::of(b"hello"),
            5,
            "handoff-reserve",
        )
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let shared = host.shared.clone();
    let request = upload_request(&session);
    let pending =
        tokio::spawn(
            async move { crate::blobs::upload(axum::extract::State(shared), request).await },
        );
    let entered = tokio::time::timeout(BOUND, store.published.notified()).await;
    if entered.is_ok() {
        fixture
            .clock
            .0
            .store(if expire_fresh { 150 } else { 130 }, Ordering::SeqCst);
    }
    store.release.notify_one();
    let response = tokio::time::timeout(BOUND, pending).await.unwrap().unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let first_objects = store.objects.lock().unwrap().clone();
    // Maintenance inspection uses the explicit worker's authority, not an expired caller.
    let first = host
        .shared
        .runtime
        .read::<rom_blob::Blob>(&rom_blob::worker_actor(), "one")
        .await
        .unwrap();
    let first_state = first.value.unwrap().state;
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let recovered = if expire_fresh {
        let response = crate::blobs::upload(
            axum::extract::State(host.shared.clone()),
            upload_request(&session),
        )
        .await;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        Some((
            status,
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        ))
    } else {
        None
    };
    let final_row = host
        .shared
        .runtime
        .read::<rom_blob::Blob>(&rom_blob::worker_actor(), "one")
        .await
        .unwrap();
    let final_objects = store.objects.lock().unwrap().clone();
    let requests = fixture.requests.load(Ordering::SeqCst);
    let expiry = fixture.credentials.expiry;
    host.shutdown().await.unwrap();
    fixture.close().await;
    entered.expect("immutable bytes must exist before the clock advances");
    assert_eq!(until, Some(150));
    assert_eq!(first_objects.len(), 1);
    assert_eq!(first_objects.values().next().unwrap().as_slice(), b"hello");
    assert_eq!(first_objects, final_objects);
    assert_eq!(expiry, 400);
    assert_eq!(final_row.revision, 2);
    assert_eq!(final_row.value.unwrap().state, rom_blob::BlobState::Ready);
    if expire_fresh {
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value, serde_json::json!({"error":"outcome_unknown"}));
        assert_eq!(first_state, rom_blob::BlobState::Pending);
        assert_eq!(
            snapshot
                .stage(crate::AuthStage::BlobUploadUnattached)
                .denied,
            1
        );
        assert_eq!(recovered.unwrap().0, axum::http::StatusCode::OK);
        assert_eq!(requests, 2);
    } else {
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(value["status"], "attached");
        assert_eq!(first_state, rom_blob::BlobState::Ready);
        assert_eq!(requests, 1);
    }
}

fn redb() -> (std::path::PathBuf, Arc<dyn rom::Storage>) {
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).unwrap();
    let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let root = std::env::temp_dir().join(format!("rom-handoff-{suffix}"));
    std::fs::create_dir(&root).unwrap();
    let storage = Arc::new(rom_redb::Redb::open(root.join("state.redb")).unwrap());
    (root, storage)
}
#[tokio::test]
async fn sqlite_early_handoff_survives_old_expiry_before_commit() {
    command(Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap())).await;
}
#[tokio::test]
async fn redb_early_handoff_survives_old_expiry_before_commit() {
    let (root, storage) = redb();
    command(storage).await;
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn sqlite_early_blob_handoff_survives_old_expiry_after_publication() {
    publication(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        false,
    )
    .await;
}
#[tokio::test]
async fn redb_early_blob_handoff_survives_old_expiry_after_publication() {
    let (root, storage) = redb();
    publication(storage, false).await;
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn sqlite_fresh_expiry_after_publication_stays_unknown_then_reconciles() {
    publication(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        true,
    )
    .await;
}
#[tokio::test]
async fn redb_fresh_expiry_after_publication_stays_unknown_then_reconciles() {
    let (root, storage) = redb();
    publication(storage, true).await;
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn configured_blob_staging_budget_at_proof_ceiling_refuses_before_host_start() {
    let fixture = Fixture::observed(200, false).await;
    let service = rom_blob::BlobService::builder(fixture.host.shared.runtime.clone())
        .store("objects", Arc::new(Publication::default()))
        .limits(rom_blob::Limits {
            staging_timeout: Duration::from_secs(rom_auth::oidc::MAX_PROOF_SECONDS),
            ..Default::default()
        })
        .build()
        .unwrap();
    let result = fixture
        .host
        .shared
        .config
        .clone()
        .blobs(service.clone())
        .validate();
    service.shutdown().await.unwrap();
    fixture.close().await;
    assert!(matches!(result, Err(rom::Error::Invalid { .. })));
}
