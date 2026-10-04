//! Trusted local blob adapter and opt-in publication barrier for process shutdown trials.
use rom_blob::{BlobService, BlobStore, ObjectKey, StoreFuture};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::watch;

pub struct PublicationGate {
    paused: watch::Sender<bool>,
    entered: AtomicBool,
}
impl PublicationGate {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            paused: watch::channel(false).0,
            entered: AtomicBool::new(false),
        })
    }
    pub fn pause(&self) {
        self.entered.store(false, Ordering::SeqCst);
        self.paused.send_replace(true);
    }
    pub fn release(&self) {
        self.paused.send_replace(false);
    }
    pub fn entered(&self) -> bool {
        self.entered.load(Ordering::SeqCst)
    }
    async fn wait(&self) {
        let mut paused = self.paused.subscribe();
        if !*paused.borrow_and_update() {
            return;
        }
        self.entered.store(true, Ordering::SeqCst);
        while *paused.borrow_and_update() {
            if paused.changed().await.is_err() {
                break;
            }
        }
    }
}
struct GatedStore {
    inner: rom_blob_object_store::Adapter,
    gate: Arc<PublicationGate>,
}
impl BlobStore for GatedStore {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.gate.wait().await;
            self.inner.create(key, bytes).await
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>> {
        self.inner.get(key, max_bytes)
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, rom_blob::Metadata> {
        self.inner.head(key)
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        self.inner.delete(key)
    }
}
pub fn build(
    runtime: rom::Runtime,
    database: &Path,
    gate: Arc<PublicationGate>,
) -> rom_blob::Result<BlobService> {
    let folder = database.with_extension("attachments");
    std::fs::create_dir_all(&folder).map_err(|_| rom_blob::Error::Input)?;
    let limits = rom_blob::Limits::default();
    let adapter = rom_blob_object_store::Adapter::trusted_folder(&folder, limits.blob_bytes)?;
    BlobService::builder(runtime)
        .store(
            "local",
            Arc::new(GatedStore {
                inner: adapter,
                gate,
            }),
        )
        .limits(limits)
        .build()
}
