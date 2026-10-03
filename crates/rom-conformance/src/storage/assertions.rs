use super::{StorageFacts, StorageFixture};
use crate::{
    ConformanceResult,
    error::{check, observe},
};
use rom::{Bundle, Receipt, Storage};

pub(super) fn fresh(fixture: &dyn StorageFixture) -> ConformanceResult {
    let empty = observe(fixture.facts(), "storage.facts")?;
    check(
        empty.counts == [0; 4] && empty.events.is_empty() && empty.effects.is_empty(),
        "storage.fixture.empty",
    )
}

/// Assert the fresh-store two-commit baseline, with two update effects.
/// Requires exactly one Row, two events and two receipts; not arbitrary history.
pub fn assert_bundle(
    storage: &dyn Storage,
    facts: &StorageFacts,
    previous: &Receipt,
    bundle: &Bundle,
) -> ConformanceResult {
    check(
        observe(storage.load(&bundle.receipt.row.key), "storage.bundle.load")?
            == Some(bundle.receipt.row.clone()),
        "storage.bundle.row",
    )?;
    check(
        observe(
            storage.receipt(&bundle.receipt.identity),
            "storage.bundle.receipt.read",
        )? == Some(bundle.receipt.clone()),
        "storage.bundle.receipt",
    )?;
    check(facts.counts == [1, 2, 2, 2], "storage.bundle.counts")?;
    check(
        facts.events == [previous.row.clone(), bundle.receipt.row.clone()],
        "storage.bundle.events",
    )?;
    let effects: Vec<_> = bundle
        .effects
        .iter()
        .cloned()
        .map(|i| (bundle.receipt.identity.clone(), i))
        .collect();
    check(facts.effects == effects, "storage.bundle.effects")
}
