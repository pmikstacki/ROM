use rom_query_planning_prototype::{selector::*, *};
use std::{hint::black_box, time::Instant};
fn main() -> Result<()> {
    println!("rows,indexed,case,sample,mode,choice,ns,candidates,vm_steps");
    for n in [1000, 10000, 100000] {
        let rows = dataset(n);
        for indexed in [false, true] {
            let db = Sqlite::new(&rows, indexed)?;
            let adapter = SqlitePlanner::new(&db, Statistics::from_rows(&rows, db.generation()));
            for (case, threshold) in [
                ("broad", 0),
                ("selective", n as u64 - 100),
                ("none", n as u64),
            ] {
                let plan = Plan {
                    filters: vec![Predicate::AmountGe(threshold)],
                    order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
                };
                let expected = oracle(&rows, &plan, None, 20);
                for sample in 0..11 {
                    for offset in 0..3 {
                        let mode = (sample + offset) % 3;
                        let start = Instant::now();
                        let (label, choice, read) = match mode {
                            0 => {
                                let result = execute(&adapter, &plan, true, 20, n)?;
                                ("selected", result.strategy, result.read)
                            }
                            1 => {
                                let raw = db.snapshot(n)?;
                                let mut result = oracle(&raw.rows, &plan, None, 20);
                                result.candidates = raw.candidates;
                                result.vm_steps = raw.vm_steps;
                                ("core", Strategy::Core, result)
                            }
                            _ => ("native", Strategy::Native, db.query(&plan, None, 20, n)?),
                        };
                        let ns = start.elapsed().as_nanos();
                        assert_eq!(read.rows, expected.rows);
                        black_box(&read);
                        println!(
                            "{n},{indexed},{case},{sample},{label},{choice:?},{ns},{},{}",
                            read.candidates, read.vm_steps
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
