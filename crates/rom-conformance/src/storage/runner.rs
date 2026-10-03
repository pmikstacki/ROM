use super::{
    StorageFixture, assert_bundle,
    scenario::{Record, create, update},
    typed,
};
use crate::{
    ConformanceResult, PROFILE_VERSION,
    error::{check, observe},
    profile,
};
use rom::{Error, Intent, Resource, json};

/// Run the current assertion profile. Each scenario requires a fresh exclusive store.
pub async fn basic(
    factory: impl Fn() -> rom::Result<Box<dyn StorageFixture>>,
) -> ConformanceResult {
    for_profile(PROFILE_VERSION, factory).await
}
/// Reject an unsupported assertion profile before invoking the fixture factory.
pub async fn for_profile(
    version: u32,
    factory: impl Fn() -> rom::Result<Box<dyn StorageFixture>>,
) -> ConformanceResult {
    profile::require(version)?;
    atomic(&mut *observe(factory(), "storage.factory")?)?;
    typed::run(&mut *observe(factory(), "storage.factory")?).await
}
fn atomic(fixture: &mut dyn StorageFixture) -> ConformanceResult {
    let storage = fixture.storage();
    super::assertions::fresh(fixture)?;
    observe(
        storage.register(&[Record::descriptor()]),
        "storage.register",
    )?;
    let initial = create();
    check(
        observe(storage.commit(&initial), "storage.create")? == initial.receipt,
        "storage.create.receipt",
    )?;
    let updated = update();
    check(
        observe(storage.commit(&updated), "storage.update")? == updated.receipt,
        "storage.update.receipt",
    )?;
    assert_bundle(
        &*storage,
        &observe(fixture.facts(), "storage.facts")?,
        &initial.receipt,
        &updated,
    )?;
    check(
        observe(storage.commit(&updated), "storage.replay")? == updated.receipt,
        "storage.replay.receipt",
    )?;
    assert_bundle(
        &*storage,
        &observe(fixture.facts(), "storage.facts")?,
        &initial.receipt,
        &updated,
    )?;
    let mut conflict = updated.clone();
    conflict.receipt.fingerprint = "different".into();
    check(
        storage.commit(&conflict) == Err(Error::IdentityMismatch),
        "storage.fingerprint",
    )?;
    let mut stale = updated.clone();
    stale.receipt.identity = "stale".into();
    stale.receipt.fingerprint = "stale".into();
    check(
        storage.commit(&stale) == Err(Error::Conflict),
        "storage.stale",
    )?;
    assert_bundle(
        &*storage,
        &observe(fixture.facts(), "storage.facts")?,
        &initial.receipt,
        &updated,
    )?;
    drop(storage);
    observe(fixture.reopen(), "storage.reopen")?;
    let storage = fixture.storage();
    assert_bundle(
        &*storage,
        &observe(fixture.facts(), "storage.facts")?,
        &initial.receipt,
        &updated,
    )?;
    check(
        observe(storage.commit(&updated), "storage.reopen.replay")? == updated.receipt,
        "storage.reopen.receipt",
    )?;
    assert_bundle(
        &*storage,
        &observe(fixture.facts(), "storage.facts")?,
        &initial.receipt,
        &updated,
    )?;
    let mut noop = updated.clone();
    noop.expected = Some(2);
    noop.changed = false;
    noop.effects.clear();
    noop.receipt.identity = "noop".into();
    noop.receipt.fingerprint = "noop".into();
    check(
        observe(storage.commit(&noop), "storage.noop")? == noop.receipt,
        "storage.noop.receipt",
    )?;
    let facts = observe(fixture.facts(), "storage.facts")?;
    check(facts.counts == [1, 2, 3, 2], "storage.noop.counts")?;
    check(
        facts.events == [initial.receipt.row, updated.receipt.row.clone()],
        "storage.noop.events",
    )?;
    let expected: Vec<_> = updated
        .effects
        .iter()
        .cloned()
        .map(|i| (updated.receipt.identity.clone(), i))
        .collect();
    check(facts.effects == expected, "storage.noop.effects")?;
    let mut invalid = noop.clone();
    invalid.receipt.identity = "invalid".into();
    invalid.effects.push(Intent::new("bad", json!(null)));
    check(
        storage.commit(&invalid) == Err(Error::NotCommitted),
        "storage.noop.invalid.effects",
    )?;
    invalid.effects.clear();
    invalid.receipt.row.value = Some(json!({"done":false}));
    check(
        storage.commit(&invalid) == Err(Error::NotCommitted),
        "storage.noop.invalid.row",
    )?;
    check(
        observe(fixture.facts(), "storage.facts")? == facts,
        "storage.noop.unchanged",
    )?;
    check(
        observe(storage.load(&updated.receipt.row.key), "storage.noop.load")?
            == Some(updated.receipt.row),
        "storage.noop.row",
    )?;
    Ok(())
}
