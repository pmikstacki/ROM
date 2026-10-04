use rom_blob::{BlobStore, Error, Metadata, ObjectKey, StoreFuture};
use std::{collections::BTreeMap, sync::Mutex};
#[derive(Default)]
pub struct Memory {
    data: Mutex<BTreeMap<String, Vec<u8>>>,
    pub pause_create: std::sync::atomic::AtomicBool,
    pub unknown_create: std::sync::atomic::AtomicBool,
    pub started: tokio::sync::Notify,
    pub release: tokio::sync::Notify,
}
impl BlobStore for Memory {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            if self
                .pause_create
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                self.started.notify_one();
                self.release.notified().await;
            }
            let mut data = self.data.lock().unwrap();
            if data.contains_key(key.as_str()) {
                return Err(Error::Conflict);
            }
            data.insert(key.as_str().into(), bytes);
            if self
                .unknown_create
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                return Err(Error::Unknown);
            }
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let data = self.data.lock().unwrap();
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
            self.data.lock().unwrap().remove(key.as_str());
            Ok(())
        })
    }
}
