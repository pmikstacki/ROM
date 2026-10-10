//! Existing adapters retain singleton semantics; transparent wrappers forward optional grouping.
#[path = "support/work_atomic_batch_fixture.rs"]
mod fixture;
use rom::*;
use std::sync::Arc;
struct Legacy(Arc<dyn Storage>);
struct Forward(Arc<dyn Storage>);
macro_rules! existing_storage {
    () => {
        fn acquire_owner(&self) -> Result<StorageOwner> {
            self.0.acquire_owner()
        }
        fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
            self.0.register(descriptors)
        }
        fn capabilities(&self) -> Capabilities {
            self.0.capabilities()
        }
        fn supports_reactions(&self) -> bool {
            self.0.supports_reactions()
        }
        fn load(&self, key: &Key) -> Result<Option<Row>> {
            self.0.load(key)
        }
        fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
            self.0.receipt(identity)
        }
        fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
            self.0.commit(bundle)
        }
        fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
            self.0.reaction_update(update)
        }
        fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
            self.0.reaction_records()
        }
        fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
            self.0.snapshot(kind, rows, bytes)
        }
    };
}
impl Storage for Legacy {
    existing_storage!();
}
impl Storage for Forward {
    existing_storage!();
    fn reaction_claim_prefix(&self, now: u64, count: usize) -> Result<Vec<WorkClaim>> {
        self.0.reaction_claim_prefix(now, count)
    }
    fn reaction_claim_live(&self, claim: &ClaimKey, now: u64) -> Result<Option<bool>> {
        self.0.reaction_claim_live(claim, now)
    }
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        self.0.reaction_updates_atomic(updates)
    }
}
fn native(redb: bool, label: &str) -> Arc<dyn Storage> {
    let path = std::env::temp_dir().join(format!(
        "rom-default-prefix-{label}-{redb}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
#[test]
fn legacy_default_claims_exactly_one_and_atomic_default_rejects_before_mutation() {
    for redb in [false, true] {
        let storage = Legacy(native(redb, "legacy"));
        fixture::seed(&storage);
        let before = storage.reaction_records().unwrap();
        assert_eq!(storage.reaction_claim_prefix(0, 0), Err(Error::TooLarge));
        assert_eq!(storage.reaction_claim_prefix(0, 33), Err(Error::TooLarge));
        assert_eq!(storage.reaction_records().unwrap(), before);
        let claims = storage.reaction_claim_prefix(0, 32).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].work.pending.id, "a");
        let after_claim = storage.reaction_records().unwrap();
        assert_eq!(after_claim[1].attempts, 0);
        assert!(matches!(
            storage.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 2]),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(storage.reaction_records().unwrap(), after_claim);
        assert!(storage.reaction_updates_atomic(vec![]).unwrap().is_empty());
        storage
            .reaction_updates_atomic(vec![WorkUpdate::Materialize {
                claim: claims[0].key(),
                now: 0,
                children: vec![],
            }])
            .unwrap();
        assert_eq!(
            storage.reaction_records().unwrap()[0].state,
            WorkState::Done
        );
        assert_eq!(storage.reaction_claim_prefix(0, 32).unwrap().len(), 1);
        assert!(storage.reaction_claim_prefix(0, 32).unwrap().is_empty());
    }
}
#[test]
fn transparent_wrapper_retains_native_grouping_and_atomic_results() {
    for redb in [false, true] {
        let storage = Forward(native(redb, "forward"));
        fixture::seed(&storage);
        let claims = storage.reaction_claim_prefix(0, 32).unwrap();
        assert_eq!(claims.len(), 2);
        let updates = claims
            .into_iter()
            .map(|claim| WorkUpdate::Materialize {
                claim: claim.key(),
                now: 0,
                children: vec![],
            })
            .collect();
        assert_eq!(
            storage.reaction_updates_atomic(updates).unwrap(),
            vec![WorkResult::Changed; 2]
        );
        assert!(
            storage
                .reaction_records()
                .unwrap()
                .iter()
                .all(|record| record.state == WorkState::Done)
        );
    }
}
