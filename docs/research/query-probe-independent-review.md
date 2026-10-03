# Independent query-prototype review

Date: 2026-10-03. Reviewed immutable prototype
`b7cfdb1907bdc30be455fd98f7e3b958c5ce87ce`, branch `codex/prototype-query-planning`, directory
`prototypes/query-planning`, based on ROM
`64240cf99e4632378e62db91f3bf151f2d306e6f`.

**Result:** no outstanding P1/P2 finding within the final prototype's stated
scope. A zero-limit correctness defect was independently reproduced and fixed
before the reviewed commit. The experiment provides useful evidence for
separating query authoring from physical execution; it does not establish a
maintained ROM query API, authorization implementation or general cost optimizer.

## Review and independently executed checks

Source inspection covered the three construction profiles, canonical Plan,
SQLite translation/anchor ordering, raw snapshot evaluator, persisted redb scan,
conformance fixtures, benchmark runner, build-cost script and final README/report.

I ran the eight conformance tests at the frozen SHA in native `rom-dev`, Rust
1.99.0, using the existing `/var/tmp/rom-query-probe-target`. All passed. I built
the exact source's release library and linked two external Rust fixtures against
it; no DataFusion or other large research dependency was rebuilt.

An independent cross-product compared complete SQLite results with the Rust
oracle for **3,888 vectors**: all pairs of ascending/descending amount/title/note
orders, four predicate profiles, no anchor plus each of eight row anchors, and
limits 1, 2 and 99. Rows included `u64::MAX`, adjacent high unsigned values,
values above 2^53 and i64::MAX, embedded NUL, composed/decomposed Unicode, equal
sort keys, missing/null/value notes and one invisible row. All passed.

The independent zero-limit reproduction initially returned ten visible rows
from a 100-row fixture while the oracle returned zero. The fix returns early
for zero and adds a conformance assertion. Relinking against the final library
produced `zero_limit sqlite=0 oracle=0`; the 3,888-vector fixture remained green.

I also executed a no-match query with a zero decoded-candidate budget: it
returned no rows and no decoded candidates after **410 SQLite VM steps** over
100 source rows. This confirms the documented limitation: the candidate budget
is not a database scan/CPU budget. The final report explicitly states it.

Commands for the frozen prototype checks performed by this reviewer:

```sh
cd prototypes/query-planning
CARGO_TARGET_DIR=/var/tmp/rom-query-probe-target CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true cargo test --locked
CARGO_TARGET_DIR=/var/tmp/rom-query-probe-target CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true cargo build --locked --release --lib
```

The independent fixture source below can be compiled with `rustc --edition 2024`,
`--extern rom_query_planning_prototype=<fresh rlib>` and
`-L dependency=<target>/release/deps`. Select the library from the exact build,
not an older feature-profile artifact in the shared target.

```rust
use rom_query_planning_prototype::*;
fn main() {
    let mut rows = Vec::new();
    for (i, (amount, title, note)) in [
        (0, "", Note::Missing), (u64::MAX, "é", Note::Null),
        (u64::MAX - 1, "e\u{301}", Note::Value("".into())),
        (1 << 53, "a\0b", Note::Value("a\0b".into())),
        ((1 << 53) + 1, "a", Note::Value("é".into())),
        (1 << 63, "a", Note::Missing), (0, "é", Note::Null),
        (u64::MAX, "é", Note::Null),
    ].into_iter().enumerate() {
        rows.push(Row { id: format!("{i}"), amount, title: title.into(),
            note, visible: i != 3 });
    }
    let db = Sqlite::new(&rows, true).unwrap();
    let orders = [Order(Field::Amount, false), Order(Field::Amount, true),
        Order(Field::Title, false), Order(Field::Title, true),
        Order(Field::Note, false), Order(Field::Note, true)];
    let mut checked = 0;
    for a in orders { for b in orders {
        for filters in [vec![], vec![Predicate::AmountGe((1 << 53) + 1)],
            vec![Predicate::NoteEq(Note::Null)], vec![Predicate::TitleEq("é".into())]] {
            let plan = Plan { filters, order: vec![a, b] };
            for anchor in std::iter::once(None).chain(rows.iter().map(Some)) {
                for limit in [1, 2, 99] {
                    assert_eq!(db.query(&plan, anchor, limit, 100).unwrap().rows,
                        oracle(&rows, &plan, anchor, limit).rows);
                    checked += 1;
                }
            }
        }
    }}
    assert_eq!(checked, 3888);
    assert!(db.query(&Plan::default(), None, 0, 100).unwrap().rows.is_empty());
}
```

## Fairness, cost and limits

The raw SQLite snapshot now materializes rows without first applying policy and
sorting, then runs the oracle once. The previously reported double-evaluation
bias is absent. The redb reopen test opens the existing database without
reinsertion and checks a changed committed payload. SQLite tests exercise actual
file reopen and a synthetic update-plus-journal transaction under three query
strategies; these remain distinct from ROM receipts/events.

I independently recalculated all reported construction, 100k-row execution and
write medians from committed `results/timings.csv`. They match the final report.
I did not rerun the full timing matrix or build-cost matrix, and those samples
remain implementer evidence. Construction starts from different representations:
native typed operands versus parsed JSON Value clone/deserialization. The report
states this and does not infer a universal runtime-compilation penalty.

Indexed SQLite wins deliberately compatible fixtures. Unindexed broad pushdown
is slower than raw snapshot evaluation, supporting case-dependent selection.
SQLite is in memory while redb is file-backed, caches are uncontrolled, and JSON
byte accounting adds asymmetric instrumentation work. Accordingly, there is no
valid general SQLite-versus-redb ranking, production latency promise or measured
allocation/RSS claim. VM-step evidence usefully exposes work hidden by candidate
counts; production admission cannot rely solely on returned-row limits.

Authorization is a synthetic visibility boolean plus a separate sort-permission
fixture; executors do not call the latter. Query operand trimming is not a
write/query custom-Field codec conformance test. Direct Plan construction and
unbound row anchors are explicitly outside a security boundary. The final README
acknowledges these limits. Maintained implementation must still verify real
normalization, predicate/order grants, anchor identity, current authority and
bounded work at its own interfaces.

Avoidable cost to investigate next: serializing every decoded SQLite row solely
to count bytes, per-query SQL preparation and full fallback materialization.
Removing accounting work requires a separately validated measurement path;
statement caching and fallback changes need equivalence tests. This review does
not recommend weakening semantic checks to obtain lower timings.
