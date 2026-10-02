use blob_adapters::Adapter;
use blob_contract::{BlobStore, Condition, Error, Limits, Upload};
use futures_util::{StreamExt, stream};
use std::time::Duration;

const LIMITS: Limits = Limits {
    max_blob_bytes: 16,
    max_chunk_bytes: 8,
    max_chunks: 16,
};

fn input(parts: Vec<Result<Vec<u8>, Error>>) -> Upload {
    Box::pin(stream::iter(parts))
}

async fn conformance(store: &dyn BlobStore) {
    assert!(!store.capabilities().conditional_write);
    assert_eq!(store.get("missing").await, Err(Error::NotFound));
    store.delete("missing").await.unwrap();
    let receipt = store
        .put(
            "folder/value",
            input(vec![Ok(b"hello ".to_vec()), Ok(b"world".to_vec())]),
            Condition::Any,
        )
        .await
        .unwrap();
    assert_eq!(receipt.bytes, 11);
    assert_eq!(store.get("folder/value").await.unwrap(), b"hello world");
    store
        .put("folder/value", input(vec![]), Condition::Any)
        .await
        .unwrap();
    assert_eq!(store.get("folder/value").await.unwrap(), b"");
    store.delete("folder/value").await.unwrap();
    assert_eq!(store.get("folder/value").await, Err(Error::NotFound));
    store
        .put(
            "boundary",
            input(vec![Ok(vec![1; 8]), Ok(vec![2; 8])]),
            Condition::Any,
        )
        .await
        .unwrap();
    assert_eq!(
        store.get("boundary").await.unwrap(),
        [vec![1; 8], vec![2; 8]].concat()
    );
    store.delete("boundary").await.unwrap();
}

async fn rejected_uploads_preserve_previous(store: &dyn BlobStore) {
    store
        .put("value", input(vec![Ok(b"old".to_vec())]), Condition::Any)
        .await
        .unwrap();
    for (parts, condition, expected) in [
        (
            vec![Ok(b"new".to_vec())],
            Condition::Absent,
            Error::UnsupportedCondition,
        ),
        (
            vec![Ok(b"new".to_vec())],
            Condition::Matches("etag".into()),
            Error::UnsupportedCondition,
        ),
        (vec![Ok(vec![0; 9])], Condition::Any, Error::LimitExceeded),
        (
            vec![Ok(vec![0; 8]), Ok(vec![0; 8]), Ok(vec![0])],
            Condition::Any,
            Error::LimitExceeded,
        ),
        (vec![Ok(vec![]); 17], Condition::Any, Error::LimitExceeded),
        (
            vec![Ok(b"partial".to_vec()), Err(Error::InputFailed)],
            Condition::Any,
            Error::InputFailed,
        ),
    ] {
        assert_eq!(
            store.put("value", input(parts), condition).await,
            Err(expected)
        );
        assert_eq!(store.get("value").await.unwrap(), b"old");
    }
    // A stalled producer has supplied partial bytes but never completes.
    let stalled = stream::iter([Ok(b"new".to_vec())]).chain(stream::pending());
    let cancelled = tokio::time::timeout(
        Duration::from_millis(30),
        store.put("value", Box::pin(stalled), Condition::Any),
    )
    .await;
    assert!(cancelled.is_err());
    assert_eq!(store.get("value").await.unwrap(), b"old");
}

async fn invalid_keys_fail_closed(store: &dyn BlobStore) {
    for key in [
        "",
        "/absolute",
        "../escape",
        "folder/../escape",
        "folder//empty",
        "back\\slash",
        "trailing/",
        "a.b",
        "UPPER",
    ] {
        assert_eq!(
            store.put(key, input(vec![]), Condition::Any).await,
            Err(Error::InvalidKey)
        );
        assert_eq!(store.get(key).await, Err(Error::InvalidKey));
        assert_eq!(store.delete(key).await, Err(Error::InvalidKey));
    }
}

#[tokio::test]
async fn folder_roundtrip_and_missing_normalization() {
    let root = tempfile::tempdir().unwrap();
    let store = Adapter::trusted_folder(root.path(), LIMITS).unwrap();
    conformance(&store).await;
}

#[tokio::test]
async fn folder_limits_conditions_and_partial_input_preserve_content() {
    let root = tempfile::tempdir().unwrap();
    let store = Adapter::trusted_folder(root.path(), LIMITS).unwrap();
    rejected_uploads_preserve_previous(&store).await;
}

#[tokio::test]
async fn folder_rejects_nonportable_paths() {
    let root = tempfile::tempdir().unwrap();
    let store = Adapter::trusted_folder(root.path(), LIMITS).unwrap();
    invalid_keys_fail_closed(&store).await;
}

#[tokio::test]
async fn folder_external_oversized_blob_is_not_returned() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("large"), [0; 17]).unwrap();
    let store = Adapter::trusted_folder(root.path(), LIMITS).unwrap();
    assert_eq!(store.get("large").await, Err(Error::LimitExceeded));
}

#[cfg(unix)]
#[tokio::test]
async fn folder_prefix_is_explicitly_not_a_symlink_sandbox() {
    // Characterization of the documented trust constraint, not a security guarantee.
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("value"), b"outside").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    let store = Adapter::trusted_folder(root.path(), LIMITS).unwrap();
    assert_eq!(store.get("link/value").await.unwrap(), b"outside");
}

#[test]
fn s3_refuses_external_addresses() {
    assert!(matches!(
        Adapter::loopback_s3(
            "192.0.2.1:9000".parse().unwrap(),
            "bucket",
            "test",
            "test",
            LIMITS
        ),
        Err(Error::Denied)
    ));
}

fn s3(endpoint: std::net::SocketAddr) -> Adapter {
    Adapter::loopback_s3(
        endpoint,
        "rom-probe",
        "rom-probe",
        "rom-probe-local-only",
        LIMITS,
    )
    .unwrap()
}

fn endpoint() -> std::net::SocketAddr {
    std::env::var("ROM_BLOB_S3_ENDPOINT")
        .expect("loopback MinIO endpoint required")
        .parse()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires loopback MinIO; run verify-s3 from README"]
async fn s3_same_contract_and_fault_cases() {
    let store = s3(endpoint());
    // Run serially because these deliberately use the same named keys.
    conformance(&store).await;
    rejected_uploads_preserve_previous(&store).await;
    invalid_keys_fail_closed(&store).await;
    store.delete("value").await.unwrap();
}

#[tokio::test]
#[ignore = "requires loopback MinIO; run verify-s3 from README"]
async fn s3_lost_response_reports_unknown_even_when_object_committed() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };
    let upstream = endpoint();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = listener.local_addr().unwrap();
    let relay = tokio::spawn(async move {
        let (client, _) = listener.accept().await.unwrap();
        let server = TcpStream::connect(upstream).await.unwrap();
        let (mut from_client, mut to_client) = client.into_split();
        let (mut from_server, mut to_server) = server.into_split();
        let upload =
            tokio::spawn(async move { tokio::io::copy(&mut from_client, &mut to_server).await });
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(from_server.read_u8().await.unwrap());
            assert!(headers.len() < 16_384);
        }
        assert!(
            headers.starts_with(b"HTTP/1.1 200"),
            "unexpected upstream response: {}",
            String::from_utf8_lossy(&headers)
        );
        // Real MinIO accepted the PUT. Deliberately lose the acknowledgment.
        to_client.shutdown().await.unwrap();
        upload.abort();
        let _ = upload.await;
    });
    assert_eq!(
        s3(proxy)
            .put(
                "lost-ack",
                input(vec![Ok(b"accepted".to_vec())]),
                Condition::Any
            )
            .await,
        Err(Error::Unknown)
    );
    tokio::time::timeout(Duration::from_secs(5), relay)
        .await
        .expect("proxy must observe a completed PUT")
        .unwrap();
    let store = s3(upstream);
    assert_eq!(store.get("lost-ack").await.unwrap(), b"accepted");
    store.delete("lost-ack").await.unwrap();
}
