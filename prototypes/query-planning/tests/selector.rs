use rom_query_planning_prototype::{selector::*, *};

#[test]
fn unknown_planner_nodes_or_similar_index_names_are_not_accepted() {
    assert_eq!(
        classify_sqlite_plan(&["SCAN items".into()]),
        Some((false, false))
    );
    assert!(classify_sqlite_plan(&["SCAN items".into(), "UNKNOWN".into()]).is_none());
    assert!(classify_sqlite_plan(&["SCAN items USING INDEX amount_order_extra".into()]).is_none());
}

#[test]
fn estimates_cannot_change_whole_collection_admission() {
    let rows = dataset(1000);
    let db = Sqlite::new(&rows, true).unwrap();
    let plan = Plan {
        filters: vec![Predicate::AmountGe(990)],
        order: vec![Order(Field::Amount, true)],
    };
    let fresh = Statistics::from_rows(&rows, db.generation());
    let mut stale = fresh;
    stale.epoch += 1;
    for stats in [fresh, stale] {
        let adapter = SqlitePlanner::new(&db, stats);
        assert!(execute(&adapter, &plan, true, 1, 10).is_err());
        assert_eq!(
            execute(&adapter, &plan, true, 1, 1000).unwrap().read.rows,
            oracle(&rows, &plan, None, 1).rows
        );
    }
}

#[test]
fn estimates_are_optional_versioned_and_query_bound() {
    let plan = Plan::default();
    let baseline = Cost {
        setup: 1,
        scan: 100,
        decode: 100,
        sort: 0,
    };
    let native = Candidate {
        plan: plan.clone(),
        epoch: 2,
        exact: true,
        cost: Cost::default(),
    };
    assert_eq!(choose(&plan, 2, baseline, Some(&native)), Strategy::Native);
    assert_eq!(choose(&plan, 3, baseline, Some(&native)), Strategy::Core);
    assert_eq!(choose(&plan, 2, baseline, None), Strategy::Core);
    let mut changed = native.clone();
    changed.exact = false;
    assert_eq!(choose(&plan, 2, baseline, Some(&changed)), Strategy::Core);
    changed = native.clone();
    changed.plan.filters.push(Predicate::AmountGe(5));
    assert_eq!(choose(&plan, 2, baseline, Some(&changed)), Strategy::Core);
    changed = native;
    changed.cost = Cost {
        setup: u64::MAX,
        scan: 1,
        ..Cost::default()
    };
    assert_eq!(choose(&plan, 2, baseline, Some(&changed)), Strategy::Core);
}

#[test]
fn sqlite_planner_bridge_preserves_results_for_both_storage_profiles() {
    let rows = dataset(1000);
    for indexed in [false, true] {
        let db = Sqlite::new(&rows, indexed).unwrap();
        let adapter = SqlitePlanner::new(&db, Statistics::from_rows(&rows, db.generation()));
        for threshold in [0, 990, 1001] {
            let plan = Plan {
                filters: vec![Predicate::AmountGe(threshold)],
                order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
            };
            let actual = execute(&adapter, &plan, true, 20, 1000).unwrap();
            assert_eq!(actual.read.rows, oracle(&rows, &plan, None, 20).rows);
            if indexed || threshold > 0 {
                assert_eq!(actual.strategy, Strategy::Native);
            } else {
                assert_eq!(actual.strategy, Strategy::Core);
            }
        }
    }
}

#[test]
fn stale_statistics_fall_back_without_changing_mutation_or_events() {
    let mut rows = dataset(20);
    let mut db = Sqlite::new(&rows, true).unwrap();
    let stats = Statistics::from_rows(&rows, db.generation());
    db.mutate_amount(&rows[0].id, u64::MAX).unwrap();
    rows[0].amount = u64::MAX;
    let adapter = SqlitePlanner::new(&db, stats);
    let plan = Plan {
        filters: vec![Predicate::AmountGe(u64::MAX)],
        order: vec![Order(Field::Amount, true)],
    };
    let actual = execute(&adapter, &plan, true, 20, 20).unwrap();
    assert_eq!(actual.strategy, Strategy::Core);
    assert_eq!(actual.read.rows, oracle(&rows, &plan, None, 20).rows);
    assert_eq!(db.journal().unwrap().len(), 1);
    assert!(execute(&adapter, &plan, false, 20, 20).is_err());
    assert!(execute(&adapter, &plan, true, 0, 20).is_err());
}

#[test]
fn incorrect_statistics_can_change_choice_but_not_results() {
    let rows = dataset(1000);
    let db = Sqlite::new(&rows, false).unwrap();
    let mut stats = Statistics::from_rows(&rows, db.generation());
    stats.rows = 1;
    stats.max_amount = 0;
    let adapter = SqlitePlanner::new(&db, stats);
    let plan = Plan {
        filters: vec![Predicate::AmountGe(0)],
        order: vec![Order(Field::Amount, true)],
    };
    assert_eq!(
        execute(&adapter, &plan, true, 20, 1000).unwrap().read.rows,
        oracle(&rows, &plan, None, 20).rows
    );
}

#[test]
fn backend_without_a_native_planner_uses_same_query_and_policy() {
    let path =
        std::path::PathBuf::from(format!("/var/tmp/rom-selector-{}.redb", std::process::id()));
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    let rows = dataset(50);
    let db = Redb::new(&path, &rows).unwrap();
    let adapter = RedbPlanner { db: &db, epoch: 1 };
    let plan = Plan {
        filters: vec![Predicate::AmountGe(9)],
        order: vec![Order(Field::Amount, true)],
    };
    let result = execute(&adapter, &plan, true, 4, 50).unwrap();
    assert_eq!(result.strategy, Strategy::Core);
    assert_eq!(result.read.rows, oracle(&rows, &plan, None, 4).rows);
    assert!(execute(&adapter, &plan, false, 4, 50).is_err());
    drop(db);
    std::fs::remove_file(path).unwrap();
}
