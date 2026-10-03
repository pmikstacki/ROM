mod common;
use common::*;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn explicit_epoch_is_sent_unchanged_and_expiry_never_triggers_automatic_retry() {
    for mutation in ["create", "delete"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let body = request(&mut socket).await;
            assert_eq!(body["retry_epoch"], 7);
            assert_eq!(body["idempotency"], "original-key");
            let reply = r#"{"error":"identity_expired"}"#;
            socket.write_all(format!("HTTP/1.1 410 Gone\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).as_bytes()).await.unwrap();
            drop(socket);
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                    .await
                    .is_err()
            );
        });
        let mut args = vec![
            mutation,
            "tasks",
            "one",
            "--retry-epoch",
            "7",
            "--idempotency",
            "original-key",
        ];
        if mutation == "create" {
            args.extend(["--input-file", "-"]);
        } else {
            args.extend(["--expected", "1"]);
        }
        let output = run(&endpoint, None, &args, "{}").await;
        assert_eq!(output.status.code(), Some(5));
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("retry identity expired"));
        assert!(message.contains("do not retry the old operation with a new epoch"));
        task.await.unwrap();
    }
}
