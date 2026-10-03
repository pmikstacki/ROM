use super::support::*;

#[tokio::test]
async fn strict_json_and_declared_and_chunked_body_bounds_reject_before_execution() {
    let server = Server::start(Limits {
        body_bytes: 512,
        ..Limits::default()
    })
    .await;
    let valid = serde_json::to_string(&create()).unwrap();
    let duplicate = valid.replace(
        "\"owner\":\"alice\"",
        "\"owner\":\"mallory\",\"owner\":\"alice\"",
    );
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", &duplicate).await),
        400
    );
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    "{\"kind\":\"tasks\",\"id\":\"one\",\"unexpected\":false}"
                )
                .await
        ),
        400
    );
    assert_eq!(
        status(
            &server
                .post("/invoke", "owner-secret", &" ".repeat(513))
                .await
        ),
        413
    );
    let huge = " ".repeat(513);
    let chunked = format!(
        "POST /invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        huge.len(),
        huge
    );
    assert_eq!(status(&server.raw(chunked).await), 413);
    assert_eq!(server.store.counts().unwrap(), [0, 0, 0, 0]);
    server.finish().await;
}

#[tokio::test]
async fn slow_body_owns_independent_admission_until_timeout() {
    let server = Server::start(Limits {
        bodies: 1,
        body_timeout: Duration::from_millis(100),
        ..Limits::default()
    })
    .await;
    let mut slow = TcpStream::connect(server.address).await.unwrap();
    slow.write_all(b"POST /invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nConnection: close\r\nContent-Length: 100\r\n\r\n{").await.unwrap();
    // Readiness is established by an actual second request receiving overload.
    let mut overloaded = false;
    for _ in 0..20 {
        if status(
            &server
                .post(
                    "/read",
                    "owner-secret",
                    "{\"kind\":\"tasks\",\"id\":\"one\"}",
                )
                .await,
        ) == 429
        {
            overloaded = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(overloaded);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), slow.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status(&response), 429);
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    &serde_json::to_string(&create()).unwrap()
                )
                .await
        ),
        200
    );
    server.finish().await;
}
