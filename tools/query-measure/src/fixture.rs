//! Deterministic, reusable Resource and dataset contract for current and baseline runs.
use rom::{Actor, Definition, Resource, StorageLimits};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    Independent,
    Skewed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Dataset {
    pub distribution: Distribution,
    pub seed: u64,
}

#[derive(Clone, Debug, Resource)]
#[resource(name = "query-measure-rows")]
pub struct MeasurementRow {
    pub amount: u64,
    pub category: String,
    pub active: bool,
    pub title: String,
    pub payload: Vec<u64>,
}

pub fn actor() -> Actor {
    Actor::trusted("query-measure", "owner")
}
pub fn definition() -> Definition<MeasurementRow> {
    MeasurementRow::definition()
        .policy(|_, _, _| true)
        .read_policy(|_| true)
        .allow_all_fields()
}
pub fn id(index: usize) -> String {
    format!("row-{index:08}")
}

/// Separate domain seeds keep category, activity and title independent of amount.
fn random(seed: u64, index: usize, domain: u64) -> u64 {
    let key = mix(seed ^ domain.wrapping_mul(0x9e3779b97f4a7c15));
    mix(key.wrapping_add(index as u64))
}
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub fn value(dataset: Dataset, index: usize, _size: usize) -> MeasurementRow {
    let category = random(dataset.seed, index, 1);
    MeasurementRow {
        amount: index as u64,
        category: if dataset.distribution == Distribution::Skewed && category % 10 < 9 {
            "hot".into()
        } else {
            format!("category-{:02}", category % 16)
        },
        active: random(dataset.seed, index, 2).is_multiple_of(2),
        title: format!("title-{:02}-é", random(dataset.seed, index, 3) % 32),
        payload: (0..8)
            .map(|offset| random(dataset.seed, index, 4 + offset))
            .collect(),
    }
}

/// The same bounded journal policy is used in indexed and pre-index write trials.
pub fn storage_limits(size: usize) -> StorageLimits {
    StorageLimits {
        journal_rows: 32,
        receipts: StorageLimits::default()
            .receipts
            .max(size.saturating_add(129)),
        ..StorageLimits::default()
    }
}
