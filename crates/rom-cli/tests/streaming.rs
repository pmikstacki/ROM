mod common;
use common::*;
use serde_json::{Value, json};
use std::{
    process::{Command, Stdio},
    time::Duration,
};
use tokio::io::AsyncWriteExt;
#[tokio::test]
async fn fragmented_multiline_crlf_keeps_whole_batches_and_explicit_errors() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let data = "\r\n:keepalive\r\n\r\nevent: data\r\ndata: {\"events\": [],\r\ndata: \"cursor\": {\"generation\":\"é\",\"kind\":\"tasks\",\"position\":7}}\r\n\r\nevent: error\r\ndata: history_gap\r\n\r\n";
        for byte in data.as_bytes() {
            socket.write_all(&[*byte]).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    let out = run(&endpoint, None, &["subscribe", "tasks"], "").await;
    assert_eq!(out.status.code(), Some(6));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        value,
        json!({"events":[],"cursor":{"generation":"é","kind":"tasks","position":7}})
    );
    task.await.unwrap();
}
#[tokio::test]
async fn malformed_oversized_and_unfinished_streams_stop_without_success() {
    for body in [
        b"event: data\ndata: {\n\n".to_vec(),
        b"event: data\ndata: []".to_vec(),
        vec![b'x'; 2 * 1024 * 1024 + 1],
        b"event: error\ndata: untrusted-secret\n\n".to_vec(),
    ] {
        let (endpoint, task) = response(200, "text/event-stream", body).await;
        let out = run(&endpoint, None, &["live", "tasks"], "").await;
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("untrusted-secret"));
        task.await.unwrap();
    }
}
#[tokio::test]
async fn completed_snapshot_is_emitted_before_stream_loss() {
    let (endpoint, task) = response(200, "text/event-stream", b"data: []\n\n".to_vec()).await;
    let out = run(&endpoint, None, &["live", "tasks"], "").await;
    assert_eq!(out.status.code(), Some(4));
    assert_eq!(out.stdout, b"[]\n");
    task.await.unwrap();
}
#[tokio::test]
async fn inactive_stream_times_out() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        request(&mut s).await;
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });
    let out = run(
        &endpoint,
        None,
        &["--idle-timeout", "1", "live", "tasks"],
        "",
    )
    .await;
    assert_eq!(out.status.code(), Some(4));
    task.abort();
}
#[cfg(unix)]
#[tokio::test]
async fn closed_stdout_stops_stream_cleanly_and_ctrl_c_exits_a_blocked_pipe() {
    for interrupt in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            request(&mut s).await;
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
                .await
                .unwrap();
            let frame = format!(
                "data: {}\n\n",
                json!([{"key":{"kind":"tasks","id":"one"},"revision":1,"value":{"text":"x".repeat(200_000)}}])
            );
            let _ = s.write_all(frame.as_bytes()).await;
            let _ = ready_tx.send(());
            tokio::time::sleep(Duration::from_secs(20)).await;
        });
        let mut child = Command::new(env!("CARGO_BIN_EXE_rom"))
            .args(["--endpoint", &endpoint, "live", "tasks"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if !interrupt {
            drop(child.stdout.take());
        }
        ready_rx.await.unwrap();
        if interrupt {
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert!(
                Command::new("kill")
                    .args(["-INT", &child.id().to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::task::spawn_blocking(move || child.wait().unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(result.success(), "{result:?}");
        task.abort();
    }
}
#[tokio::test]
async fn journal_stream_keeps_prior_generation_and_position_between_frames() {
    for (generation, position) in [("restored", 101), ("original", 99)] {
        let first =
            json!({"events":[],"cursor":{"generation":"original","kind":"tasks","position":100}});
        let second = json!({"events":[],"cursor":{"generation":generation,"kind":"tasks","position":position}});
        let body = format!("data: {first}\n\ndata: {second}\n\n");
        let (endpoint, task) = response(200, "text/event-stream", body.into_bytes()).await;
        let out = run(&endpoint, None, &["subscribe", "tasks"], "").await;
        assert_eq!(out.status.code(), Some(4));
        let rows: String = String::from_utf8(out.stdout).unwrap();
        assert_eq!(rows.lines().count(), 1);
        assert_eq!(serde_json::from_str::<Value>(&rows).unwrap(), first);
        task.await.unwrap();
    }
}
