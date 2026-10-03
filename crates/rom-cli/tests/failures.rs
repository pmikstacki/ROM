mod common;
use common::*;
use serde_json::{Value, json};
#[tokio::test]
async fn remote_categories_do_not_invent_rollback_and_never_echo_body() {
    for (status, category, code) in [
        (403, "denied", 5),
        (429, "overloaded", 5),
        (503, "closed", 5),
        (500, "internal", 5),
        (503, "outcome_unknown", 5),
        (503, "not_committed", 5),
        (409, "conflict", 5),
        (409, "identity_mismatch", 5),
        (410, "identity_expired", 5),
        (400, "invalid", 5),
        (200, "secret-malformed-envelope", 5),
    ] {
        let (endpoint, task) = response(
            status,
            "application/json",
            json!({"error":category}).to_string().into_bytes(),
        )
        .await;
        let out = run(
            &endpoint,
            None,
            &[
                "create",
                "tasks",
                "one",
                "--idempotency",
                "key",
                "--input-file",
                "-",
            ],
            r#"{"title":"secret-payload"}"#,
        )
        .await;
        assert_eq!(out.status.code(), Some(code), "{category}");
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
        task.await.unwrap();
    }
}
#[tokio::test]
async fn invalid_responses_are_transport_errors_for_reads_and_uncertain_for_mutations() {
    for mutation in [false, true] {
        for body in [
            b"<html>secret</html>".to_vec(),
            vec![b'x'; 2 * 1024 * 1024 + 1],
            b"{\"revision\":1,\"revision\":2}".to_vec(),
        ] {
            let (endpoint, task) = response(200, "application/json", body).await;
            let args = if mutation {
                vec![
                    "delete",
                    "tasks",
                    "one",
                    "--expected",
                    "1",
                    "--idempotency",
                    "key",
                ]
            } else {
                vec!["read", "tasks", "one"]
            };
            let out = run(&endpoint, None, &args, "").await;
            assert_eq!(out.status.code(), Some(if mutation { 5 } else { 4 }));
            assert!(out.stdout.is_empty());
            assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
            task.await.unwrap();
        }
    }
}
#[tokio::test]
async fn output_escapes_terminal_controls_in_values_and_identifiers() {
    let payload = json!({"key":{"kind":"tasks","id":"one\u{1b}[31m"},"revision":1,"value":{"title":"\u{9b}31m\nline"}});
    let (endpoint, task) =
        response(200, "application/json", payload.to_string().into_bytes()).await;
    let out = run(&endpoint, None, &["read", "tasks", "one\u{1b}[31m"], "").await;
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        payload
    );
    assert!(!out.stdout.contains(&0x1b));
    assert!(!String::from_utf8_lossy(&out.stdout).contains('\u{9b}'));
    task.await.unwrap();
}
#[tokio::test]
async fn auth_file_header_injection_and_oversize_are_local_redacted_failures() {
    let dir = Directory::new();
    for value in ["secret\r\nInjected: yes".to_string(), "secret".repeat(2000)] {
        let auth = dir.file("auth", value);
        let out = run(
            "http://127.0.0.1:1",
            Some(&auth),
            &["read", "tasks", "one"],
            "",
        )
        .await;
        assert_eq!(out.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&out.stderr).contains("secret"));
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_mutation_with_lost_http_reply_recovers_only_with_original_identity() {
    use std::sync::Arc;
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = rom_demo::build(store, Default::default()).unwrap();
    rom_demo::bootstrap(&runtime).await.unwrap();
    let server = Server::new(runtime.clone()).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let target = server.endpoint.clone();
    let task = tokio::spawn(async move {
        let (mut downstream, _) = listener.accept().await.unwrap();
        let body = request(&mut downstream).await;
        let reply = reqwest::Client::new()
            .post(format!("{target}/invoke"))
            .header("Authorization", "Demo local")
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(reply.status(), 200);
        let _ = reply.bytes().await.unwrap();
        // HTTP committed and replied, but the intermediary loses that reply.
        drop(downstream);
    });
    let args = [
        "create",
        "tasks",
        "lost",
        "--idempotency",
        "original-key",
        "--input-file",
        "-",
    ];
    let body = r#"{"title":"durable","done":false}"#;
    let lost = run(&proxy, Some(&auth), &args, body).await;
    assert_eq!(lost.status.code(), Some(5));
    task.await.unwrap();
    assert_eq!(
        runtime
            .read_projected(&rom_demo::session_actor(), "tasks", "lost")
            .await
            .unwrap()
            .revision,
        1
    );
    let replay = json(&run(&server.endpoint, Some(&auth), &args, body).await);
    assert_eq!(replay["revision"], 1);
    let mismatch = run(
        &server.endpoint,
        Some(&auth),
        &args,
        r#"{"title":"different","done":false}"#,
    )
    .await;
    assert_eq!(mismatch.status.code(), Some(5));
    let events = runtime
        .journal(&rom_demo::session_actor(), "tasks", None)
        .await
        .unwrap();
    assert_eq!(events.events.len(), 1);
    server.finish().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authorization_revoked_after_commit_is_uncertain_but_state_is_durable() {
    use rom::Storage;
    use std::sync::Arc;
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = rom_demo::build(store.clone(), Default::default()).unwrap();
    rom_demo::bootstrap(&runtime).await.unwrap();
    let revoked = runtime.clone();
    store.on_commit(Some(Arc::new(move |point| {
        if point == usize::MAX {
            revoked.revoke(&rom_demo::session_actor());
        }
        Ok(())
    })));
    let server = Server::new(runtime).await;
    let out = run(
        &server.endpoint,
        Some(&auth),
        &[
            "create",
            "tasks",
            "revoked",
            "--idempotency",
            "revoked-key",
            "--input-file",
            "-",
        ],
        r#"{"title":"committed","done":false}"#,
    )
    .await;
    assert_eq!(out.status.code(), Some(5));
    assert!(out.stdout.is_empty());
    let row = store
        .load(&rom::Key {
            kind: "tasks".into(),
            id: "revoked".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(row.revision, 1);
    assert_eq!(row.value.unwrap()["title"], "committed");
    store.on_commit(None);
    server.finish().await;
}
#[tokio::test]
async fn redirect_is_not_followed_or_treated_as_a_success() {
    use tokio::io::AsyncWriteExt;
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", origin.local_addr().unwrap());
    let location = format!("http://{}/read", target.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = origin.accept().await.unwrap();
        request(&mut socket).await;
        socket.write_all(format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n").as_bytes()).await.unwrap();
    });
    let out = run(&endpoint, None, &["read", "tasks", "id"], "").await;
    assert_eq!(out.status.code(), Some(4));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), target.accept())
            .await
            .is_err()
    );
    task.await.unwrap();
}
#[cfg(unix)]
#[tokio::test]
async fn interrupt_during_mutation_returns_uncertain_130() {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::Duration,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut child = Command::new(env!("CARGO_BIN_EXE_rom"))
        .args([
            "--endpoint",
            &endpoint,
            "delete",
            "tasks",
            "id",
            "--expected",
            "1",
            "--idempotency",
            "key",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"").unwrap();
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
        .await
        .unwrap()
        .unwrap();
    request(&mut socket).await;
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let out = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::task::spawn_blocking(move || child.wait_with_output().unwrap()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(out.status.code(), Some(130));
    assert!(out.stdout.is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn arbitrary_actor_gate_error_after_commit_cannot_prove_rejection() {
    use rom::Storage;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Gate(Arc<AtomicBool>);
    impl rom::ActorGate for Gate {
        fn check(&self, _: &rom::Actor, _: &mut dyn rom::AuthorizationRead) -> rom::Result<()> {
            if self.0.load(Ordering::SeqCst) {
                Err(rom::Error::TooLarge)
            } else {
                Ok(())
            }
        }
    }
    let denied = Arc::new(AtomicBool::new(false));
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = rom_demo::declarations(Default::default())
        .unwrap()
        .actor_gate(Arc::new(Gate(denied.clone())))
        .build(store.clone(), rom::Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let after = denied.clone();
    store.on_commit(Some(Arc::new(move |point| {
        if point == usize::MAX {
            after.store(true, Ordering::SeqCst);
        }
        Ok(())
    })));
    let server = Server::new(runtime).await;
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    let out = run(
        &server.endpoint,
        Some(&auth),
        &[
            "create",
            "tasks",
            "gate-error",
            "--idempotency",
            "gate-error",
            "--input-file",
            "-",
        ],
        r#"{"title":"persisted","done":false}"#,
    )
    .await;
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(
        store
            .load(&rom::Key {
                kind: "tasks".into(),
                id: "gate-error".into()
            })
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    store.on_commit(None);
    server.finish().await;
}
#[tokio::test]
async fn response_identities_and_journal_continuity_match_the_request() {
    let dir = Directory::new();
    let after = dir.file(
        "cursor",
        r#"{"generation":"original","kind":"tasks","position":100}"#,
    );
    let wrong = json!({"key":{"kind":"other-kind","id":"other-id"},"revision":1,"value":{}});
    let cursor = json!({"generation":"original","kind":"tasks","position":101});
    let cases = vec![
        (vec!["read", "tasks", "requested-id"], wrong.clone(), 4),
        (
            vec![
                "delete",
                "tasks",
                "requested-id",
                "--expected",
                "1",
                "--idempotency",
                "key",
            ],
            wrong.clone(),
            5,
        ),
        (vec!["query", "tasks"], json!([wrong.clone()]), 4),
        (
            vec!["journal-head", "tasks"],
            json!({"generation":"original","kind":"other-kind","position":101}),
            4,
        ),
        (
            vec!["journal", "tasks", "--after-file", &after],
            json!({"events":[],"cursor":{"generation":"restored","kind":"tasks","position":1}}),
            4,
        ),
        (
            vec!["journal", "tasks", "--after-file", &after],
            json!({"events":[],"cursor":{"generation":"original","kind":"tasks","position":99}}),
            4,
        ),
        (
            vec!["journal", "tasks", "--after-file", &after],
            json!({"events":[{"position":101,"view":wrong}],"cursor":cursor}),
            4,
        ),
        (
            vec!["journal", "tasks", "--after-file", &after],
            json!({"events":[{"position":102,"view":{"key":{"kind":"tasks","id":"one"},"revision":1,"value":{}}}],"cursor":cursor}),
            4,
        ),
    ];
    for (args, body, code) in cases {
        let (endpoint, task) =
            response(200, "application/json", body.to_string().into_bytes()).await;
        let out = run(&endpoint, None, &args, "").await;
        assert_eq!(out.status.code(), Some(code), "{args:?}");
        assert!(out.stdout.is_empty());
        task.await.unwrap();
    }
}
#[cfg(unix)]
#[tokio::test]
async fn mutation_interrupt_does_not_wait_for_a_full_stderr_pipe() {
    use std::{
        io::Write,
        os::{fd::OwnedFd, unix::net::UnixStream},
        process::{Command, Stdio},
        time::Duration,
    };
    let (mut stderr, _reader) = UnixStream::pair().unwrap();
    stderr.set_nonblocking(true).unwrap();
    loop {
        match stderr.write(&[b'x'; 8192]) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("{e}"),
        }
    }
    stderr.set_nonblocking(false).unwrap();
    let fd: OwnedFd = stderr.into();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rom"))
        .args([
            "--endpoint",
            &endpoint,
            "delete",
            "tasks",
            "id",
            "--expected",
            "1",
            "--idempotency",
            "key",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::from(fd))
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    request(&mut socket).await;
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().unwrap().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let result = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
    if result.is_err() {
        child.kill().await.unwrap();
    }
    assert_eq!(
        result
            .expect("interrupt must not wait on stderr")
            .unwrap()
            .code(),
        Some(130)
    );
}
#[tokio::test]
async fn strict_json_matches_http_duplicate_key_rejection() {
    use std::sync::Arc;
    let runtime = rom_demo::build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        Default::default(),
    )
    .unwrap();
    let server = Server::new(runtime).await;
    let dir = Directory::new();
    let auth = dir.file("auth", "Demo local");
    for body in [
        r#"{"kind":"tasks","kind":"inventory","id":"one","expected":null,"idempotency":"key","operation":{"type":"create","input":{"title":"x","done":false}}}"#,
        r#"{"kind":"tasks","id":"one","expected":null,"idempotency":"key","operation":{"type":"create","input":{"title":"x","done":false,"done":true}}}"#,
    ] {
        assert_eq!(
            run(
                &server.endpoint,
                Some(&auth),
                &["invoke", "--request-file", "-"],
                body
            )
            .await
            .status
            .code(),
            Some(2)
        );
        let remote = reqwest::Client::new()
            .post(format!("{}/invoke", server.endpoint))
            .header("Authorization", "Demo local")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(remote.status(), 400);
    }
    server.finish().await;
}
#[tokio::test]
async fn environment_proxy_is_not_used() {
    use std::{process::Command, time::Duration};
    let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
    let (endpoint, task) = response(
        200,
        "application/json",
        json!({"key":{"kind":"tasks","id":"one"},"revision":1,"value":{}})
            .to_string()
            .into_bytes(),
    )
    .await;
    let out = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_rom"))
            .args(["--endpoint", &endpoint, "read", "tasks", "one"])
            .env("HTTP_PROXY", &proxy_url)
            .env("http_proxy", &proxy_url)
            .env("ALL_PROXY", &proxy_url)
            .env("all_proxy", &proxy_url)
            .env_remove("NO_PROXY")
            .env_remove("no_proxy")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(out.status.success());
    task.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), proxy.accept())
            .await
            .is_err()
    );
}
