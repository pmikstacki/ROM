# Independent maintained-query review

Date: 2026-10-03. Reviewed frozen implementation
`0641993ac732f2a434f06031f4c81449c96b403c`, based on `3780860`, in the
`query-maintained` worktree. The tracked checkout was clean. Workspace Cargo.lock
SHA-256: `35a128d90a9c31bc03f0e073cdba52e819bffb565bfb05cfd34a8261bf112e7d`.

**Result: no outstanding P1/P2 findings within the maintained query slice.**
This delivers exact scalar ranges, multiple sort fields and client-selected moving
anchors through the existing bounded snapshot evaluator. It introduces no
optimized adapter, index, storage format or public physical-planning API.

## Source and contract assessment

The review covered the approved implementation plan and query-semantics OpenSpec,
private `query_eval.rs`, public typed/wire query definitions, registration grants,
HTTP deserialization, shared adapter tests, and updated query/CLI documentation.

Integer comparison stays in its declared signed/unsigned representation; there is
no unsigned-to-float or signed-storage shortcut. Finite floats compare numerically,
including signed-zero equality. Canonical operands use the actual Field codec.
Missing/null/value ordering and equality/range distinctions match the documented
contract. Structural collection equality remains available, while normalization
rejects collection range/order operations. Typed helpers intentionally defer some
operator/shape checks to normalization; docs make this explicit.

Predicate and sort grants run before storage access. Sort permission is separately
default-denied; the explicit whole-field grant enables it. Every row-readable
candidate must also grant read access to each ordered field before predicates,
anchors or page limits can affect selection. Row-hidden candidates supply no sort
keys. The same selector serves typed, projected and live queries. Existing actor
rechecks and complete-result field checks remain outside and around that selector.

Anchors bind normalized predicates/order, kind/schema/version and exact key count;
canonical values and size are validated even against empty data. They remain
client input, not credentials. A caller can choose a different valid boundary but
cannot use it to bypass current query/sort/read authority. The projected anchor
helper explicitly does not certify supplied-view provenance or current row/field
visibility; actual evaluation checks those grants. Page limits are intentionally
not bound. `and_where` rejects nested continuation/order/limit instead of silently
dropping that configuration, and preserves the outer query's configuration.

Existing ID-only defaults, equality and `after_id` remain supported. Full snapshot
row/byte and duplicate checks precede limiting. Small pages therefore do not turn
a bounded scan into an unbounded collection facility. Field-sorted queries perform
all required disclosure checks before limiting.

## Independently executed verification

Native `rom-dev`, Rust 1.99.0, target
`/var/tmp/rom-query-maintained-review-target`, two jobs, dev/test debug information
disabled, offline dependency use. At the frozen SHA this reviewer ran:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-query-maintained-review-target
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_NET_OFFLINE=true
cargo test --locked -p rom-storage-conformance --test query_planning
cargo test --locked -p rom-consumer --test query_all --test query_contract
cargo test --locked -p rom-http --test loopback ranges_sort_and_client_anchor_share_query_and_live_wire_paths
```

All three commands passed **12 tests**: six new shared SQLite/redb cases, five
existing consumer cases, and one actual HTTP query/live/anchor test. This is
independent evidence; implementer Clippy/full HTTP results and the coordinator's
full workspace integration run are separate checks, not credited to this reviewer.

An independent external consumer fixture passed **928 comparison vectors** against
actual SQLite and redb: per adapter, 336 ordering/anchor cases and 128 comparison
cases. It covers all pairs of four distinct ordered fields in both directions,
each observed boundary with a two-row page, and all six comparison operators.
Data includes unsigned/signed extremes, adjacent large unsigned values, finite
floats with signed zero, Unicode forms, embedded NUL, and optional nullable values.
Null range operands also reject; these negative cases are not included in the
928 accepted-comparison count.

Additional assertions require denial for a forged valid anchor under missing
sort/read grants, denial even when a predicate would exclude every row containing
a protected sort field, and denial of buffered live disclosure after actor
revocation. Nested configuration rejects while outer configuration survives.

The standalone fixture has its own dependency resolution and is not a replacement
for the locked workspace tests above. Its source, manifest template and execution
instructions are included below. The original local fixture/lockfile are retained
in `.worktrees/post-mvp-review/review-query`. The original standalone lockfile
SHA-256 is `32a3879356c0266c98c59a61cd044f4fa4283757b934db7858bff49015c66256`;
formatted fixture source SHA-256 is
`f36930cd16aaa3f660355e3441eda202a00849738c3538d7a55abf69898bd567`.

## Optimization assessment

The initial WIP performed unnecessary second sorting and complete selection for
ID-only pages. This was raised during review. The final code preserves a fast path:
after full snapshot/duplicate validation, it skips earlier IDs, stops selecting
when the requested page is full, and omits the second sort when no fields are
ordered. An independent counter observes **three row-read policy calls** for a
one-row projected page from six rows on each adapter, including disclosure checks;
it does not evaluate every row through that policy. This is a narrow work-count
observation, not a latency/throughput benchmark.

Field sorting still materializes bounded candidates/keys and sorts in memory.
Snapshot acquisition and encoded-size validation still cover the entire kind;
small limits cannot remove those costs. No allocation counts, peak RSS, production
latency or index speedup were measured for this maintained change. Optimized
adapter planning remains a separate prototype/contract question.

The documentation was corrected to use the maintained demo's `quantity`/`code`
fields, and to explain helper authority and runtime operator validation. No
correctness or authorization defect remained after final inspection and the
independent fixtures. This conclusion does not imply exhaustive fuzzing, retained
transactional snapshots, or universal provider/codec performance.

## Reproducing the independent fixture

Place a disposable independent workspace directly beneath a checkout of the
reviewed ROM SHA; the relative path dependencies below assume that placement.
Save the Rust block as `tests/adversarial.rs`, then run `cargo test -- --nocapture`.
Use a separate target and two jobs as above. This reconstructs the fixture; its
first dependency resolution is not a claim to reproduce the original lockfile
byte-for-byte. For release validation, also run the locked workspace commands.

```toml
[package]
name = "independent-maintained-query-review"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
rom = { path = "../crates/rom" }
rom-sqlite = { path = "../crates/rom-sqlite" }
rom-redb = { path = "../crates/rom-redb" }
tokio = { version = "=1.53.1", features = ["rt-multi-thread", "macros"] }
[profile.dev]
debug = 0
[profile.test]
debug = 0
```

```rust
use rom::*;
use std::{cmp::Ordering as Cmp, sync::Arc};
#[derive(Clone, Debug, Resource)]
#[resource(name = "review-values")]
struct Item {
    n: u64,
    signed: i64,
    score: FiniteF64,
    note: Presence<Option<String>>,
    visible: bool,
}
fn actor(name: &str) -> Actor {
    Actor::trusted("review", name)
}
static READ_CHECKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn definition() -> Definition<Item> {
    Item::definition()
        .policy(|a, access, r| {
            if matches!(access, Access::Read) {
                READ_CHECKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            matches!(access, Access::Write) || r.visible || a.subject == "owner"
        })
        .allow_all_fields()
        .field_policy(|a, access, f, _| {
            matches!(access, Access::Write) || a.subject != "hidden-field" || f != "n"
        })
        .sort_policy(|a, _| a.subject != "no-sort")
}
fn values() -> Vec<Item> {
    [
        (0, i64::MIN, -0.0, Presence::Missing),
        (u64::MAX, i64::MAX, 0.0, Presence::Value(None)),
        (u64::MAX - 1, 0, 1.5, Presence::Value(Some("é".into()))),
        (
            (1 << 53) + 1,
            -1,
            -1.5,
            Presence::Value(Some("e\u{301}".into())),
        ),
        (1 << 53, 1, 0.0, Presence::Value(Some("a\0b".into()))),
        (1 << 63, 0, 1.5, Presence::Missing),
    ]
    .into_iter()
    .map(|(n, signed, score, note)| Item {
        n,
        signed,
        score: FiniteF64::new(score).unwrap(),
        note,
        visible: true,
    })
    .collect()
}
fn note(v: &Presence<Option<String>>) -> (u8, &str) {
    match v {
        Presence::Missing => (0, ""),
        Presence::Value(None) => (1, ""),
        Presence::Value(Some(s)) => (2, s),
    }
}
fn cmp(a: &Item, b: &Item, field: &str) -> Cmp {
    match field {
        "n" => a.n.cmp(&b.n),
        "signed" => a.signed.cmp(&b.signed),
        "note" => note(&a.note).cmp(&note(&b.note)),
        _ => a.score.get().partial_cmp(&b.score.get()).unwrap(),
    }
}
#[tokio::test]
async fn independent_edges_and_authority() {
    for redb in [false, true] {
        let path = std::path::PathBuf::from(format!(
            "/var/tmp/rom-query-review-{}-{redb}",
            std::process::id()
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = Runtime::builder()
            .resource(definition())
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let data = values();
        for (i, item) in data.iter().enumerate() {
            runtime
                .execute(
                    &actor("owner"),
                    Command::create(&format!("{i}"), item.clone()).idempotency(&format!("seed{i}")),
                )
                .await
                .unwrap();
        }
        let mut vectors = 0;
        for a in ["n", "signed", "note", "score"] {
            for b in ["n", "signed", "note", "score"] {
                if a == b {
                    continue;
                }
                for da in [Direction::Asc, Direction::Desc] {
                    for db in [Direction::Asc, Direction::Desc] {
                        let spec = QuerySpec::all().order_by(a, da).order_by(b, db);
                        let mut expected: Vec<_> = (0..data.len()).collect();
                        expected.sort_by(|i, j| {
                            let x = cmp(&data[*i], &data[*j], a);
                            let y = cmp(&data[*i], &data[*j], b);
                            (if da == Direction::Desc {
                                x.reverse()
                            } else {
                                x
                            })
                            .then(if db == Direction::Desc {
                                y.reverse()
                            } else {
                                y
                            })
                            .then(i.cmp(j))
                        });
                        let all = runtime
                            .query_spec_projected(&actor("owner"), Item::KIND, spec.clone())
                            .await
                            .unwrap();
                        assert_eq!(
                            all.iter().map(|v| v.key.id.clone()).collect::<Vec<_>>(),
                            expected.iter().map(|i| i.to_string()).collect::<Vec<_>>()
                        );
                        vectors += 1;
                        for (position, view) in all.iter().enumerate() {
                            let anchor = runtime
                                .query_anchor(&actor("owner"), &spec, view)
                                .await
                                .unwrap();
                            let after = runtime
                                .query_spec_projected(
                                    &actor("owner"),
                                    Item::KIND,
                                    spec.clone().after(anchor).limit(2),
                                )
                                .await
                                .unwrap();
                            assert_eq!(
                                after.iter().map(|v| v.key.id.clone()).collect::<Vec<_>>(),
                                expected
                                    .iter()
                                    .skip(position + 1)
                                    .take(2)
                                    .map(|i| i.to_string())
                                    .collect::<Vec<_>>()
                            );
                            vectors += 1;
                        }
                    }
                }
            }
        }
        let mut range_vectors = 0;
        for field in ["n", "signed", "score", "note"] {
            for operand in &data {
                let encoded = operand.encode();
                let Some(value) = encoded.get(field) else {
                    continue;
                };
                for op in [
                    CompareOp::Eq,
                    CompareOp::Ne,
                    CompareOp::Lt,
                    CompareOp::Le,
                    CompareOp::Gt,
                    CompareOp::Ge,
                ] {
                    let spec = QuerySpec::all().compare(field, op, value.clone());
                    if value.is_null() && !matches!(op, CompareOp::Eq | CompareOp::Ne) {
                        assert!(
                            runtime
                                .query_spec_projected(&actor("owner"), Item::KIND, spec)
                                .await
                                .is_err()
                        );
                        continue;
                    }
                    let expected: Vec<_> = data
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| {
                            let encoded = r.encode();
                            let Some(actual) = encoded.get(field) else {
                                return false;
                            };
                            if actual.is_null() && !matches!(op, CompareOp::Eq | CompareOp::Ne) {
                                return false;
                            }
                            let c = cmp(r, operand, field);
                            match op {
                                CompareOp::Eq => c == Cmp::Equal,
                                CompareOp::Ne => c != Cmp::Equal,
                                CompareOp::Lt => c == Cmp::Less,
                                CompareOp::Le => c != Cmp::Greater,
                                CompareOp::Gt => c == Cmp::Greater,
                                CompareOp::Ge => c != Cmp::Less,
                            }
                        })
                        .map(|(i, _)| i.to_string())
                        .collect();
                    let actual = runtime
                        .query_spec_projected(&actor("owner"), Item::KIND, spec.clone())
                        .await
                        .unwrap();
                    assert_eq!(
                        actual.iter().map(|r| r.key.id.clone()).collect::<Vec<_>>(),
                        expected,
                        "{spec:?}"
                    );
                    range_vectors += 1;
                }
            }
        }
        println!("backend={redb} independent_comparison_vectors={range_vectors}");
        READ_CHECKS.store(0, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            runtime
                .query_spec_projected(&actor("owner"), Item::KIND, QuerySpec::all().limit(1))
                .await
                .unwrap()
                .len(),
            1
        );
        let checks = READ_CHECKS.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            checks < data.len(),
            "ID-only limit evaluated every row: {checks}"
        );
        println!("backend={redb} ID_only_one_row_read_policy_checks={checks}");
        let query = Query::<Item>::all()
            .order_by(Item::n_field(), Direction::Asc)
            .limit(1);
        let views = runtime
            .query_spec_projected(&actor("owner"), Item::KIND, query.spec().clone())
            .await
            .unwrap();
        let anchor = runtime
            .query_anchor(&actor("owner"), query.spec(), &views[0])
            .await
            .unwrap();
        for name in ["hidden-field", "no-sort"] {
            assert_eq!(
                runtime
                    .query_spec_projected(
                        &actor(name),
                        Item::KIND,
                        query.spec().clone().after(anchor.clone())
                    )
                    .await
                    .unwrap_err(),
                Error::Denied
            );
        }
        // Excluding every row with a predicate/anchor does not bypass sort-field disclosure.
        assert_eq!(
            runtime
                .query_spec_projected(
                    &actor("hidden-field"),
                    Item::KIND,
                    query.spec().clone().compare("n", CompareOp::Lt, json!(0))
                )
                .await
                .unwrap_err(),
            Error::Denied
        );
        let mut live = runtime
            .live_spec_projected(&actor("owner"), Item::KIND, query.spec().clone())
            .await
            .unwrap();
        runtime.revoke(&actor("owner"));
        assert_eq!(live.changed().await.unwrap_err(), Error::Denied);
        // Nested configuration is rejected, while outer configuration is retained.
        assert!(query.clone().and_where(Query::all().after_id("1")).is_err());
        assert!(
            query
                .clone()
                .and_where(Query::all().order_by(Item::n_field(), Direction::Desc))
                .is_err()
        );
        let combined = query
            .clone()
            .and_where(Item::n_field().at_least(0))
            .unwrap();
        assert_eq!(combined.spec().limit, Some(1));
        assert_eq!(combined.spec().order, query.spec().order);
        println!("backend={redb} independent_order_anchor_vectors={vectors}");
        runtime.shutdown().await.unwrap();
    }
}

```
