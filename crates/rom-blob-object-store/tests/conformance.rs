use rom_blob::*;
use rom_blob_object_store::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-blobs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn key(input: &[u8]) -> ObjectKey {
    ObjectKey::parse(Digest::of(input).as_str()).unwrap()
}
#[tokio::test]
async fn folder_atomic_create_bounds_and_reopen() {
    let scratch = Scratch::new();
    let store = Adapter::trusted_folder(&scratch.0, 16).unwrap();
    rom_conformance::blob::basic(&store).await.unwrap();
    let k = key(b"restart");
    store.create(&k, b"persist".to_vec()).await.unwrap();
    drop(store);
    let reopened = Adapter::trusted_folder(&scratch.0, 16).unwrap();
    assert_eq!(reopened.get(&k, 16).await.unwrap(), b"persist");
    let external = key(b"externally-oversized");
    std::fs::write(scratch.0.join(external.as_str()), vec![0; 17]).unwrap();
    assert_eq!(reopened.get(&external, 16).await, Err(Error::TooLarge));
}
#[test]
fn physical_keys_and_remote_endpoint_policy_reject_unsafe_input() {
    for k in ["../x", "/etc/passwd", "", "a/b", "ABC"] {
        assert!(ObjectKey::parse(k).is_err());
    }
    for endpoint in [
        "http://example.com",
        "http://localhost:9000",
        "http://127.0.0.1:9000/#fragment",
    ] {
        assert!(
            Adapter::s3(
                S3Config {
                    endpoint,
                    region: "us-east-1",
                    bucket: "rom-probe",
                    access_key: "test",
                    secret: "test",
                    policy: EndpointPolicy::LoopbackTestOnly
                },
                16
            )
            .is_err()
        );
    }
}
#[tokio::test]
#[ignore = "requires disposable real MinIO; run verify-s3"]
async fn real_s3_uses_identical_contract() {
    let endpoint = std::env::var("ROM_BLOB_S3_ENDPOINT").unwrap();
    let store = Adapter::s3(
        S3Config {
            endpoint: &endpoint,
            region: "us-east-1",
            bucket: "rom-probe",
            access_key: "rom-probe",
            secret: "rom-probe-local-only",
            policy: EndpointPolicy::LoopbackTestOnly,
        },
        16,
    )
    .unwrap();
    rom_conformance::blob::basic(&store).await.unwrap();
}

#[tokio::test]
#[ignore = "requires disposable real MinIO; run verify-s3"]
async fn real_s3_lost_publish_response_is_unknown_and_reconcilable() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };
    let endpoint = std::env::var("ROM_BLOB_S3_ENDPOINT").unwrap();
    let upstream = endpoint.strip_prefix("http://").unwrap().to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = listener.local_addr().unwrap();
    let relay = tokio::spawn(async move {
        let (client, _) = listener.accept().await.unwrap();
        let server = TcpStream::connect(upstream).await.unwrap();
        let (mut from_client, mut to_client) = client.into_split();
        let (mut from_server, mut to_server) = server.into_split();
        let upload =
            tokio::spawn(async move { tokio::io::copy(&mut from_client, &mut to_server).await });
        let mut headers = vec![];
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(from_server.read_u8().await.unwrap());
            assert!(headers.len() < 16384);
        }
        assert!(
            headers.starts_with(b"HTTP/1.1 200"),
            "provider did not accept publication"
        );
        to_client.shutdown().await.unwrap();
        upload.abort();
        let _ = upload.await;
    });
    let proxied = format!("http://{proxy}");
    let config = |endpoint| S3Config {
        endpoint,
        region: "us-east-1",
        bucket: "rom-probe",
        access_key: "rom-probe",
        secret: "rom-probe-local-only",
        policy: EndpointPolicy::LoopbackTestOnly,
    };
    let key = key(b"real-lost-ack");
    assert_eq!(
        Adapter::s3(config(&proxied), 16)
            .unwrap()
            .create(&key, b"accepted".to_vec())
            .await,
        Err(Error::Unknown)
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), relay)
        .await
        .unwrap()
        .unwrap();
    let direct = Adapter::s3(config(&endpoint), 16).unwrap();
    assert_eq!(direct.get(&key, 16).await.unwrap(), b"accepted");
    direct.delete(&key).await.unwrap();
}

#[tokio::test]
async fn folder_and_sqlite_restart_recover_real_attachment_and_detachment() {
    use rom::{Actor, Runtime};
    use std::{collections::BTreeMap, sync::Arc};
    let scratch = Scratch::new();
    let blobs = scratch.0.join("objects");
    std::fs::create_dir(&blobs).unwrap();
    let database = scratch.0.join("metadata.db");
    let actor = Actor::trusted("local", "alice");
    for pass in 0..2 {
        let store = Arc::new(Adapter::trusted_folder(&blobs, 16).unwrap());
        let runtime = Runtime::builder()
            .resource(definition())
            .build(
                Arc::new(rom_sqlite::Sqlite::open(&database).unwrap()),
                Runtime::shared_cpu_pool(2).unwrap(),
            )
            .unwrap();
        let service = BlobService::new(
            runtime.clone(),
            BTreeMap::from([("folder".into(), store.clone() as Arc<dyn BlobStore>)]),
            Limits::default(),
        )
        .unwrap();
        if pass == 0 {
            service
                .reserve(
                    &actor,
                    "asset",
                    "folder",
                    Digest::of(b"content"),
                    7,
                    "reserve",
                )
                .await
                .unwrap();
            let bytes = Box::pin(futures_util::stream::iter(vec![Ok(b"content".to_vec())]));
            assert!(matches!(
                service.upload(&actor, "asset", bytes).await.unwrap(),
                UploadOutcome::Attached(_)
            ));
        }
        assert_eq!(service.read(&actor, "asset").await.unwrap(), b"content");
        if pass == 1 {
            let detached = service.detach(&actor, "asset").await.unwrap();
            assert!(service.read(&actor, "asset").await.is_err());
            service.shutdown().await.unwrap();
            // Fixture has no competing writer; host proves quiescence before maintenance.
            store.delete(&detached.key).await.unwrap();
            assert_eq!(store.head(&detached.key).await, Err(Error::Missing));
        } else {
            service.shutdown().await.unwrap();
        }
        runtime.shutdown().await.unwrap();
    }
}
