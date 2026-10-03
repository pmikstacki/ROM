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
struct Fixture {
    runtime: Runtime,
    storage: Arc<dyn Storage>,
    path: PathBuf,
}
impl Fixture {
    fn new(redb: bool) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-transitions-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = Runtime::builder()
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
            )
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self {
            runtime,
            storage,
            path,
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
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
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
        let f = Fixture::new(redb);
        let runtime = Runtime::builder()
            .resource(
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
            )
            .build(f.storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
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
        f.runtime.shutdown().await.unwrap();
    }
}
