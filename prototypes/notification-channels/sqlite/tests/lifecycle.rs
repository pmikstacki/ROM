use notification_core::{Channels, Delivery, Dispatcher, Outbox, Outcome};
use notification_sqlite_probe::SqliteOutbox;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Notice {
    text: String,
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-notify-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("outbox.db")
    }
    fn open(&self) -> SqliteOutbox {
        SqliteOutbox::open(self.path()).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dispatcher(channels: Channels) -> Dispatcher {
    Dispatcher {
        channels,
        max_attempts: 3,
        base_backoff: 10,
        lease: 10,
        timeout: Duration::from_secs(1),
    }
}
fn enqueue(db: &mut SqliteOutbox, channels: &Channels) {
    let intent = channels
        .intent(
            "delivery-1",
            "custom",
            "notice/v1",
            &Notice {
                text: "changed".into(),
            },
        )
        .unwrap();
    db.transition("resource-1", "new", &[intent]).unwrap();
}
async fn ordinary_custom_function(delivery: Delivery<Notice>) -> Outcome {
    Outcome::Accepted(format!("local:{}:{}", delivery.id.0, delivery.payload.text))
}
fn channels() -> Channels {
    let mut channels = Channels::default();
    channels
        .register("custom", "notice/v1", ordinary_custom_function)
        .unwrap();
    channels
}

// Would fail if the adapter committed resource writes before inserting all intents.
#[test]
fn sql_constraint_failure_rolls_back_resource_and_intents_across_reopen() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let channels = channels();
    db.transition("resource-1", "old", &[]).unwrap();
    let intent = channels
        .intent(
            "collision",
            "custom",
            "notice/v1",
            &Notice { text: "x".into() },
        )
        .unwrap();
    assert!(
        db.transition("resource-1", "new", &[intent.clone(), intent])
            .is_err()
    );
    drop(db);
    let db = scratch.open();
    assert_eq!(db.resource("resource-1").unwrap().as_deref(), Some("old"));
    assert!(db.status("collision").unwrap().is_none());
}

// Would fail if pending intentions or delivery identity lived only in memory.
#[tokio::test]
async fn restart_recovers_committed_work_and_stores_acceptance_receipt() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let dispatcher = dispatcher(channels());
    enqueue(&mut db, &dispatcher.channels);
    assert!(db.status("delivery-1").unwrap().unwrap().receipt.is_none());
    drop(db);
    let mut db = scratch.open();
    assert_eq!(db.resource("resource-1").unwrap().as_deref(), Some("new"));
    assert!(dispatcher.step(&mut db, 0).await.unwrap());
    drop(db);
    let mut db = scratch.open();
    let status = db.status("delivery-1").unwrap().unwrap();
    assert_eq!(status.state, "accepted");
    assert_eq!(status.receipt.as_deref(), Some("local:delivery-1:changed"));
    assert!(!dispatcher.step(&mut db, 500).await.unwrap());
}

// Would fail if retries ran early, changed their ID, or ignored the attempt budget.
#[tokio::test]
async fn transient_retries_use_stable_identity_exponential_backoff_and_attempt_bound() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let ids = Arc::new(Mutex::new(Vec::new()));
    let mut channels = Channels::default();
    let recorded = ids.clone();
    channels
        .register("custom", "notice/v1", move |d: Delivery<Notice>| {
            recorded.lock().unwrap().push(d.id.0);
            async { Outcome::Retryable }
        })
        .unwrap();
    let dispatcher = dispatcher(channels);
    enqueue(&mut db, &dispatcher.channels);
    assert!(dispatcher.step(&mut db, 0).await.unwrap());
    assert!(!dispatcher.step(&mut db, 9).await.unwrap());
    assert!(dispatcher.step(&mut db, 10).await.unwrap());
    assert!(!dispatcher.step(&mut db, 29).await.unwrap());
    assert!(dispatcher.step(&mut db, 30).await.unwrap());
    assert!(!dispatcher.step(&mut db, 500).await.unwrap());
    let status = db.status("delivery-1").unwrap().unwrap();
    assert_eq!((status.state.as_str(), status.attempts), ("failed", 3));
    assert!(status.receipt.is_none());
    assert_eq!(
        ids.lock().unwrap().as_slice(),
        ["delivery-1", "delivery-1", "delivery-1"]
    );
}

// Would fail if permanent errors entered the retry queue or invented a receipt.
#[tokio::test]
async fn permanent_failure_is_terminal_without_provider_receipt() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let mut channels = Channels::default();
    channels
        .register("custom", "notice/v1", |_: Delivery<Notice>| async {
            Outcome::Permanent
        })
        .unwrap();
    let dispatcher = dispatcher(channels);
    enqueue(&mut db, &dispatcher.channels);
    dispatcher.step(&mut db, 0).await.unwrap();
    assert!(!dispatcher.step(&mut db, 500).await.unwrap());
    let status = db.status("delivery-1").unwrap().unwrap();
    assert_eq!((status.state.as_str(), status.attempts), ("failed", 1));
    assert_eq!(status.last_outcome.as_deref(), Some("permanent"));
    assert!(status.receipt.is_none());
}

// Would fail if the registry accepted an incompatible payload, version, or duplicate name.
#[test]
fn payload_schema_and_named_registration_are_checked() {
    let mut channels = channels();
    assert!(
        channels
            .register("custom", "notice/v1", ordinary_custom_function)
            .is_err()
    );
    assert!(
        channels
            .intent("id", "missing", "notice/v1", &Notice { text: "x".into() })
            .is_err()
    );
    assert!(
        channels
            .intent("id", "custom", "notice/v2", &Notice { text: "x".into() })
            .is_err()
    );
    assert!(channels.intent("id", "custom", "notice/v1", &42).is_err());
    assert!(
        channels
            .intent(
                "id",
                "custom",
                "notice/v1",
                &Notice {
                    text: "x".repeat(65537)
                }
            )
            .is_err()
    );
}

async fn ambiguous_acceptance(deduplicate: bool) -> (usize, Vec<String>) {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let attempts = Arc::new(Mutex::new(Vec::<String>::new()));
    let effects = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let accepted = Arc::new(Notify::new());
    let gate = Arc::new(Notify::new());
    let mut channels = Channels::default();
    let (a, e, s, ack, block) = (
        attempts.clone(),
        effects.clone(),
        seen,
        accepted.clone(),
        gate,
    );
    channels
        .register("custom", "notice/v1", move |delivery: Delivery<Notice>| {
            let first = {
                let mut attempts = a.lock().unwrap();
                attempts.push(delivery.id.0.clone());
                attempts.len() == 1
            };
            let duplicate = !s.lock().unwrap().insert(delivery.id.0.clone());
            if !deduplicate || !duplicate {
                e.lock().unwrap().push(delivery.payload.text);
            }
            let (ack, block) = (ack.clone(), block.clone());
            async move {
                if first {
                    ack.notify_one();
                    block.notified().await;
                }
                Outcome::Accepted("receiver-accepted".into())
            }
        })
        .unwrap();
    let dispatcher = Arc::new(dispatcher(channels));
    enqueue(&mut db, &dispatcher.channels);
    let runner = dispatcher.clone();
    let task = tokio::spawn(async move { runner.step(&mut db, 0).await });
    accepted.notified().await;
    // The receiver applied its effect. The channel has not returned acknowledgment.
    let mut observer = scratch.open();
    let status = observer.status("delivery-1").unwrap().unwrap();
    assert_eq!(status.state, "leased");
    assert!(status.receipt.is_none());
    assert!(!dispatcher.step(&mut observer, 9).await.unwrap());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(observer);
    let mut restarted = scratch.open();
    assert!(dispatcher.step(&mut restarted, 10).await.unwrap());
    assert_eq!(
        restarted.status("delivery-1").unwrap().unwrap().state,
        "accepted"
    );
    let count = effects.lock().unwrap().len();
    let ids = attempts.lock().unwrap().clone();
    (count, ids)
}

// Would fail if crash recovery dropped an unacknowledged delivery instead of retrying.
#[tokio::test]
async fn receiver_acceptance_before_ack_can_duplicate_effects() {
    let (effects, ids) = ambiguous_acceptance(false).await;
    assert_eq!(effects, 2);
    assert_eq!(ids, ["delivery-1", "delivery-1"]);
}

// Demonstrates receiver-owned deduplication, not an outbox exactly-once guarantee.
#[tokio::test]
async fn receiver_deduplication_suppresses_duplicate_effect_with_same_delivery_id() {
    let (effects, ids) = ambiguous_acceptance(true).await;
    assert_eq!(effects, 1);
    assert_eq!(ids, ["delivery-1", "delivery-1"]);
}

// Would fail if a hanging callback held the worker forever or was classified as rejected.
#[tokio::test(start_paused = true)]
async fn callback_timeout_is_unknown_and_retryable_with_no_receipt() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let mut channels = Channels::default();
    channels
        .register("custom", "notice/v1", |_: Delivery<Notice>| async {
            std::future::pending::<Outcome>().await
        })
        .unwrap();
    let dispatcher = dispatcher(channels);
    enqueue(&mut db, &dispatcher.channels);
    dispatcher.step(&mut db, 0).await.unwrap();
    let status = db.status("delivery-1").unwrap().unwrap();
    assert_eq!(status.last_outcome.as_deref(), Some("unknown"));
    assert_eq!(status.state, "pending");
    assert_eq!(status.next_at, 10);
    assert!(status.receipt.is_none());
}

// Would fail if persisted bytes bypassed the active channel's versioned decoder.
#[tokio::test]
async fn channel_version_change_fails_old_payload_without_invoking_callback() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    enqueue(&mut db, &channels());
    let invoked = Arc::new(AtomicUsize::new(0));
    let counted = invoked.clone();
    let mut replacement = Channels::default();
    replacement
        .register("custom", "notice/v2", move |_: Delivery<Notice>| {
            counted.fetch_add(1, Ordering::Relaxed);
            async { Outcome::Accepted("wrong".into()) }
        })
        .unwrap();
    dispatcher(replacement).step(&mut db, 0).await.unwrap();
    assert_eq!(invoked.load(Ordering::Relaxed), 0);
    assert_eq!(db.status("delivery-1").unwrap().unwrap().state, "failed");
}

// Would fail if an expired worker could overwrite the newer worker's accepted result.
#[test]
fn lease_generation_rejects_stale_acknowledgment() {
    let scratch = Scratch::new();
    let mut first = scratch.open();
    enqueue(&mut first, &channels());
    let old = first.claim(0, 10, 3).unwrap().unwrap();
    let mut second = scratch.open();
    assert!(second.claim(9, 10, 3).unwrap().is_none());
    let fresh = second.claim(10, 10, 3).unwrap().unwrap();
    second
        .finish(&fresh, Outcome::Accepted("fresh".into()), 20, 3)
        .unwrap();
    assert!(
        first
            .finish(&old, Outcome::Accepted("stale".into()), 20, 3)
            .is_err()
    );
    assert_eq!(
        second
            .status("delivery-1")
            .unwrap()
            .unwrap()
            .receipt
            .as_deref(),
        Some("fresh")
    );
}

// Would fail if every worker death reset the budget and caused unlimited attempts.
#[test]
fn repeated_worker_loss_exhausts_attempt_budget_without_fabricating_receipt() {
    let scratch = Scratch::new();
    let mut db = scratch.open();
    enqueue(&mut db, &channels());
    for now in [0, 10, 20] {
        assert!(db.claim(now, 10, 3).unwrap().is_some());
        drop(db);
        db = scratch.open();
    }
    assert!(db.claim(30, 10, 3).unwrap().is_none());
    let status = db.status("delivery-1").unwrap().unwrap();
    assert_eq!((status.state.as_str(), status.attempts), ("failed", 3));
    assert_eq!(status.last_outcome.as_deref(), Some("unknown"));
    assert!(status.receipt.is_none());
}

// Would fail if dispatch knew only one channel or erased payload contracts into unchecked bytes.
#[tokio::test]
async fn same_dispatcher_routes_two_named_ordinary_functions_with_different_payloads() {
    async fn number_channel(d: Delivery<u32>) -> Outcome {
        Outcome::Accepted(format!("number:{}", d.payload))
    }
    let scratch = Scratch::new();
    let mut db = scratch.open();
    let mut channels = channels();
    channels
        .register("numbers", "number/v1", number_channel)
        .unwrap();
    let number = channels
        .intent("delivery-2", "numbers", "number/v1", &7_u32)
        .unwrap();
    enqueue(&mut db, &channels);
    db.transition("resource-2", "new", &[number]).unwrap();
    let dispatcher = dispatcher(channels);
    dispatcher.step(&mut db, 0).await.unwrap();
    dispatcher.step(&mut db, 0).await.unwrap();
    assert_eq!(
        db.status("delivery-1").unwrap().unwrap().receipt.as_deref(),
        Some("local:delivery-1:changed")
    );
    assert_eq!(
        db.status("delivery-2").unwrap().unwrap().receipt.as_deref(),
        Some("number:7")
    );
}
