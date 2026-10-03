use super::support::*;
use rom::*;
use std::sync::Mutex;

type Pause = (
    tokio::sync::oneshot::Sender<()>,
    std::sync::mpsc::Receiver<()>,
);
static PAUSE: Mutex<Option<Pause>> = Mutex::new(None);
struct Release(Option<std::sync::mpsc::Sender<()>>);
impl Release {
    fn finish(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.finish();
    }
}
fn paused(record: &mut Record, _: ()) -> Result<Vec<Intent>> {
    let (entered, release) = PAUSE.lock().unwrap().take().unwrap();
    let _ = entered.send(());
    release.recv().unwrap();
    record.value += 1;
    Ok(vec![])
}
const PAUSED: Action<Record, ()> = Action::new("paused", paused);

#[tokio::test]
async fn managed_identity_change_denies_accepted_proposal_and_all_subsequent_disclosure() {
    for redb in [false, true] {
        for target in identity_targets() {
            for cancel in [false, true] {
                let scratch = Scratch::new();
                let db = Database::open(redb, &scratch.path("database"));
                let runtime = base()
                    .resource(record_definition().action(PAUSED))
                    .build(db.storage(), Runtime::shared_cpu_pool(2).unwrap())
                    .unwrap();
                let activation = provision(&runtime).await;
                let actor = human(&runtime, &activation).await;
                let create: Invocation = Command::create(
                    "one",
                    Record {
                        owner: "reader".into(),
                        value: 7,
                    },
                )
                .idempotency("create")
                .into();
                runtime.invoke(&actor, create.clone()).await.unwrap();
                runtime
                    .execute(
                        &actor,
                        Command::create(
                            "ledger",
                            Ledger {
                                owner: "reader".into(),
                                total: 3,
                            },
                        )
                        .idempotency("ledger"),
                    )
                    .await
                    .unwrap();
                let mut live = runtime.live(&actor, Query::<Record>::all()).await.unwrap();
                assert_eq!(runtime.discover(&actor).await.unwrap().resources.len(), 2);
                let (entered, waiting) = tokio::sync::oneshot::channel();
                let (release, wait) = std::sync::mpsc::channel();
                let mut release = Release(Some(release));
                *PAUSE.lock().unwrap() = Some((entered, wait));
                let running = runtime.clone();
                let pending_actor = actor.clone();
                let task = tokio::spawn(async move {
                    running
                        .execute(
                            &pending_actor,
                            Command::action("one", PAUSED, ())
                                .at_revision(1)
                                .idempotency("paused"),
                        )
                        .await
                });
                tokio::time::timeout(BOUND, waiting).await.unwrap().unwrap();
                assert_eq!(runtime.available_capacity(), 1);
                if cancel {
                    task.abort();
                }
                // This mutation must finish while the accepted proposal remains paused.
                tokio::time::timeout(BOUND, enabled(&runtime, &target, 1, false))
                    .await
                    .unwrap();
                assert_eq!(
                    runtime.available_capacity(),
                    1,
                    "cancelled wait cannot release the accepted action permit"
                );
                assert!(runtime.status().unwrap().owned_work > 0);
                release.finish();
                if cancel {
                    assert!(task.await.unwrap_err().is_cancelled());
                    shutdown(&runtime).await;
                } else {
                    assert!(matches!(
                        tokio::time::timeout(BOUND, task).await.unwrap().unwrap(),
                        Err(Error::Denied)
                    ));
                    assert!(matches!(
                        runtime.invoke(&actor, create).await,
                        Err(Error::Denied)
                    ));
                    assert!(matches!(runtime.discover(&actor).await, Err(Error::Denied)));
                    assert!(matches!(
                        tokio::time::timeout(BOUND, live.changed()).await.unwrap(),
                        Err(Error::Denied)
                    ));
                    enabled(&runtime, &target, 2, true).await;
                    assert!(matches!(
                        runtime.read::<Ledger>(&actor, "ledger").await,
                        Err(Error::Denied)
                    ));
                    shutdown(&runtime).await;
                }
                let record = db
                    .storage()
                    .load(&Key {
                        kind: Record::KIND.into(),
                        id: "one".into(),
                    })
                    .unwrap()
                    .unwrap();
                assert_eq!(record.revision, 1);
                assert_eq!(Record::decode(record.value.unwrap()).unwrap().value, 7);
                let events = db
                    .storage()
                    .journal(Record::KIND, None, 16, 65536)
                    .unwrap()
                    .events;
                assert_eq!(
                    events.len(),
                    1,
                    "rejected accepted proposal cannot produce a business event"
                );
                drop(live);
                drop(runtime);
                drop(db);
                let reopened = Database::open(redb, &scratch.path("database"));
                let row = reopened
                    .storage()
                    .load(&Key {
                        kind: Record::KIND.into(),
                        id: "one".into(),
                    })
                    .unwrap()
                    .unwrap();
                assert_eq!(row.revision, 1);
            }
        }
    }
}
