use rom::{Actor, Command, Runtime};
use rom_blob::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
#[derive(Default)]
struct Controlled {
    memory: Memory,
    block_create: AtomicBool,
    block_read: AtomicBool,
    unknown_create: AtomicBool,
    panic_create: AtomicBool,
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl BlobStore for Controlled {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            assert!(
                !self.panic_create.swap(false, Ordering::SeqCst),
                "injected backend panic"
            );
            if self.block_create.swap(false, Ordering::SeqCst) {
                self.started.notify_one();
                self.release.notified().await;
            }
            self.memory.create(key, bytes).await?;
            if self.unknown_create.swap(false, Ordering::SeqCst) {
                Err(Error::Unknown)
            } else {
                Ok(())
            }
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            if self.block_read.swap(false, Ordering::SeqCst) {
                self.started.notify_one();
                self.release.notified().await;
            }
            self.memory.get(key, max).await
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.memory.head(key)
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.memory.delete(key)
    }
}
fn controlled(limits: Limits) -> (Runtime, BlobService, Arc<Controlled>) {
    let runtime = Runtime::builder()
        .resource(definition())
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap();
    let store = Arc::new(Controlled::default());
    let service = BlobService::new(
        runtime.clone(),
        BTreeMap::from([("attachments".into(), store.clone() as Arc<dyn BlobStore>)]),
        limits,
    )
    .unwrap();
    (runtime, service, store)
}
async fn reserve(service: &BlobService, actor: &Actor) {
    service
        .reserve(
            actor,
            "one",
            "attachments",
            Digest::of(b"hello"),
            5,
            "reserve",
        )
        .await
        .unwrap();
}
#[tokio::test]
async fn canceled_waiter_retains_capacity_and_shutdown_drains_publication() {
    let (runtime, service, store) = controlled(Limits {
        operations: 1,
        ..Default::default()
    });
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    store.block_create.store(true, Ordering::SeqCst);
    let (s, a) = (service.clone(), actor.clone());
    let caller = tokio::spawn(async move { s.upload(&a, "one", upload(b"hello")).await });
    store.started.notified().await;
    caller.abort();
    let _ = caller.await;
    assert!(matches!(
        service.read(&actor, "one").await,
        Err(Error::Overloaded)
    ));
    let s = service.clone();
    let mut drain = tokio::spawn(async move { s.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut drain)
            .await
            .is_err()
    );
    store.release.notify_one();
    drain.await.unwrap().unwrap();
    assert_eq!(
        runtime
            .read::<Blob>(&actor, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .state,
        BlobState::Ready
    );
    assert!(matches!(
        service.read(&actor, "one").await,
        Err(Error::Closed)
    ));
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn revocation_during_backend_read_prevents_disclosure() {
    let (runtime, service, store) = controlled(Limits::default());
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    service
        .upload(&actor, "one", upload(b"hello"))
        .await
        .unwrap();
    store.block_read.store(true, Ordering::SeqCst);
    assert_eq!(
        tokio::time::timeout(
            Duration::from_millis(100),
            service.read(&Actor::trusted("test", "mallory"), "one")
        )
        .await
        .unwrap(),
        Err(Error::Core(rom::Error::Denied))
    );
    assert!(store.block_read.load(Ordering::SeqCst));
    let (s, a) = (service.clone(), actor.clone());
    let reader = tokio::spawn(async move { s.read(&a, "one").await });
    store.started.notified().await;
    runtime.revoke(&actor);
    store.release.notify_one();
    assert_eq!(reader.await.unwrap(), Err(Error::Core(rom::Error::Denied)));
}
#[tokio::test]
async fn unknown_object_ack_retries_complete_input_without_overwrite() {
    let (runtime, service, store) = controlled(Limits::default());
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    store.unknown_create.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.upload(&actor, "one", upload(b"hello")).await,
        Err(Error::Unknown)
    ));
    assert_eq!(
        runtime
            .read::<Blob>(&actor, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .state,
        BlobState::Pending
    );
    assert_eq!(store.memory.0.lock().unwrap().len(), 1);
    assert!(matches!(
        service
            .upload(&actor, "one", upload(b"hello"))
            .await
            .unwrap(),
        UploadOutcome::Attached(_)
    ));
    assert_eq!(store.memory.0.lock().unwrap().len(), 1);
    let detached = service.detach(&actor, "one").await.unwrap();
    assert_eq!(
        service.detach(&actor, "one").await.unwrap().key,
        detached.key
    );
    assert!(matches!(
        service.read(&actor, "one").await,
        Err(Error::Missing)
    ));
    assert_eq!(store.head(&detached.key).await.unwrap().bytes, 5);
}
#[tokio::test]
async fn changed_reservation_returns_explicit_unattached_object() {
    let (runtime, service, store) = controlled(Limits::default());
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    store.block_create.store(true, Ordering::SeqCst);
    let (s, a) = (service.clone(), actor.clone());
    let writer = tokio::spawn(async move { s.upload(&a, "one", upload(b"hello")).await });
    store.started.notified().await;
    runtime
        .execute(
            &actor,
            Command::<Blob>::delete("one")
                .at_revision(1)
                .idempotency("cancel"),
        )
        .await
        .unwrap();
    store.release.notify_one();
    let UploadOutcome::Unattached { object, .. } = writer.await.unwrap().unwrap() else {
        panic!("attachment must lose revision race")
    };
    assert_eq!(store.get(&object.key, 5).await.unwrap(), b"hello");
}
#[tokio::test]
async fn input_bounds_and_timeout_publish_nothing() {
    let (_, service, store) = controlled(Limits {
        blob_bytes: 5,
        chunk_bytes: 3,
        chunks: 2,
        staging_timeout: Duration::from_millis(20),
        ..Default::default()
    });
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    assert!(matches!(
        service.upload(&actor, "one", upload(b"hello")).await,
        Err(Error::TooLarge)
    ));
    let flood = Box::pin(futures_util::stream::iter(vec![
        Ok(vec![]),
        Ok(vec![]),
        Ok(vec![]),
    ]));
    assert!(matches!(
        service.upload(&actor, "one", flood).await,
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        service
            .upload(&actor, "one", Box::pin(futures_util::stream::pending()))
            .await,
        Err(Error::Timeout)
    ));
    assert!(store.memory.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn backend_panic_releases_work_and_closes_service() {
    let (_, service, store) = controlled(Limits::default());
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    store.panic_create.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.upload(&actor, "one", upload(b"hello")).await,
        Err(Error::Panicked)
    ));
    assert_eq!(service.read(&actor, "one").await, Err(Error::Closed));
    assert_eq!(service.shutdown().await, Err(Error::Panicked));
    assert!(store.memory.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn lost_metadata_ack_is_explicit_and_retry_resolves_one_attachment() {
    let database = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(definition())
        .build(database.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let store = Arc::new(Memory::default());
    let service = BlobService::builder(runtime.clone())
        .store("attachments", store.clone())
        .build()
        .unwrap();
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    database.inject_fault(5);
    assert!(matches!(
        service
            .upload(&actor, "one", upload(b"hello"))
            .await
            .unwrap(),
        UploadOutcome::Unattached {
            cause: rom::Error::Unknown,
            ..
        }
    ));
    assert_eq!(
        runtime
            .read::<Blob>(&actor, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .state,
        BlobState::Ready
    );
    assert!(matches!(
        service
            .upload(&actor, "one", upload(b"hello"))
            .await
            .unwrap(),
        UploadOutcome::Attached(_)
    ));
    assert_eq!(database.counts().unwrap(), [1, 2, 2, 0]);
    assert_eq!(store.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn pending_detachment_keeps_one_cleanup_identity_and_cannot_publish() {
    let (_, service, store) = setup();
    let actor = Actor::trusted("test", "alice");
    reserve(&service, &actor).await;
    let first = service.detach(&actor, "one").await.unwrap();
    assert_eq!(first.key, service.detach(&actor, "one").await.unwrap().key);
    assert!(matches!(
        service.upload(&actor, "one", upload(b"hello")).await,
        Err(Error::Conflict)
    ));
    assert!(store.0.lock().unwrap().is_empty());
}
#[derive(Default)]
struct Memory(Mutex<BTreeMap<String, Vec<u8>>>);
impl BlobStore for Memory {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let mut data = self.0.lock().unwrap();
            if data.contains_key(key.as_str()) {
                return Err(Error::Conflict);
            }
            data.insert(key.as_str().into(), bytes);
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let data = self.0.lock().unwrap();
            let bytes = data.get(key.as_str()).ok_or(Error::Missing)?;
            if bytes.len() > max {
                return Err(Error::TooLarge);
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
            self.0.lock().unwrap().remove(key.as_str());
            Ok(())
        })
    }
}
fn upload(bytes: &[u8]) -> Upload {
    Box::pin(futures_util::stream::iter(vec![Ok(bytes.to_vec())]))
}
fn setup() -> (Runtime, BlobService, Arc<Memory>) {
    let runtime = Runtime::builder()
        .resource(definition())
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap();
    let store = Arc::new(Memory::default());
    let stores = BTreeMap::from([("attachments".into(), store.clone() as Arc<dyn BlobStore>)]);
    let service = BlobService::new(runtime.clone(), stores, Limits::default()).unwrap();
    (runtime, service, store)
}
#[tokio::test]
async fn complete_verified_upload_is_attached_and_private() {
    let (runtime, service, store) = setup();
    let alice = Actor::trusted("test", "alice");
    service
        .reserve(
            &alice,
            "one",
            "attachments",
            Digest::of(b"hello"),
            5,
            "reserve",
        )
        .await
        .unwrap();
    assert!(matches!(
        service
            .upload(&alice, "one", upload(b"hello"))
            .await
            .unwrap(),
        UploadOutcome::Attached(_)
    ));
    assert_eq!(service.read(&alice, "one").await.unwrap(), b"hello");
    assert!(
        service
            .read(&Actor::trusted("test", "mallory"), "one")
            .await
            .is_err()
    );
    assert_eq!(store.0.lock().unwrap().len(), 1);
    service.shutdown().await.unwrap();
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn generic_invocation_cannot_forge_ready_or_replace_pending_with_ready() {
    let (runtime, service, _) = setup();
    let alice = Actor::trusted("test", "alice");
    let pending = service
        .reserve(
            &alice,
            "one",
            "attachments",
            Digest::of(b"hello"),
            5,
            "reserve",
        )
        .await
        .unwrap();
    let mut forged = pending.value.unwrap();
    forged.state = BlobState::Ready;
    assert!(matches!(
        runtime
            .execute(
                &alice,
                Command::replace("one", forged.clone())
                    .at_revision(1)
                    .idempotency("forge")
            )
            .await,
        Err(rom::Error::Denied)
    ));
    assert!(matches!(
        runtime
            .execute(
                &alice,
                Command::create("forged", forged).idempotency("forge-create")
            )
            .await,
        Err(rom::Error::Denied)
    ));
}
#[tokio::test]
async fn conflicting_or_partial_input_never_publishes() {
    let (_, service, store) = setup();
    let alice = Actor::trusted("test", "alice");
    service
        .reserve(
            &alice,
            "one",
            "attachments",
            Digest::of(b"hello"),
            5,
            "reserve",
        )
        .await
        .unwrap();
    assert!(
        service
            .upload(&alice, "one", upload(b"wrong"))
            .await
            .is_err()
    );
    let failed = Box::pin(futures_util::stream::iter(vec![
        Ok(b"he".to_vec()),
        Err(Error::Input),
    ]));
    assert!(service.upload(&alice, "one", failed).await.is_err());
    assert!(store.0.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_releases_all_adapter_owners_before_return() {
    for _ in 0..32 {
        let (runtime, service, store) = controlled(Limits::default());
        let actor = Actor::trusted("test", "alice");
        reserve(&service, &actor).await;
        store.block_create.store(true, Ordering::SeqCst);
        let (s, a) = (service.clone(), actor.clone());
        let caller = tokio::spawn(async move { s.upload(&a, "one", upload(b"hello")).await });
        store.started.notified().await;
        caller.abort();
        let _ = caller.await;
        let weak = Arc::downgrade(&store);
        store.release.notify_one();
        service.shutdown().await.unwrap();
        runtime.shutdown().await.unwrap();
        drop(service);
        drop(runtime);
        drop(store);
        assert!(
            weak.upgrade().is_none(),
            "drained service still retains backend owner"
        );
    }
}
