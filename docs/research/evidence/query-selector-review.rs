use rom_query_planning_prototype::{selector::*, *};
use std::cell::Cell;
struct Faults {
    count: Cell<usize>,
    planning: Cell<usize>,
    snapshots: Cell<usize>,
    native: Cell<usize>,
    fail_plan: bool,
}
impl PlannerAdapter for Faults {
    fn row_count(&self) -> Result<usize> {
        self.count.set(self.count.get() + 1);
        Ok(0)
    }
    fn epoch(&self) -> u64 {
        0
    }
    fn core_cost(&self, _: &Plan) -> Cost {
        Cost {
            setup: 10,
            ..Default::default()
        }
    }
    fn estimate(&self, p: &Plan, _: usize) -> Result<Option<Candidate>> {
        self.planning.set(self.planning.get() + 1);
        if self.fail_plan {
            Err("optional planning failure".into())
        } else {
            Ok(Some(Candidate {
                plan: p.clone(),
                epoch: 0,
                exact: true,
                cost: Cost::default(),
            }))
        }
    }
    fn snapshot(&self, _: usize) -> Result<Read> {
        self.snapshots.set(self.snapshots.get() + 1);
        Ok(Read::default())
    }
    fn native(&self, _: &Plan, _: usize, _: usize) -> Result<Read> {
        self.native.set(self.native.get() + 1);
        Err("execution failed".into())
    }
}
fn faults(fail_plan: bool) -> Faults {
    Faults {
        count: Cell::new(0),
        planning: Cell::new(0),
        snapshots: Cell::new(0),
        native: Cell::new(0),
        fail_plan,
    }
}
fn main() -> Result<()> {
    let rows = dataset(1000);
    let db = Sqlite::new(&rows, true)?;
    let plan = Plan {
        filters: vec![Predicate::AmountGe(990)],
        order: vec![Order(Field::Amount, true)],
    };
    let stats = Statistics::from_rows(&rows, db.generation());
    let mut stale = stats;
    stale.epoch += 1;
    for s in [stats, stale] {
        let error = execute(&SqlitePlanner::new(&db, s), &plan, true, 1, 10)
            .err()
            .expect("common admission must fail");
        assert_eq!(error.to_string(), "collection budget exhausted");
    }
    println!("fresh and stale reject identically at common collection bound");
    let mut passed = 0;
    for indexed in [false, true] {
        let db = Sqlite::new(&rows, indexed)?;
        let adapter = SqlitePlanner::new(&db, Statistics::from_rows(&rows, db.generation()));
        for threshold in [0, 1, 10, 990, 999, 1000, u64::MAX] {
            for order in [
                vec![],
                vec![Order(Field::Amount, false)],
                vec![Order(Field::Amount, true), Order(Field::Title, false)],
                vec![Order(Field::Note, true)],
            ] {
                for extra in [
                    None,
                    Some(Predicate::TitleEq(" title-000 ".into())),
                    Some(Predicate::NoteEq(Note::Null)),
                ] {
                    let mut p = Plan {
                        filters: vec![Predicate::AmountGe(threshold)],
                        order: order.clone(),
                    };
                    if let Some(x) = extra {
                        p.filters.push(x);
                    }
                    let normalized = normalize(p.clone())?;
                    let result = execute(&adapter, &p, true, 20, 1000)?;
                    assert_eq!(result.read.rows, oracle(&rows, &normalized, None, 20).rows);
                    passed += 1;
                }
            }
        }
    }
    println!("{passed} varied normalized query comparisons passed");
    let f = faults(false);
    assert!(execute(&f, &plan, false, 1, 1).is_err());
    assert_eq!(f.count.get(), 0);
    assert_eq!(f.planning.get(), 0);
    let f = faults(true);
    assert_eq!(
        execute(&f, &Plan::default(), true, 1, 1)?.strategy,
        Strategy::Core
    );
    assert_eq!(f.snapshots.get(), 1);
    assert_eq!(f.native.get(), 0);
    let f = faults(false);
    assert!(execute(&f, &Plan::default(), true, 1, 1).is_err());
    assert_eq!(f.snapshots.get(), 0);
    assert_eq!(f.native.get(), 1);
    let mut db = Sqlite::new(&rows, true)?;
    let stats = Statistics::from_rows(&rows, db.generation());
    db.mutate_amount(&rows[0].id, u64::MAX)?;
    let p = Plan {
        filters: vec![Predicate::AmountGe(u64::MAX)],
        order: vec![Order(Field::Amount, true)],
    };
    let r = execute(&SqlitePlanner::new(&db, stats), &p, true, 1, 1000)?;
    assert_eq!(r.strategy, Strategy::Core);
    assert_eq!(r.read.rows[0].amount, u64::MAX);
    println!(
        "authorization precedes adapter; planning failure falls back; execution failure does not; mutation invalidates estimates"
    );
    Ok(())
}
