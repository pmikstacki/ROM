//! Standalone selection experiment. Costs are hints, never semantic authority.
use crate::{Plan, Predicate, Read, Redb, Result, Row, Sqlite, authorize, normalize, oracle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    Core,
    Native,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Cost {
    pub setup: u64,
    pub scan: u64,
    pub decode: u64,
    pub sort: u64,
}
impl Cost {
    pub fn total(self) -> Option<u64> {
        self.setup
            .checked_add(self.scan)?
            .checked_add(self.decode)?
            .checked_add(self.sort)
    }
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub plan: Plan,
    pub epoch: u64,
    pub exact: bool,
    pub cost: Cost,
}
pub fn choose(plan: &Plan, epoch: u64, core: Cost, native: Option<&Candidate>) -> Strategy {
    let Some(candidate) = native.filter(|c| c.exact && c.epoch == epoch && &c.plan == plan) else {
        return Strategy::Core;
    };
    match (core.total(), candidate.cost.total()) {
        (Some(a), Some(b)) if b < a => Strategy::Native,
        _ => Strategy::Core,
    }
}

/// An adapter owns planner inspection and native execution, never domain rules.
pub trait PlannerAdapter {
    fn row_count(&self) -> Result<usize>;
    fn epoch(&self) -> u64;
    fn core_cost(&self, plan: &Plan) -> Cost;
    fn estimate(&self, plan: &Plan, budget: usize) -> Result<Option<Candidate>>;
    fn snapshot(&self, budget: usize) -> Result<Read>;
    fn native(&self, plan: &Plan, limit: usize, budget: usize) -> Result<Read>;
}
pub struct SelectedRead {
    pub strategy: Strategy,
    pub read: Read,
}
pub fn execute(
    adapter: &impl PlannerAdapter,
    plan: &Plan,
    may_sort_amount: bool,
    limit: usize,
    budget: usize,
) -> Result<SelectedRead> {
    if limit == 0 || limit > budget {
        return Err("invalid result limit".into());
    }
    let plan = normalize(plan.clone())?;
    authorize(&plan, may_sort_amount)?;
    // One admitted-collection profile for all choices, independent of estimates.
    if adapter.row_count()? > budget {
        return Err("collection budget exhausted".into());
    }
    // An optional planning failure selects the reference path. Execution failures
    // are not silently retried or interpreted as successful empty results.
    let candidate = adapter.estimate(&plan, budget).ok().flatten();
    let strategy = choose(
        &plan,
        adapter.epoch(),
        adapter.core_cost(&plan),
        candidate.as_ref(),
    );
    let read = match strategy {
        Strategy::Core => {
            let raw = adapter.snapshot(budget)?;
            let mut result = oracle(&raw.rows, &plan, None, limit);
            result.candidates = raw.candidates;
            result.decoded_bytes = raw.decoded_bytes;
            result.vm_steps = raw.vm_steps;
            result
        }
        Strategy::Native => adapter.native(&plan, limit, budget)?,
    };
    Ok(SelectedRead { strategy, read })
}

#[derive(Clone, Copy)]
pub struct Statistics {
    pub epoch: u64,
    pub rows: u64,
    pub min_amount: u64,
    pub max_amount: u64,
}
impl Statistics {
    /// Fixture collection at dataset creation. Not an extra scan per query.
    pub fn from_rows(rows: &[Row], epoch: u64) -> Self {
        Self {
            epoch,
            rows: rows.len() as u64,
            min_amount: rows.iter().map(|r| r.amount).min().unwrap_or(0),
            max_amount: rows.iter().map(|r| r.amount).max().unwrap_or(0),
        }
    }
    fn matches(self, plan: &Plan) -> u64 {
        let lower = plan
            .filters
            .iter()
            .filter_map(|p| {
                if let Predicate::AmountGe(n) = p {
                    Some(*n)
                } else {
                    None
                }
            })
            .max();
        match lower {
            Some(n) if n > self.max_amount => 0,
            Some(n) if n > self.min_amount => {
                // Distribution guess, not scalar comparison or result filtering.
                let width = u128::from(self.max_amount) - u128::from(self.min_amount) + 1;
                let remain = u128::from(self.max_amount) - u128::from(n) + 1;
                ((u128::from(self.rows) * remain).div_ceil(width)) as u64
            }
            _ => self.rows,
        }
    }
}
fn sort_units(rows: u64, weight: u64) -> u64 {
    rows.saturating_mul(u64::from(rows.max(1).ilog2()) + 1)
        .saturating_mul(weight)
}

/// Version-pinned SQLite EXPLAIN bridge for this experiment only. Unknown plan
/// shapes return no estimate. Production use needs a versioned stable adapter.
pub struct SqlitePlanner<'a> {
    db: &'a Sqlite,
    stats: Statistics,
}
impl<'a> SqlitePlanner<'a> {
    pub fn new(db: &'a Sqlite, stats: Statistics) -> Self {
        Self { db, stats }
    }
}
impl PlannerAdapter for SqlitePlanner<'_> {
    fn row_count(&self) -> Result<usize> {
        self.db.row_count()
    }
    fn epoch(&self) -> u64 {
        self.db.generation()
    }
    fn core_cost(&self, plan: &Plan) -> Cost {
        Cost {
            setup: 100,
            scan: self.stats.rows.saturating_mul(10),
            decode: self.stats.rows.saturating_mul(200),
            sort: sort_units(self.stats.matches(plan), 2),
        }
    }
    fn estimate(&self, plan: &Plan, budget: usize) -> Result<Option<Candidate>> {
        if self.stats.epoch != self.epoch() || self.db.version() != "3.53.2" {
            return Ok(None);
        }
        let details = self.db.explain_with_budget(plan, budget)?;
        let Some((indexed, needs_sort)) = classify_sqlite_plan(&details) else {
            return Ok(None);
        };
        let matching = self.stats.matches(plan);
        let scan = if indexed {
            matching.saturating_add(u64::from(self.stats.rows.max(1).ilog2()) + 1)
        } else {
            self.stats.rows
        };
        Ok(Some(Candidate {
            plan: plan.clone(),
            epoch: self.epoch(),
            exact: true,
            cost: Cost {
                setup: 1000,
                scan: scan.saturating_mul(10),
                decode: matching.saturating_mul(200),
                sort: if needs_sort {
                    sort_units(matching, 40)
                } else {
                    0
                },
            },
        }))
    }
    fn snapshot(&self, budget: usize) -> Result<Read> {
        self.db.snapshot(budget)
    }
    fn native(&self, plan: &Plan, limit: usize, budget: usize) -> Result<Read> {
        self.db.query(plan, None, limit, budget)
    }
}

/// A transactional key/value backend has no SQL planner to wrap. It supplies the
/// same exact core path rather than making the application choose another query.
pub struct RedbPlanner<'a> {
    pub db: &'a Redb,
    pub epoch: u64,
}
impl PlannerAdapter for RedbPlanner<'_> {
    fn row_count(&self) -> Result<usize> {
        self.db.row_count()
    }
    fn epoch(&self) -> u64 {
        self.epoch
    }
    fn core_cost(&self, _: &Plan) -> Cost {
        Cost::default()
    }
    fn estimate(&self, _: &Plan, _: usize) -> Result<Option<Candidate>> {
        Ok(None)
    }
    fn snapshot(&self, budget: usize) -> Result<Read> {
        self.db.snapshot(budget)
    }
    fn native(&self, _: &Plan, _: usize, _: usize) -> Result<Read> {
        Err("no native query planner".into())
    }
}

pub fn classify_sqlite_plan(details: &[String]) -> Option<(bool, bool)> {
    let mut access = None;
    let mut sort = false;
    for node in details {
        let indexed = match node.as_str() {
            "SEARCH items USING INDEX amount_order (amount>?)"
            | "SCAN items USING INDEX amount_order" => Some(true),
            "SCAN items" | "SCAN items USING INDEX sqlite_autoindex_items_1" => Some(false),
            "USE TEMP B-TREE FOR ORDER BY"
            | "USE TEMP B-TREE FOR LAST TERM OF ORDER BY"
            | "USE TEMP B-TREE FOR RIGHT PART OF ORDER BY" => {
                sort = true;
                None
            }
            _ => return None,
        };
        if let Some(indexed) = indexed
            && access.replace(indexed).is_some()
        {
            return None;
        }
    }
    access.map(|indexed| (indexed, sort))
}
