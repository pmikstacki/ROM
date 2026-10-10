//! Optional native Work capabilities survive the fixture observer unchanged.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Native {
    ledger: Mutex<WorkLedger>,
    singles: AtomicUsize,
    prefixes: Mutex<Vec<(u64, usize)>>,
    live: Mutex<Vec<(ClaimKey, u64)>>,
    atomic: Mutex<Vec<Vec<WorkUpdate>>>,
}
fn pending(id: &str) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            retry_epoch: 0,
            root: format!("root-{id}"),
            parent: None,
            depth: 1,
            started_at: 10,
            path: vec![id.into()],
        },
        definition: "observer-map".into(),
        version: 1,
        service_key: "observer-service".into(),
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        payload: WorkPayload::Source(Row {
            key: Key {
                kind: "observer-source".into(),
                id: id.into(),
            },
            revision: 1,
            value: Some(json!({"enabled":true})),
            protected: Default::default(),
        }),
    }
}
fn fixture() -> (Arc<Native>, ObservedStorage) {
    let native = Arc::new(Native {
        ledger: Mutex::new(WorkLedger::default()),
        singles: AtomicUsize::new(0),
        prefixes: Mutex::new(vec![]),
        live: Mutex::new(vec![]),
        atomic: Mutex::new(vec![]),
    });
    let observer = ObservedStorage::new(native.clone());
    for id in ["a", "b"] {
        let work = pending(id);
        let WorkPayload::Source(row) = &work.payload else {
            unreachable!()
        };
        observer
            .commit(&Bundle {
                expected: None,
                changed: true,
                effects: vec![],
                completed_work: None,
                receipt: Receipt {
                    retry_epoch: 0,
                    replay_version: None,
                    identity: id.into(),
                    fingerprint: id.into(),
                    row: row.clone(),
                },
                reaction_limits: Some(ReactionLimits::default()),
                reactions: vec![work],
            })
            .unwrap();
    }
    (native, observer)
}
impl Storage for Native {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        unreachable!()
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        unreachable!()
    }
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        unreachable!()
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.ledger.lock().unwrap().enqueue(
            bundle.reaction_limits.as_ref().unwrap(),
            bundle.reactions.clone(),
        )?;
        Ok(bundle.receipt.clone())
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.singles.fetch_add(1, Ordering::SeqCst);
        self.ledger.lock().unwrap().apply(update)
    }
    fn reaction_claim_prefix(&self, now: u64, max_claims: usize) -> Result<Vec<WorkClaim>> {
        self.prefixes.lock().unwrap().push((now, max_claims));
        if now == 99 {
            return Err(Error::Unknown);
        }
        let mut ledger = self.ledger.lock().unwrap();
        let mut claims = vec![];
        for _ in 0..max_claims {
            let WorkResult::Claimed(claim) = ledger.apply(WorkUpdate::Claim { now })? else {
                break;
            };
            claims.push(*claim);
        }
        Ok(claims)
    }
    fn reaction_claim_live(&self, claim: &ClaimKey, now: u64) -> Result<Option<bool>> {
        self.live.lock().unwrap().push((claim.clone(), now));
        if now == 99 {
            return Err(Error::Storage);
        }
        if now == 98 {
            return Ok(None);
        }
        Ok(Some(self.ledger.lock().unwrap().records().iter().any(|r| r.pending.id == claim.id && r.generation == claim.generation && matches!(r.state, WorkState::Leased { until, generation, .. } if until > now && generation == claim.generation))))
    }
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        self.atomic.lock().unwrap().push(updates.clone());
        if updates
            .iter()
            .any(|update| matches!(update, WorkUpdate::Materialize { now: 99, .. }))
        {
            return Err(Error::Unknown);
        }
        let mut ledger = self.ledger.lock().unwrap();
        let mut candidate = ledger.clone();
        let results = updates
            .into_iter()
            .map(|update| candidate.apply(update))
            .collect::<Result<Vec<_>>>()?;
        *ledger = candidate;
        Ok(results)
    }
}
#[test]
fn prefix_forwarding_uses_native_group_and_records_each_acknowledged_queue_claim_once() {
    let (native, observer) = fixture();
    let claims = observer.reaction_claim_prefix(10, 32).unwrap();
    assert_eq!(claims.len(), 2);
    assert_eq!(
        claims
            .iter()
            .map(|c| c.work.pending.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(*native.prefixes.lock().unwrap(), vec![(10, 32)]);
    assert_eq!(native.singles.load(Ordering::SeqCst), 0);
    let (timings, missing, rejected) = observer.timings().unwrap();
    assert_eq!((timings.len(), missing, rejected), (2, 0, 0));
    assert_eq!(observer.reaction_claim_prefix(99, 7), Err(Error::Unknown));
    assert_eq!(observer.timings().unwrap().0.len(), 2);
}
#[test]
fn keyed_live_forwarding_preserves_key_time_true_false_none_and_errors() {
    let (native, observer) = fixture();
    let claim = native.reaction_claim_prefix(10, 2).unwrap().remove(0).key();
    assert_eq!(observer.reaction_claim_live(&claim, 10), Ok(Some(true)));
    let stale = ClaimKey {
        id: claim.id.clone(),
        generation: 0,
    };
    assert_eq!(observer.reaction_claim_live(&stale, 10), Ok(Some(false)));
    assert_eq!(observer.reaction_claim_live(&claim, 98), Ok(None));
    assert_eq!(
        observer.reaction_claim_live(&claim, 99),
        Err(Error::Storage)
    );
    assert_eq!(
        *native.live.lock().unwrap(),
        vec![
            (claim.clone(), 10),
            (stale, 10),
            (claim.clone(), 98),
            (claim, 99)
        ]
    );
}
#[test]
fn atomic_forwarding_preserves_vector_results_and_unknown_without_singleton_replay() {
    let (native, observer) = fixture();
    let claims = native.reaction_claim_prefix(10, 2).unwrap();
    let unknown = vec![WorkUpdate::Materialize {
        claim: claims[0].key(),
        now: 99,
        children: vec![],
    }];
    assert_eq!(
        observer.reaction_updates_atomic(unknown.clone()),
        Err(Error::Unknown)
    );
    let updates: Vec<_> = claims
        .into_iter()
        .map(|claim| WorkUpdate::Materialize {
            claim: claim.key(),
            now: 10,
            children: vec![],
        })
        .collect();
    assert_eq!(
        observer.reaction_updates_atomic(updates.clone()),
        Ok(vec![WorkResult::Changed; 2])
    );
    assert_eq!(*native.atomic.lock().unwrap(), vec![unknown, updates]);
    assert_eq!(native.singles.load(Ordering::SeqCst), 0);
    assert!(
        native
            .ledger
            .lock()
            .unwrap()
            .records()
            .iter()
            .all(|r| r.state == WorkState::Done)
    );
}

#[tokio::test]
async fn actual_load_fixture_reaches_native_grouped_path_on_both_adapters() {
    for adapter in ["sqlite", "redb"] {
        let directory = std::env::temp_dir().join(format!(
            "rom-load-wrapper-{adapter}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let db = crate::database::Database::open(adapter, &directory.join("database")).unwrap();
        let storage = Arc::new(ObservedStorage::new(db.storage()));
        let runtime =
            crate::application::runtime(storage.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
        for index in 0..2 {
            runtime
                .execute(
                    &rom_demo::bootstrap_actor(),
                    crate::application::seeded(index),
                )
                .await
                .unwrap();
        }
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        let work = storage.reaction_records().unwrap();
        assert_eq!(work.len(), 2);
        assert!(
            work.iter()
                .all(|record| record.state == WorkState::Done && record.attempts == 1)
        );
        let calls = serde_json::to_value(storage.calls().unwrap()).unwrap();
        let count = |name: &str| {
            calls
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["call"] == name)
                .map(|entry| entry["counts"]["observed_calls"].as_u64().unwrap())
                .unwrap_or(0)
        };
        assert_eq!(count("claim_prefix"), 1);
        assert_eq!(count("atomic_work_updates"), 1);
        assert!(count("claim_live") >= 2);
        assert_eq!((count("claim"), count("materialize")), (0, 0));
        let (timings, missing, rejected) = storage.timings().unwrap();
        assert_eq!((timings.len(), missing, rejected), (2, 0, 0));
        runtime.shutdown().await.unwrap();
    }
}
