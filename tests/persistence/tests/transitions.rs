use rom::*;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Debug, Resource)]
#[resource(name = "guarded-counters")]
struct Counter {
    amount: u64,
    locked: bool,
}
const NOTICE: Channel<u64> = Channel::new("transition-notice", 1);
const SET: Action<Counter, u64> = Action::new("set", |counter, amount| {
    counter.amount = amount;
    Ok(vec![NOTICE.intent(amount)])
});
fn validate(actor: &Actor, before: Option<&Counter>, after: Option<&Counter>) -> Result<()> {
    if actor.subject == "validator-denied" {
        return Err(Error::Denied);
    }
    if after.is_some_and(|r| r.amount == 99) {
        panic!("injected validator panic");
    }
    if after.is_some_and(|r| r.amount > 10) {
        return Err(Error::invalid(Counter::KIND, "amount"));
    }
    if let Some(old) = before.filter(|old| old.locked)
        && after.is_none_or(|new| !new.locked || new.amount < old.amount)
    {
        return Err(Error::invalid(Counter::KIND, "locked transition"));
    }
    Ok(())
}
type Counts = Box<dyn Fn() -> Result<[u64; 4]>>;
struct Fixture {
    runtime: Runtime,
    storage: Arc<dyn Storage>,
    counts: Counts,
    _files: FixtureFiles,
}
struct FixtureFiles(PathBuf);
impl Drop for FixtureFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Fixture {
    fn new(redb: bool) -> Self {
        let builder = Runtime::builder()
            .resource(
                Counter::definition()
                    .policy(|a, _, _| a.subject != "policy-denied")
                    .allow_all_fields()
                    .validate_transition(validate)
                    .action(SET),
            )
            .channel(
                NOTICE,
                actor().with_kind(PrincipalKind::Service),
                |_| async { DeliveryOutcome::Accepted },
            );
        Self::with_builder(redb, builder)
    }
    fn with_builder(redb: bool, builder: Builder) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "rom-transitions-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let files = FixtureFiles(directory);
        let path = files.0.join("database");
        let (storage, counts): (Arc<dyn Storage>, Counts) = if redb {
            let storage = Arc::new(rom_redb::Redb::open(&path).unwrap());
            let inspected = storage.clone();
            (storage, Box::new(move || inspected.counts()))
        } else {
            let storage = Arc::new(rom_sqlite::Sqlite::open(&path).unwrap());
            let inspected = storage.clone();
            (storage, Box::new(move || inspected.counts()))
        };
        let runtime = builder
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self {
            runtime,
            storage,
            counts,
            _files: files,
        }
    }
    async fn create(&self, amount: u64, locked: bool) {
        self.runtime
            .execute(
                &actor(),
                Command::create("one", Counter { amount, locked }).idempotency("create"),
            )
            .await
            .unwrap();
    }
    async fn assert_unchanged(&self, revision: u64, event_count: usize, work_count: usize) {
        assert_eq!(
            self.runtime
                .read::<Counter>(&actor(), "one")
                .await
                .unwrap()
                .revision,
            revision
        );
        assert_eq!(
            self.runtime
                .journal(&actor(), Counter::KIND, None)
                .await
                .unwrap()
                .events
                .len(),
            event_count
        );
        assert_eq!(self.storage.reaction_records().unwrap().len(), work_count);
    }
}
fn actor() -> Actor {
    Actor::trusted("tests", "owner")
}

#[tokio::test]
async fn every_mutation_path_validates_and_rejection_is_atomic() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        assert!(matches!(
            f.runtime
                .execute(
                    &actor(),
                    Command::create(
                        "one",
                        Counter {
                            amount: 11,
                            locked: true
                        }
                    )
                    .idempotency("create")
                )
                .await,
            Err(Error::Invalid { .. })
        ));
        // Reusing the rejected identity with changed input proves no receipt was written.
        f.create(4, true).await;
        for (n, command) in [
            Command::replace(
                "one",
                Counter {
                    amount: 3,
                    locked: true,
                },
            ),
            Command::patch("one", Patch::new().set(Counter::amount_field(), 3)),
            Command::action("one", SET, 3),
            Command::delete("one"),
        ]
        .into_iter()
        .enumerate()
        {
            let command = command.at_revision(1).idempotency(&format!("invalid-{n}"));
            let result = if n % 2 == 0 {
                f.runtime.execute(&actor(), command).await.map(|_| ())
            } else {
                f.runtime.invoke(&actor(), command.into()).await.map(|_| ())
            };
            assert!(result.is_err());
            f.assert_unchanged(1, 1, 0).await;
        }
        let valid = Command::action("one", SET, 5)
            .at_revision(1)
            .idempotency("invalid-2");
        f.runtime.invoke(&actor(), valid.into()).await.unwrap();
        f.assert_unchanged(2, 2, 1).await;
        f.runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn receipts_do_not_revalidate_obsolete_transitions_but_authority_still_applies() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        f.create(4, true).await;
        let earlier = || {
            Command::action("one", SET, 5)
                .at_revision(1)
                .idempotency("earlier")
        };
        f.runtime.execute(&actor(), earlier()).await.unwrap();
        f.runtime
            .execute(
                &actor(),
                Command::action("one", SET, 6)
                    .at_revision(2)
                    .idempotency("later"),
            )
            .await
            .unwrap();
        let receipt = f.runtime.execute(&actor(), earlier()).await.unwrap();
        assert_eq!(receipt.revision, 2);
        assert_eq!(receipt.value.unwrap().amount, 5);
        f.assert_unchanged(3, 3, 2).await;
        f.runtime.revoke(&actor());
        assert!(matches!(
            f.runtime.execute(&actor(), earlier()).await,
            Err(Error::Denied)
        ));
        f.runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn validator_panic_does_not_poison_commit_gate_and_cannot_grant_authority() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        f.create(4, false).await;
        // Stale revisions and policy denial remain authoritative before user validation.
        assert!(matches!(
            f.runtime
                .execute(
                    &actor(),
                    Command::action("one", SET, 99)
                        .at_revision(0)
                        .idempotency("stale")
                )
                .await,
            Err(Error::Conflict)
        ));
        assert!(matches!(
            f.runtime
                .execute(
                    &Actor::trusted("tests", "policy-denied"),
                    Command::action("one", SET, 99)
                        .at_revision(1)
                        .idempotency("denied-panic")
                )
                .await,
            Err(Error::Denied)
        ));
        assert!(matches!(
            f.runtime
                .execute(
                    &actor(),
                    Command::action("one", SET, 99)
                        .at_revision(1)
                        .idempotency("panic")
                )
                .await,
            Err(Error::Panicked)
        ));
        f.assert_unchanged(1, 1, 0).await;
        for subject in ["validator-denied", "policy-denied"] {
            assert!(matches!(
                f.runtime
                    .execute(
                        &Actor::trusted("tests", subject),
                        Command::action("one", SET, 5)
                            .at_revision(1)
                            .idempotency(subject)
                    )
                    .await,
                Err(Error::Denied)
            ));
        }
        f.runtime
            .execute(
                &actor(),
                Command::<Counter>::delete("one")
                    .at_revision(1)
                    .idempotency("delete"),
            )
            .await
            .unwrap();
        assert_eq!(
            f.storage
                .journal(Counter::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        f.runtime.shutdown().await.unwrap();
    }
}

#[derive(Clone)]
struct Canonical(String);
impl Field for Canonical {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_str()
            .map(|s| Self(s.trim().to_ascii_uppercase()))
            .ok_or_else(|| Error::invalid("canonical", "string"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "normalized-labels")]
struct Label {
    name: Canonical,
}
#[tokio::test]
async fn transition_sees_normalized_previous_and_candidate_values() {
    for redb in [false, true] {
        let builder = Runtime::builder().resource(
            Label::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .validate_transition(|_, old, new| {
                    if old.is_some_and(|label| label.name.0 != "FIRST")
                        || new.is_none_or(|label| {
                            !matches!(label.name.0.as_str(), "FIRST" | "SECOND")
                        })
                    {
                        return Err(Error::invalid(Label::KIND, "unnormalized"));
                    }
                    Ok(())
                }),
        );
        let f = Fixture::with_builder(redb, builder);
        let runtime = &f.runtime;
        runtime
            .execute(
                &actor(),
                Command::create(
                    "label",
                    Label {
                        name: Canonical(" first ".into()),
                    },
                )
                .idempotency("label-create"),
            )
            .await
            .unwrap();
        let changed = runtime
            .execute(
                &actor(),
                Command::patch(
                    "label",
                    Patch::new().set(Label::name_field(), Canonical(" second ".into())),
                )
                .at_revision(1)
                .idempotency("label-patch"),
            )
            .await
            .unwrap();
        assert_eq!(changed.value.unwrap().name.0, "SECOND");
        runtime.shutdown().await.unwrap();
    }
}

/// Captured snapshots must belong to their Runtime, not a global Resource kind.
#[tokio::test]
async fn captured_catalogs_are_isolated_between_real_runtimes() {
    use std::sync::RwLock;
    for redb in [false, true] {
        let first_catalog = Arc::new(RwLock::new(Arc::new(4_u64)));
        let second_catalog = Arc::new(RwLock::new(Arc::new(8_u64)));
        let build = |catalog: Arc<RwLock<Arc<u64>>>| {
            Runtime::builder().resource(
                Counter::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields()
                    .validate_transition(move |_, _, after| {
                        let snapshot = catalog.read().unwrap().clone();
                        if after.is_some_and(|row| row.amount > *snapshot) {
                            return Err(Error::invalid(Counter::KIND, "catalog limit"));
                        }
                        Ok(())
                    }),
            )
        };
        let first = Fixture::with_builder(redb, build(first_catalog.clone()));
        let second = Fixture::with_builder(redb, build(second_catalog));
        assert!(!first.runtime.same_instance(&second.runtime));
        for fixture in [&first, &second] {
            fixture.create(4, false).await;
        }
        let replace = || {
            Command::replace(
                "one",
                Counter {
                    amount: 6,
                    locked: false,
                },
            )
            .at_revision(1)
            .idempotency("same-command")
        };
        assert!(matches!(
            first.runtime.execute(&actor(), replace()).await,
            Err(Error::Invalid { .. })
        ));
        second.runtime.execute(&actor(), replace()).await.unwrap();
        assert_eq!((first.counts)().unwrap(), [1, 1, 1, 0]);
        assert_eq!((second.counts)().unwrap(), [1, 2, 2, 0]);
        *first_catalog.write().unwrap() = Arc::new(7);
        first.runtime.execute(&actor(), replace()).await.unwrap();
        *first_catalog.write().unwrap() = Arc::new(0);
        let next = || {
            Command::replace(
                "one",
                Counter {
                    amount: 7,
                    locked: false,
                },
            )
            .at_revision(2)
            .idempotency("next-command")
        };
        assert!(matches!(
            first.runtime.execute(&actor(), next()).await,
            Err(Error::Invalid { .. })
        ));
        second.runtime.execute(&actor(), next()).await.unwrap();
        assert_eq!(
            first
                .runtime
                .read::<Counter>(&actor(), "one")
                .await
                .unwrap()
                .value
                .unwrap()
                .amount,
            6
        );
        assert_eq!(
            second
                .runtime
                .read::<Counter>(&actor(), "one")
                .await
                .unwrap()
                .value
                .unwrap()
                .amount,
            7
        );
        assert_eq!((first.counts)().unwrap(), [1, 2, 2, 0]);
        assert_eq!((second.counts)().unwrap(), [1, 3, 3, 0]);
        first.runtime.shutdown().await.unwrap();
        second.runtime.shutdown().await.unwrap();
    }
}

/// Neither callback failure nor replay may create another durable bundle.
#[tokio::test]
async fn captured_validation_preserves_atomic_failures_and_authorized_replay() {
    use std::sync::RwLock;
    for redb in [false, true] {
        let catalog = Arc::new(RwLock::new(Arc::new(6_u64)));
        let captured = catalog.clone();
        let calls = Arc::new(AtomicU64::new(0));
        let observed = calls.clone();
        let builder = Runtime::builder()
            .resource(
                Counter::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields()
                    .action(SET)
                    .validate_transition(move |_, _, after| {
                        observed.fetch_add(1, Ordering::Relaxed);
                        let snapshot = captured.read().unwrap().clone();
                        if after.is_some_and(|row| row.amount == 99) {
                            panic!("captured validator panic");
                        }
                        if after.is_some_and(|row| row.amount > *snapshot) {
                            return Err(Error::invalid(Counter::KIND, "catalog limit"));
                        }
                        Ok(())
                    }),
            )
            .channel(
                NOTICE,
                actor().with_kind(PrincipalKind::Service),
                |_| async { DeliveryOutcome::Accepted },
            );
        let fixture = Fixture::with_builder(redb, builder);
        fixture.create(4, false).await;
        let original_row = fixture
            .storage
            .snapshot(Counter::KIND, 10, 100_000)
            .unwrap();
        let original_journal = fixture
            .storage
            .journal(Counter::KIND, None, 10, 100_000)
            .unwrap();
        let original_work = fixture.storage.reaction_records().unwrap();
        for amount in [7, 99] {
            let result = fixture
                .runtime
                .execute(
                    &actor(),
                    Command::action("one", SET, amount)
                        .at_revision(1)
                        .idempotency("rejected"),
                )
                .await;
            if amount == 99 {
                assert!(matches!(result, Err(Error::Panicked)));
            } else {
                assert!(matches!(result, Err(Error::Invalid { .. })));
            }
            assert_eq!((fixture.counts)().unwrap(), [1, 1, 1, 0]);
            assert_eq!(
                fixture
                    .storage
                    .snapshot(Counter::KIND, 10, 100_000)
                    .unwrap(),
                original_row
            );
            assert_eq!(
                fixture
                    .storage
                    .journal(Counter::KIND, None, 10, 100_000)
                    .unwrap(),
                original_journal
            );
            assert_eq!(fixture.storage.reaction_records().unwrap(), original_work);
        }
        // Reuse the rejected identity: no failed callback stored a receipt.
        let accepted = || {
            Command::action("one", SET, 5)
                .at_revision(1)
                .idempotency("rejected")
        };
        fixture.runtime.execute(&actor(), accepted()).await.unwrap();
        fixture.assert_unchanged(2, 2, 1).await;
        assert_eq!((fixture.counts)().unwrap(), [1, 2, 2, 1]);
        let committed_calls = calls.load(Ordering::Relaxed);
        let committed_work = fixture.storage.reaction_records().unwrap();
        *catalog.write().unwrap() = Arc::new(0);
        let replay = fixture.runtime.execute(&actor(), accepted()).await.unwrap();
        assert_eq!(replay.revision, 2);
        assert_eq!(replay.value.unwrap().amount, 5);
        assert_eq!(calls.load(Ordering::Relaxed), committed_calls);
        assert_eq!((fixture.counts)().unwrap(), [1, 2, 2, 1]);
        assert_eq!(fixture.storage.reaction_records().unwrap(), committed_work);
        assert!(matches!(
            fixture
                .runtime
                .execute(
                    &actor(),
                    Command::action("one", SET, 6)
                        .at_revision(1)
                        .idempotency("rejected")
                )
                .await,
            Err(Error::IdentityMismatch)
        ));
        fixture.runtime.revoke(&actor());
        assert!(matches!(
            fixture.runtime.execute(&actor(), accepted()).await,
            Err(Error::Denied)
        ));
        assert_eq!(calls.load(Ordering::Relaxed), committed_calls);
        assert_eq!((fixture.counts)().unwrap(), [1, 2, 2, 1]);
        assert_eq!(fixture.storage.reaction_records().unwrap(), committed_work);
        fixture.runtime.shutdown().await.unwrap();
    }
}
