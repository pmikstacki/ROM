use crate::{
    ConformanceResult,
    error::{check, observe},
};
use rom_blob::{BlobStore, Digest, Error, ObjectKey};

fn key(input: &[u8]) -> ObjectKey {
    ObjectKey::parse(Digest::of(input).as_str()).expect("fixed digest key")
}
/// Requires a fresh disposable namespace and an adapter object limit of 16 bytes.
/// Deletes fixed fixture keys; it is not safe for a production namespace.
pub async fn basic(store: &dyn BlobStore) -> ConformanceResult {
    let k = key(b"contract");
    observe(store.delete(&k).await, "blob.delete.initial")?;
    check(
        store.get(&k, 16).await == Err(Error::Missing),
        "blob.missing",
    )?;
    observe(store.create(&k, b"original".to_vec()).await, "blob.create")?;
    check(
        store.create(&k, b"replace".to_vec()).await == Err(Error::Conflict),
        "blob.create.conflict",
    )?;
    check(
        observe(store.get(&k, 16).await, "blob.get")? == b"original",
        "blob.immutable",
    )?;
    check(
        observe(store.head(&k).await, "blob.head")?.bytes == 8,
        "blob.head.bytes",
    )?;
    check(
        store.get(&k, 7).await == Err(Error::TooLarge),
        "blob.read.bound",
    )?;
    check(
        store.create(&key(b"large"), vec![0; 17]).await == Err(Error::TooLarge),
        "blob.create.bound",
    )?;
    let empty = key(b"empty");
    observe(store.delete(&empty).await, "blob.empty.delete.initial")?;
    observe(store.create(&empty, vec![]).await, "blob.empty.create")?;
    check(
        observe(store.get(&empty, 0).await, "blob.empty.get")?.is_empty(),
        "blob.empty.bytes",
    )?;
    observe(store.delete(&empty).await, "blob.empty.delete")?;
    observe(store.delete(&k).await, "blob.delete")?;
    observe(store.delete(&k).await, "blob.delete.replay")?;
    let race = key(b"race");
    observe(store.delete(&race).await, "blob.race.delete.initial")?;
    let (first, second) = tokio::join!(
        store.create(&race, b"first".to_vec()),
        store.create(&race, b"second".to_vec())
    );
    check(
        (first == Ok(()) && second == Err(Error::Conflict))
            || (second == Ok(()) && first == Err(Error::Conflict)),
        "blob.race.arbitration",
    )?;
    check(
        observe(store.get(&race, 16).await, "blob.race.get")?
            == if first.is_ok() {
                b"first".to_vec()
            } else {
                b"second".to_vec()
            },
        "blob.race.value",
    )?;
    observe(store.delete(&race).await, "blob.race.delete")?;
    let exact = key(b"exact");
    observe(store.delete(&exact).await, "blob.exact.delete.initial")?;
    observe(store.create(&exact, vec![1; 16]).await, "blob.exact.create")?;
    check(
        observe(store.get(&exact, 16).await, "blob.exact.get")?.len() == 16,
        "blob.exact.bytes",
    )?;
    observe(store.delete(&exact).await, "blob.exact.delete")?;
    Ok(())
}
