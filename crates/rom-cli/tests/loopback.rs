mod common;
use common::*;
use serde_json::{Value, json};
use std::sync::Arc;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_kinds_use_same_binary_protocol_on_both_adapters() {
    for redb in [false, true] {
        let dir = Directory::new();
        let storage: Arc<dyn rom::Storage> = if redb {
            Arc::new(rom_redb::Redb::open(dir.0.join("db")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(dir.0.join("db")).unwrap())
        };
        let runtime = rom_demo::build(storage, Default::default()).unwrap();
        rom_demo::bootstrap(&runtime).await.unwrap();
        let http =
            rom_http::Http::new(runtime.clone(), rom_demo::resolver(), Default::default()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            http.serve(listener, async {
                let _ = stop_rx.await;
            })
            .await
            .unwrap()
        });
        let auth = dir.file("auth", "Demo local\n");
        let discovered = json(&run(&endpoint, Some(&auth), &["discover"], "").await);
        assert_eq!(
            discovered["resources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["kind"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["checkouts", "inventory", "reservation-stock", "tasks"]
        );
        assert_eq!(
            json(&run(&endpoint, Some(&auth), &["discover", "tasks"], "").await)["actions"],
            json!(["complete"])
        );
        assert_eq!(
            run(&endpoint, Some(&auth), &["discover", "settings"], "")
                .await
                .status
                .code(),
            Some(3)
        );
        let create = [
            "create",
            "tasks",
            "one",
            "--idempotency",
            "one-create",
            "--input-file",
            "-",
        ];
        let value = json(
            &run(
                &endpoint,
                Some(&auth),
                &create,
                r#"{"title":"first","done":false}"#,
            )
            .await,
        );
        assert_eq!(value["revision"], 1);
        assert_eq!(value["value"]["done"], false);
        let inv = json(
            &run(
                &endpoint,
                Some(&auth),
                &[
                    "create",
                    "inventory",
                    "bin",
                    "--idempotency",
                    "bin-create",
                    "--input-file",
                    "-",
                ],
                r#"{"code":" bolt-7 ","quantity":0}"#,
            )
            .await,
        );
        assert_eq!(inv["value"], json!({"code":"BOLT-7","quantity":0}));
        let patch = [
            "patch",
            "tasks",
            "one",
            "--expected",
            "1",
            "--idempotency",
            "one-patch",
            "--input-file",
            "-",
        ];
        let body = r#"{"title":{"op":"set","value":"next"}}"#;
        let patched = json(&run(&endpoint, Some(&auth), &patch, body).await);
        assert_eq!(patched["revision"], 2);
        assert_eq!(
            json(&run(&endpoint, Some(&auth), &patch, body).await),
            patched
        );
        let mismatch = run(
            &endpoint,
            Some(&auth),
            &patch,
            r#"{"title":{"op":"set","value":"changed"}}"#,
        )
        .await;
        assert_eq!(mismatch.status.code(), Some(5));
        let conflict = run(
            &endpoint,
            Some(&auth),
            &[
                "delete",
                "tasks",
                "one",
                "--expected",
                "1",
                "--idempotency",
                "delete-conflict",
            ],
            "",
        )
        .await;
        assert_eq!(conflict.status.code(), Some(5));
        let action = json(
            &run(
                &endpoint,
                Some(&auth),
                &[
                    "action",
                    "tasks",
                    "one",
                    "complete",
                    "--expected",
                    "2",
                    "--idempotency",
                    "complete",
                    "--input-file",
                    "-",
                ],
                "null",
            )
            .await,
        );
        assert_eq!(action["value"]["done"], true);
        let rows = json(
            &run(
                &endpoint,
                Some(&auth),
                &["query", "tasks", "--query-file", "-"],
                r#"{"filters":[{"field":"done","value":true}],"limit":10}"#,
            )
            .await,
        );
        assert_eq!(rows.as_array().unwrap().len(), 1);
        let head = json(&run(&endpoint, Some(&auth), &["journal-head", "tasks"], "").await);
        let cursor = dir.file("cursor", serde_json::to_vec(&head).unwrap());
        let batch = json(
            &run(
                &endpoint,
                Some(&auth),
                &["journal", "tasks", "--after-file", &cursor],
                "",
            )
            .await,
        );
        assert_eq!(batch["events"], json!([]));
        let denied = run(&endpoint, None, &["read", "tasks", "one"], "").await;
        assert_eq!(denied.status.code(), Some(3));
        let invalid = run(
            &endpoint,
            Some(&auth),
            &[
                "create",
                "inventory",
                "bad",
                "--idempotency",
                "bad",
                "--input-file",
                "-",
            ],
            r#"{"code":"?","quantity":1}"#,
        )
        .await;
        assert_eq!(invalid.status.code(), Some(5));
        let read: Value = json(&run(&endpoint, Some(&auth), &["read", "tasks", "one"], "").await);
        assert_eq!(read["revision"], 3);
        let replaced = json(
            &run(
                &endpoint,
                Some(&auth),
                &[
                    "replace",
                    "tasks",
                    "one",
                    "--expected",
                    "3",
                    "--idempotency",
                    "replace-one",
                    "--input-file",
                    "-",
                ],
                r#"{"title":"replaced","done":false}"#,
            )
            .await,
        );
        assert_eq!(replaced["revision"], 4);
        let exact = r#"{"kind":"tasks","id":"one","expected":4,"idempotency":"raw-patch","operation":{"type":"patch","input":{"done":{"op":"set","value":true}}}}"#;
        let invoked = json(
            &run(
                &endpoint,
                Some(&auth),
                &["invoke", "--request-file", "-"],
                exact,
            )
            .await,
        );
        assert_eq!(invoked["revision"], 5);
        assert_eq!(
            json(
                &run(
                    &endpoint,
                    Some(&auth),
                    &["invoke", "--request-file", "-"],
                    exact
                )
                .await
            ),
            invoked
        );
        let deleted = json(
            &run(
                &endpoint,
                Some(&auth),
                &[
                    "delete",
                    "tasks",
                    "one",
                    "--expected",
                    "5",
                    "--idempotency",
                    "delete-one",
                ],
                "",
            )
            .await,
        );
        assert_eq!(deleted["revision"], 6);
        assert_eq!(deleted["value"], Value::Null);
        stop_tx.send(()).unwrap();
        server.await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_live_and_journal_streams_flush_updates_and_stop_on_revocation() {
    use std::{process::Stdio, time::Duration};
    use tokio::io::{AsyncBufReadExt, BufReader};
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    let runtime = rom_demo::build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        Default::default(),
    )
    .unwrap();
    rom_demo::bootstrap(&runtime).await.unwrap();
    let server = Server::new(runtime.clone()).await;
    let mut live = tokio::process::Command::new(env!("CARGO_BIN_EXE_rom"))
        .args([
            "--endpoint",
            &server.endpoint,
            "--auth-file",
            &auth,
            "--output",
            "json",
            "live",
            "tasks",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(live.stdout.take().unwrap()).lines();
    let initial = tokio::time::timeout(Duration::from_secs(3), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(initial, "[]");
    let mut journal = tokio::process::Command::new(env!("CARGO_BIN_EXE_rom"))
        .args([
            "--endpoint",
            &server.endpoint,
            "--auth-file",
            &auth,
            "--output",
            "json",
            "subscribe",
            "tasks",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut batches = BufReader::new(journal.stdout.take().unwrap()).lines();
    let initial = tokio::time::timeout(Duration::from_secs(3), batches.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&initial).unwrap()["events"],
        json!([])
    );
    json(
        &run(
            &server.endpoint,
            Some(&auth),
            &[
                "create",
                "tasks",
                "streamed",
                "--idempotency",
                "streamed",
                "--input-file",
                "-",
            ],
            r#"{"title":"streamed","done":false}"#,
        )
        .await,
    );
    let update = tokio::time::timeout(Duration::from_secs(3), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&update).unwrap()[0]["revision"],
        1
    );
    let batch = tokio::time::timeout(Duration::from_secs(3), batches.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let batch: Value = serde_json::from_str(&batch).unwrap();
    assert_eq!(batch["events"][0]["view"]["key"]["id"], "streamed");
    assert!(batch["cursor"]["position"].as_u64().unwrap() > 0);
    runtime.revoke(&rom_demo::session_actor());
    for child in [&mut live, &mut journal] {
        let status = tokio::time::timeout(Duration::from_secs(3), child.wait())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.code(), Some(3));
    }
    server.finish().await;
}
#[derive(Clone, rom::Resource)]
#[resource(name = "notes")]
struct Note {
    memo: rom::Presence<Option<String>>,
    flag: bool,
    count: u64,
    secret: String,
}
#[tokio::test]
async fn partial_projection_preserves_null_absence_false_and_exact_u64() {
    use rom::Resource;
    let runtime = rom::Runtime::builder()
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .field_policy(|_, access, name, _| {
                    matches!(access, rom::Access::Write) || name != "secret"
                }),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            rom::Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let server = Server::new(runtime).await;
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    let value = json(
        &run(
            &server.endpoint,
            Some(&auth),
            &[
                "create",
                "notes",
                "one",
                "--idempotency",
                "create",
                "--input-file",
                "-",
            ],
            r#"{"flag":false,"count":18446744073709551615,"secret":"private"}"#,
        )
        .await,
    );
    assert_eq!(value["value"], json!({"flag":false,"count":u64::MAX}));
    let value = json(
        &run(
            &server.endpoint,
            Some(&auth),
            &[
                "patch",
                "notes",
                "one",
                "--expected",
                "1",
                "--idempotency",
                "null",
                "--input-file",
                "-",
            ],
            r#"{"memo":{"op":"set","value":null}}"#,
        )
        .await,
    );
    assert_eq!(
        value["value"],
        json!({"memo":null,"flag":false,"count":u64::MAX})
    );
    let value = json(
        &run(
            &server.endpoint,
            Some(&auth),
            &[
                "patch",
                "notes",
                "one",
                "--expected",
                "2",
                "--idempotency",
                "remove",
                "--input-file",
                "-",
            ],
            r#"{"memo":{"op":"remove"},"count":{"op":"set","value":0}}"#,
        )
        .await,
    );
    assert_eq!(value["value"], json!({"flag":false,"count":0}));
    server.finish().await;
}
