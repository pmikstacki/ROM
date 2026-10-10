//! Real identity query profiling; timings are informational, never pass limits.
use super::*;
use rom::{ActorGate, AuthorizationRead, Key, QuerySpec, Row, Storage};
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Metrics {
    calls: AtomicUsize,
    reads: AtomicUsize,
    gate_ns: AtomicU64,
    read_ns: AtomicU64,
}
fn nanos(start: Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap()
}
struct TimedRead<'a> {
    inner: &'a mut dyn AuthorizationRead,
    metrics: &'a Metrics,
}
impl AuthorizationRead for TimedRead<'_> {
    fn load(&mut self, key: &Key) -> rom::Result<Option<Row>> {
        self.metrics.reads.fetch_add(1, Ordering::SeqCst);
        let start = Instant::now();
        let result = self.inner.load(key);
        self.metrics
            .read_ns
            .fetch_add(nanos(start), Ordering::SeqCst);
        result
    }
}
struct TimedGate {
    inner: IdentityGate,
    metrics: Arc<Metrics>,
}
impl ActorGate for TimedGate {
    fn check(&self, actor: &Actor, read: &mut dyn AuthorizationRead) -> rom::Result<()> {
        if actor.host_stamp().is_none() {
            return self.inner.check(actor, read);
        }
        self.metrics.calls.fetch_add(1, Ordering::SeqCst);
        let start = Instant::now();
        let result = self.inner.check(
            actor,
            &mut TimedRead {
                inner: read,
                metrics: &self.metrics,
            },
        );
        self.metrics
            .gate_ns
            .fetch_add(nanos(start), Ordering::SeqCst);
        result
    }
}
impl Metrics {
    fn sample(&self) -> [u64; 4] {
        [
            self.calls.load(Ordering::SeqCst) as u64,
            self.reads.load(Ordering::SeqCst) as u64,
            self.gate_ns.load(Ordering::SeqCst),
            self.read_ns.load(Ordering::SeqCst),
        ]
    }
}
struct Fixture {
    runtime: Runtime,
    clock: Arc<Time>,
    actor: Actor,
    metrics: Arc<Metrics>,
    rows: Vec<Row>,
}
async fn fixture() -> Fixture {
    let clock = Arc::new(Time(AtomicU64::new(NOW)));
    let metrics = Arc::new(Metrics::default());
    let gate = TimedGate {
        inner: IdentityGate::default()
            .allow_host("host", PrincipalKind::Embedded, "bootstrap")
            .unwrap(),
        metrics: metrics.clone(),
    };
    let storage = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .clock(clock.clone())
        .actor_gate(Arc::new(gate))
        .resource(User::definition().policy(admin_policy).allow_all_fields())
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| a == &admin())
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| a == &admin())
                .allow_all_fields(),
        )
        .resource(
            Document::definition()
                .policy(|a, _, _| a == &admin() || a.subject == "subject")
                .allow_all_fields(),
        )
        .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    runtime
        .execute(
            &admin(),
            Command::create(
                "user",
                User {
                    enabled: true,
                    display_name: "Profile".into(),
                },
            )
            .idempotency("user"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &admin(),
            Command::create("provider", provider()).idempotency("provider"),
        )
        .await
        .unwrap();
    let link = link_key("provider", PrincipalKind::Human, "subject");
    runtime
        .execute(
            &admin(),
            Command::create(
                &link,
                IdentityLink {
                    authority: "provider".into(),
                    principal_kind: "human".into(),
                    subject: "subject".into(),
                    user_id: "user".into(),
                    enabled: true,
                },
            )
            .idempotency("link"),
        )
        .await
        .unwrap();
    for index in 0..50 {
        let id = format!("document-{index:02}");
        runtime
            .execute(
                &admin(),
                Command::create(
                    &id,
                    Document {
                        body: "protected".into(),
                    },
                )
                .idempotency(&id),
            )
            .await
            .unwrap();
    }
    let activation = ProviderActivation::read(&runtime, &admin(), "provider")
        .await
        .unwrap();
    let actor = proof(&activation, "subject").bind(&runtime).await.unwrap();
    let rows = [
        Key {
            kind: IdentityProvider::KIND.into(),
            id: "provider".into(),
        },
        Key {
            kind: IdentityLink::KIND.into(),
            id: link,
        },
        Key {
            kind: User::KIND.into(),
            id: "user".into(),
        },
    ]
    .iter()
    .map(|key| storage.load(key).unwrap().unwrap())
    .collect();
    Fixture {
        runtime,
        clock,
        actor,
        metrics,
        rows,
    }
}
struct ReplayRead<'a>(&'a [Row]);
impl AuthorizationRead for ReplayRead<'_> {
    fn load(&mut self, key: &Key) -> rom::Result<Option<Row>> {
        Ok(self.0.iter().find(|row| row.key == *key).cloned())
    }
}

#[tokio::test]
async fn identity_gate_query_profile() {
    // Existing real RSA fixture setup is separate from measured query lifetime.
    let f = fixture().await;
    let measured = tokio::time::timeout(Duration::from_secs(5), async {
        let mut samples = vec![];
        for _ in 0..10 {
            let before = f.metrics.sample();
            let start = Instant::now();
            let rows = f
                .runtime
                .query_spec_projected(&f.actor, Document::KIND, QuerySpec::all().limit(50))
                .await;
            let total_ns = nanos(start);
            let after = f.metrics.sample();
            samples.push((rows, total_ns, before, after));
        }
        samples
    })
    .await;
    let replay_metrics = Arc::new(Metrics::default());
    let replay_gate = TimedGate {
        inner: IdentityGate::default(),
        metrics: replay_metrics.clone(),
    };
    let replay_start = Instant::now();
    let replay_result = (|| -> rom::Result<()> {
        for _ in 0..52 {
            replay_gate.check(&f.actor, &mut ReplayRead(&f.rows))?;
        }
        Ok(())
    })();
    let replay_ns = nanos(replay_start);
    let replay = replay_metrics.sample();
    let decode_result = (|| -> rom::Result<Vec<u64>> {
        let mut decode_ns = vec![];
        for (index, row) in f.rows.iter().enumerate() {
            let start = Instant::now();
            for _ in 0..52 {
                let value = row.value.clone().unwrap();
                match index {
                    0 => {
                        IdentityProvider::decode(value)?;
                    }
                    1 => {
                        IdentityLink::decode(value)?;
                    }
                    2 => {
                        User::decode(value)?;
                    }
                    _ => unreachable!(),
                }
            }
            decode_ns.push(nanos(start));
        }
        Ok(decode_ns)
    })();
    let stamp_start = Instant::now();
    let mut parsed_stamps = vec![];
    for _ in 0..52 {
        parsed_stamps.push(serde_json::from_str::<rom::Value>(
            f.actor.host_stamp().unwrap(),
        ));
    }
    let stamp_value_parse_ns = nanos(stamp_start);
    f.clock
        .0
        .store(f.actor.valid_until().unwrap(), Ordering::SeqCst);
    let before_expiry = f.metrics.sample();
    let expired = f
        .runtime
        .query_spec_projected(&f.actor, Document::KIND, QuerySpec::all().limit(50))
        .await;
    let after_expiry = f.metrics.sample();
    let shutdown = tokio::time::timeout(Duration::from_secs(5), f.runtime.shutdown()).await;
    // Drain the real Runtime before evaluating recorded query/profile assertions.
    shutdown.unwrap().unwrap();
    assert_eq!(f.runtime.status().unwrap().owned_work, 0);
    replay_result.unwrap();
    let decode_ns = decode_result.unwrap();
    for parsed in parsed_stamps {
        assert_eq!(parsed.unwrap().get("user_id"), Some(&rom::json!("user")));
    }
    let samples: Vec<_> = measured.unwrap().into_iter().map(|(rows, total_ns, before, after)| {
        let rows = rows.unwrap();
        assert_eq!(rows.len(), 50);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.key.id, format!("document-{index:02}"));
            assert_eq!(row.revision, 1);
            assert_eq!(row.value.as_ref().unwrap().get("body"), Some(&rom::json!("protected")));
        }
        assert_eq!(after[0] - before[0], 52);
        assert_eq!(after[1] - before[1], 156);
        rom::json!({"query_ns":total_ns,"gate_calls":52,"identity_reads":156,
            "gate_inclusive_ns":after[2]-before[2],"authorization_read_inclusive_ns":after[3]-before[3]})
    }).collect();
    assert_eq!(expired, Err(Error::Denied));
    assert_eq!(before_expiry, after_expiry);
    assert_eq!(&replay[..2], &[52, 156]);
    println!(
        "{}",
        rom::json!({"identity_gate_query_profile": {
            "queries":samples,"compute_only_replay_ns":replay_ns,
            "compute_only_gate_inclusive_ns":replay[2],"compute_only_read_inclusive_ns":replay[3],
            "public_resource_clone_decode_ns":decode_ns,"decode_repetitions_per_resource":52,
            "stamp_value_parse_ns":stamp_value_parse_ns,"stamp_parse_scope":"Value parse, not private typed Stamp",
            "read_scope":"AuthorizationRead includes native read and runtime byte accounting; spans overlap",
            "setup_in_measured_time":false,"timing_threshold":false
        }})
    );
}
