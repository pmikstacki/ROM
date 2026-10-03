use rom_query_planning_prototype::*;
use std::{hint::black_box, time::Instant};
fn construction() -> Result<()> {
    let wire = serde_json::json!({"filters":[{"field":"amount","op":"ge","value":9000},{"field":"title","op":"eq","value":" title-042 "}],"order":[{"field":"amount","descending":true},{"field":"title","descending":false}]});
    type Constructor = fn(&serde_json::Value) -> Plan;
    let mut constructors: Vec<(&str, Constructor)> = Vec::new();
    #[cfg(feature = "generated")]
    constructors.push(("generated", |_| {
        generated::InventoryQuery::new()
            .amount_ge(black_box(9000))
            .title_equals(black_box(" title-042 "))
            .order_amount_desc()
            .order_title_asc()
            .finish()
    }));
    #[cfg(feature = "hybrid")]
    constructors.push(("hybrid", |_| {
        hybrid::Builder::new()
            .filter(hybrid::AMOUNT.ge(black_box(9000)))
            .filter(hybrid::TITLE.eq(black_box(" title-042 ")))
            .order(hybrid::AMOUNT.desc())
            .order(hybrid::TITLE.asc())
            .finish()
    }));
    #[cfg(feature = "runtime")]
    constructors.push(("runtime_from_value", |v| {
        compile_wire("inventory", black_box(v)).unwrap()
    }));
    let reference = constructors[0].1(&wire);
    for (_, make) in &constructors {
        assert_eq!(make(&wire), reference);
    }
    for sample in 0..31 {
        for offset in 0..constructors.len() {
            let (name, make) = constructors[(sample + offset) % constructors.len()];
            let started = Instant::now();
            for _ in 0..10000 {
                black_box(make(black_box(&wire)));
            }
            println!(
                "construction,{name},{sample},0,0,{:.3},0,0,0",
                started.elapsed().as_nanos() as f64 / 10000.0
            );
        }
    }
    Ok(())
}
fn execution() -> Result<()> {
    let scratch = std::path::PathBuf::from(
        std::env::var("QUERY_PROBE_SCRATCH")
            .unwrap_or_else(|_| "/var/tmp/rom-query-probe-data".into()),
    );
    std::fs::create_dir_all(&scratch)?;
    for n in [1000, 10000, 100000] {
        let rows = dataset(n);
        let indexed = Sqlite::new(&rows, true)?;
        let unindexed = Sqlite::new(&rows, false)?;
        let redb = Redb::new(&scratch.join(format!("rows-{n}.redb")), &rows)?;
        let cases = [
            (
                "selective",
                Plan {
                    filters: vec![Predicate::AmountGe((n - 100) as u64)],
                    order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
                },
            ),
            (
                "broad",
                Plan {
                    filters: vec![Predicate::AmountGe(0)],
                    order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
                },
            ),
            (
                "title_sort",
                Plan {
                    filters: vec![Predicate::AmountGe((n - 100) as u64)],
                    order: vec![Order(Field::Title, false)],
                },
            ),
            (
                "none",
                Plan {
                    filters: vec![Predicate::AmountGe(n as u64)],
                    order: vec![Order(Field::Amount, true)],
                },
            ),
        ];
        if n == 1000 {
            eprintln!(
                "SQLite {} / redb 4.3.0 / exact-u64 BLOB order / 10% row visibility",
                indexed.version()
            );
            for (case, plan) in &cases {
                eprintln!(
                    "{case}: indexed {:?}; unindexed {:?}",
                    indexed.explain(plan)?,
                    unindexed.explain(plan)?
                );
            }
        }
        for (case, plan) in cases {
            let expected = oracle(&rows, &plan, None, 20);
            for sample in 0..9 {
                for offset in 0..4 {
                    let strategy = (sample + offset) % 4;
                    let started = Instant::now();
                    let (name, result) = match strategy {
                        0 => ("sqlite_indexed", indexed.query(&plan, None, 20, n + 1)?),
                        1 => ("sqlite_unindexed", unindexed.query(&plan, None, 20, n + 1)?),
                        2 => {
                            let raw = indexed.snapshot(n + 1)?;
                            let mut selected = oracle(&raw.rows, &plan, None, 20);
                            selected.candidates = raw.candidates;
                            selected.decoded_bytes = raw.decoded_bytes;
                            selected.vm_steps = raw.vm_steps;
                            ("sqlite_snapshot", selected)
                        }
                        _ => ("redb_scan", redb.query(&plan, None, 20, n + 1)?),
                    };
                    let ns = started.elapsed().as_nanos();
                    assert_eq!(result.rows, expected.rows, "{n}/{case}/{name}");
                    black_box(&result);
                    println!(
                        "execution,{name}_{case},{sample},{n},{},{ns},{},{},{}",
                        result.rows.len(),
                        result.candidates,
                        result.decoded_bytes,
                        result.vm_steps
                    );
                }
            }
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    println!("phase,strategy,sample,rows,returned,ns,candidates,decoded_bytes,sqlite_vm_steps");
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    if mode == "all" || mode == "construction" {
        construction()?;
    }
    if mode == "all" || mode == "execution" {
        execution()?;
    }
    if mode == "all" || mode == "writes" {
        writes()?;
    }
    Ok(())
}

fn writes() -> Result<()> {
    let mut rows = dataset(10000);
    let mut databases = [Sqlite::new(&rows, false)?, Sqlite::new(&rows, true)?];
    for sample in 0..31 {
        for offset in 0..2 {
            let strategy = (sample + offset) % 2;
            let name = if strategy == 0 {
                "sqlite_unindexed"
            } else {
                "sqlite_indexed"
            };
            let started = Instant::now();
            for i in 0..100 {
                let index = (sample * 100 + i) % rows.len();
                let amount = u64::MAX - (sample * 100 + i) as u64;
                databases[strategy].mutate_amount(&rows[index].id, black_box(amount))?;
            }
            println!(
                "write,{name},{sample},10000,0,{:.3},0,0,0",
                started.elapsed().as_nanos() as f64 / 100.0
            );
        }
        for i in 0..100 {
            let index = (sample * 100 + i) % rows.len();
            rows[index].amount = u64::MAX - (sample * 100 + i) as u64;
        }
        let plan = Plan {
            order: vec![Order(Field::Amount, true)],
            ..Plan::default()
        };
        for db in &databases {
            assert_eq!(
                db.query(&plan, None, 20, 10001)?.rows,
                oracle(&rows, &plan, None, 20).rows
            );
        }
    }
    assert_eq!(databases[0].journal()?, databases[1].journal()?);
    assert_eq!(databases[0].journal()?.len(), 3100);
    Ok(())
}
