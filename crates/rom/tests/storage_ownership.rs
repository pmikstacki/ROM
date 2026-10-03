use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
struct Memory {
    ownership: StorageOwnership,
    fail_registration: AtomicBool,
    boundary_reads: AtomicUsize,
    registrations: AtomicUsize,
}
impl Storage for Memory {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.ownership.acquire()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.boundary_reads.fetch_add(1, Ordering::SeqCst);
        Ok(RetryEpochs::default())
    }
    fn register(&self, _: &[Descriptor]) -> Result<()> {
        self.registrations.fetch_add(1, Ordering::SeqCst);
        if self.fail_registration.load(Ordering::SeqCst) {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        Ok(None)
    }
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        Ok(vec![])
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        Ok(None)
    }
    fn commit(&self, _: &Bundle) -> Result<Receipt> {
        Err(Error::Unsupported("unused test commit".into()))
    }
}
struct Wrapped(Arc<Memory>);
impl Storage for Wrapped {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.0.acquire_owner()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.0.register(descriptors)
    }
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.0.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.0.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.0.receipt(id)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.0.commit(bundle)
    }
}
fn build(storage: Arc<dyn Storage>) -> Result<Runtime> {
    Runtime::builder().build(storage, Runtime::shared_cpu_pool(1).unwrap())
}

#[test]
fn storage_ownership_excludes_wrapped_and_direct_builders_before_storage_reads() {
    let storage = Arc::new(Memory::default());
    let owner = build(storage.clone()).unwrap();
    assert!(matches!(build(storage.clone()), Err(Error::Conflict)));
    assert!(matches!(
        build(Arc::new(Wrapped(storage.clone()))),
        Err(Error::Conflict)
    ));
    assert_eq!(storage.boundary_reads.load(Ordering::SeqCst), 1);
    assert_eq!(storage.registrations.load(Ordering::SeqCst), 1);
    drop(owner);
    let wrapped = build(Arc::new(Wrapped(storage.clone()))).unwrap();
    assert!(matches!(build(storage.clone()), Err(Error::Conflict)));
    drop(wrapped);
    assert!(build(storage).is_ok());
}

#[test]
fn storage_ownership_failed_registration_releases_claim() {
    let storage = Arc::new(Memory::default());
    storage.fail_registration.store(true, Ordering::SeqCst);
    assert!(matches!(build(storage.clone()), Err(Error::Storage)));
    storage.fail_registration.store(false, Ordering::SeqCst);
    let owner = build(storage.clone()).unwrap();
    assert_eq!(storage.registrations.load(Ordering::SeqCst), 2);
    assert!(matches!(build(storage.clone()), Err(Error::Conflict)));
    drop(owner);
    assert!(storage.acquire_owner().is_ok());
}

#[test]
fn storage_ownership_pure_validation_precedes_acquisition() {
    let storage = Arc::new(Memory::default());
    let _claim = storage.acquire_owner().unwrap();
    assert!(matches!(
        Runtime::builder()
            .capacity(0)
            .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap()),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(storage.boundary_reads.load(Ordering::SeqCst), 0);
    assert_eq!(storage.registrations.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn storage_ownership_shutdown_retains_claim_until_last_runtime_drops() {
    let storage = Arc::new(Memory::default());
    let owner = build(storage.clone()).unwrap();
    let retained = owner.clone();
    owner.shutdown().await.unwrap();
    drop(owner);
    assert!(matches!(build(storage.clone()), Err(Error::Conflict)));
    drop(retained);
    assert!(build(storage).is_ok());
}
