use super::support::*;
use rom::*;
use std::sync::{Arc, Mutex, OnceLock};

struct Context {
    storage: Arc<dyn Storage>,
    calls: Mutex<Vec<&'static str>>,
}
static CONTEXT: OnceLock<Context> = OnceLock::new();
fn reenter(stage: &'static str) -> bool {
    let context = CONTEXT.get().unwrap();
    context.calls.lock().unwrap().push(stage);
    let row = context
        .storage
        .load(&Key {
            kind: Ledger::KIND.into(),
            id: "permission".into(),
        })
        .unwrap()
        .unwrap();
    Ledger::decode(row.value.unwrap()).unwrap().total == 1
}
fn query_policy(_: &Actor, field: &str) -> bool {
    reenter("query") && field == "value"
}
fn row_policy(actor: &Actor, access: Access, value: &Record) -> bool {
    !matches!(access, Access::Read) || (reenter("row") && actor.subject == value.owner)
}
fn uniform(actor: &Actor) -> bool {
    reenter("read") && actor.subject == "reader"
}

#[tokio::test]
async fn query_policy_child() {
    let Some(root) = std::env::var_os("ROM_STUDIO_QUERY_ROOT") else {
        return;
    };
    let redb = std::env::var("ROM_STUDIO_QUERY_BACKEND").unwrap() == "redb";
    let eligible = std::env::var("ROM_STUDIO_QUERY_MODE").unwrap() == "eligible";
    let db = Database::open(redb, &std::path::PathBuf::from(root).join("database"));
    let storage = db.storage();
    assert!(
        CONTEXT
            .set(Context {
                storage: storage.clone(),
                calls: Mutex::new(vec![])
            })
            .is_ok()
    );
    let mut definition = Record::definition()
        .policy(row_policy)
        .allow_all_fields()
        .query_policy(query_policy);
    if eligible {
        definition = definition.read_policy(uniform);
    }
    let runtime = Runtime::builder()
        .resource(definition)
        .resource(
            Ledger::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let actor = Actor::trusted("tests", "reader");
    runtime
        .execute(
            &actor,
            Command::create(
                "permission",
                Ledger {
                    owner: "reader".into(),
                    total: 1,
                },
            )
            .idempotency("permission"),
        )
        .await
        .unwrap();
    for index in 0..100 {
        runtime
            .execute(
                &actor,
                Command::create(
                    &format!("row-{index:03}"),
                    Record {
                        owner: "reader".into(),
                        value: u64::from(index == 5),
                    },
                )
                .idempotency(&format!("seed-{index}")),
            )
            .await
            .unwrap();
    }
    let selection = if eligible {
        SelectionMode::UniformReadAndFields
    } else {
        SelectionMode::ReferenceOnly
    };
    let native = storage
        .query_read(
            &StorageQuery {
                descriptor: Record::descriptor(),
                spec: QuerySpec::equal("value", json!(1)),
                semantics: QUERY_SEMANTICS_VERSION,
                selection,
            },
            QueryBounds {
                max_rows: 1024,
                max_bytes: 1024 * 1024,
            },
        )
        .unwrap();
    assert_eq!(
        matches!(native, QueryRead::NativeCandidates { .. }),
        eligible && !redb
    );
    CONTEXT.get().unwrap().calls.lock().unwrap().clear();
    let rows = runtime
        .query(&actor, &Record::value_field().equals(1))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "row-005");
    let calls = CONTEXT.get().unwrap().calls.lock().unwrap().clone();
    assert_eq!(calls[0], "query");
    assert!(calls.contains(&(if eligible { "read" } else { "row" })));
    CONTEXT.get().unwrap().calls.lock().unwrap().clear();
    assert!(matches!(
        runtime
            .query(&actor, &Record::owner_field().equals("reader".into()))
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(*CONTEXT.get().unwrap().calls.lock().unwrap(), ["query"]);
    shutdown(&runtime).await;
}

#[test]
fn reentrant_native_storage_policy_completes_on_reference_and_eligible_query_paths() {
    for redb in [false, true] {
        for mode in ["reference", "eligible"] {
            let scratch = Scratch::new();
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "studio_resilience::query::query_policy_child",
                    "--nocapture",
                ])
                .env("ROM_STUDIO_QUERY_ROOT", &scratch.0)
                .env(
                    "ROM_STUDIO_QUERY_BACKEND",
                    if redb { "redb" } else { "sqlite" },
                )
                .env("ROM_STUDIO_QUERY_MODE", mode)
                .stdin(std::process::Stdio::null());
            let mut child = crate::child_process::Process::spawn(command);
            assert!(child.wait().success(), "{redb}/{mode}");
        }
    }
}
