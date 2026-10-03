#![cfg(feature = "blob")]
use rom_blob::{BlobStore, Error, Metadata, ObjectKey, StoreFuture};
use rom_conformance::{FailureCategory, blob};
use std::{collections::BTreeMap, sync::Mutex};

/// Deliberately violates immutable conditional create.
#[derive(Default)]
struct Overwriting(Mutex<BTreeMap<String, Vec<u8>>>);
impl BlobStore for Overwriting {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().insert(key.as_str().into(), bytes);
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let value = self
                .0
                .lock()
                .unwrap()
                .get(key.as_str())
                .cloned()
                .ok_or(Error::Missing)?;
            if value.len() > max_bytes {
                Err(Error::TooLarge)
            } else {
                Ok(value)
            }
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .get(key.as_str())
                .map(|v| Metadata {
                    bytes: v.len() as u64,
                })
                .ok_or(Error::Missing)
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().remove(key.as_str());
            Ok(())
        })
    }
}
#[tokio::test]
async fn overwriting_blob_adapter_fails_named_immutable_create_case() {
    let error = blob::basic(&Overwriting::default()).await.unwrap_err();
    assert_eq!(error.case, "blob.create.conflict");
    assert_eq!(error.category, FailureCategory::Assertion);
}
