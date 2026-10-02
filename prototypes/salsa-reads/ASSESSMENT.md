# Salsa pure-read assessment

## Verdict

Salsa is worth a bounded follow-up for expensive, repeatedly requested derived reads with known input publication rules. This probe establishes dependency reuse and invalidation, not service-level performance. Keep Salsa behind ROM's Resource/read interface: application authors should write ordinary resource fields and pure reads; adapter internals can handle projection, identity and invalidation. Whether ROM can generate that bridge without a second field schema is still unanswered.

It is not an action memoizer, durable command ledger, global resource authority, queryable storage replacement, or delivery engine. The authoritative snapshot remains separate, and both implementations consume its contents. No database driver, network service, async scheduling, changefeed, persistence, tenant isolation or generic registry was implemented here.

## Reproducible evidence

Executed 2026-10-02 on the native NixOS `rom-dev` container, Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`. The worktree started at ROM commit `90a87dc`; the experiment lives on `prototype/salsa-reads`. Source and lockfile are the reproducibility inputs; [checks.txt](results/checks.txt) records the implementer's run. The coordinator independently reran formatting, Clippy, all eight tests and the demo successfully on the same toolchain.

`cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` (8 tests), and `cargo run --locked` all passed. Empty binary/doc-test suites are not counted as behavioral tests. The demo asserts equality with plain recomputation at every shown stage and on 1,000 unchanged iterations. Tests independently assert expected IDs/counts and exact query-body execution deltas.

| Sequential demo stage | IDs | Cumulative membership/list/count executions |
| --- | --- | --- |
| Cold, 3 tasks | `[1]` | `3 / 1 / 1` |
| 1,000 unchanged ID and count reads | `[1]` | `3 / 1 / 1` |
| Task 1 title edit | `[1]` | `3 / 1 / 1` |
| Reopen task 2 | `[1, 2]` | `4 / 2 / 2` |
| Disable actor | `[]` | `7 / 3 / 3` |
| Enable and switch actor to 8 | `[3]` | `10 / 4 / 4` |

A separate test changes hidden task 3's owner from 8 to 9. One membership body executes again, returns false again, and neither list nor count body executes again. This is evidence of equal-result propagation in this graph. Insertion, deletion and reinsertion match the plain oracle; reinsertion creates a new internal input identity. Policy revoke is immediate only after the authoritative policy snapshot has been applied. Actor switching is sequential, not concurrent multi-actor isolation evidence.

The untracked-external-policy test first returns `[1]`, flips its external permission flag to false, then incorrectly returns `[1]` again. The expected bug is the control's passing condition. Salsa cannot discover omitted dependencies. The correct read uses explicit actor/policy inputs; real roles, tenancy, grants, policy versions and time-dependent permissions would need equivalent treatment or an authoritative request-time check outside the memo.

These are exact work-count observations on three tasks. No wall-clock benchmark, allocation measurement, memory plateau test, cancellation/concurrency stress or distributed invalidation test was run. One avoided body is not a measured latency improvement: Salsa lookup, validation, interning, owned-result cloning and synchronization still cost work. A large collection edit still scans the list body even if only one per-task membership changed. This workload is intentionally not compared to the separate Moka dedup probe.

## What the adapter must own

`apply` receives a complete committed snapshot under exclusive mutable access and publishes no read until all changed fields are installed. It preserves per-resource handles, compares each value before calling its setter, updates the catalog for creation/deletion, and updates policy/actor facts. It records the authoritative revision separately, rejecting non-increasing revisions. A no-op source revision performs zero setters and preserves all memos. Multiple changed fields can cause multiple Salsa revisions during one source commit; these revision domains must not be confused.

Full reconciliation costs O(number of source tasks) per publication and clones changed titles. A real adapter needs either this complete snapshot boundary or complete, ordered committed deltas, plus catch-up/rebuild after missing events, rollback-safe publication, and explicit freshness behavior while lagging. The caller currently supplies authoritative snapshots; the prototype cannot detect a forged revision or falsely labeled snapshot. No rollback-on-panic or distributed transaction guarantee is claimed. Concurrent readers would require a deliberately designed snapshot/ownership boundary.

Fine-grained tracking depends on the declared model: per-task fields and a collection-membership input. Wrapping all task fields into a single input value would broaden invalidation. A derived read returning titles would legitimately depend on titles. Arbitrary Rust code reading sockets, clocks, environment variables or an external store cannot automatically receive sound invalidation. A production design should make pure read registration explicit, retain existing action authorization, and reject or conservatively handle unsupported dependencies.

## Version and primary-source findings

Resolved and pinned `salsa = "=0.28.5"` with macros/inventory features and default features disabled. Cargo downloaded Salsa, salsa-macros and salsa-macro-rules 0.28.5. The [official sparse registry entry](https://index.crates.io/sa/ls/salsa) marks 0.28.5 non-yanked, published 2026-09-24, MSRV 1.88; 0.26 search results are older. Cargo.lock records transitive versions and checksums. Beskid's prior use motivates this experiment; no Beskid workload or version-equivalence claim is made.

Salsa records field/query dependencies and memoizes deterministic functions; equal recomputed results can preserve their earlier changed-at revision. Inputs live until their database is dropped. Default memos persist until their key is removed or the database is dropped; LRU evicts results at revision boundaries but retains memo entries. Thus deleting our BTreeMap handles does not reclaim input storage. The prototype has no LRU or rotation and can retain deleted-resource data until the adapter is dropped. A production scope needs explicit lifetime/cardinality budgets and measured reclamation behavior; a result-count cap alone does not bound total memory. [Salsa 0.28.5 crate documentation and source](https://docs.rs/salsa/0.28.5/salsa/), [tracked-function options](https://docs.rs/salsa/0.28.5/salsa/attr.tracked.html).

Input setters start revisions even for equal values unless the caller compares first, which explains the adapter's comparisons. Salsa durability describes change likelihood, not crash durability or authoritative commit status. [Input documentation](https://docs.rs/salsa/0.28.5/salsa/attr.input.html), [durability documentation](https://docs.rs/salsa/0.28.5/salsa/struct.Durability.html).

Cancellation is cooperative: queries can explicitly check long-running loops. Mutating/cancellation operations can wait for other database handles; retaining a handle on the same thread can deadlock. `report_untracked_read` forces reexecution in the next revision, not immediate invalidation when external state changes within one revision. It cannot by itself make an external policy check fresh. This probe neither clones live database handles nor tests cancellation; production integration must handle cancellation/retry at the read boundary and keep irreversible effects outside tracked functions. [Database API](https://docs.rs/salsa/0.28.5/salsa/trait.Database.html).

## Useful next pattern

Keep ordinary `Task` authorship and expose an opt-in pure derived read. Generate or centralize one projection bridge from the accepted Resource contract, instead of asking humans to maintain these duplicate `Task`/`TaskFact` schemas. Measure an actually expensive graph with realistic mutation/read ratios, including source publication and owned response costs. Only then choose query scope, memory limits and synchronous/async scheduling. The present evidence supports the mechanism and identifies the contract burden; it does not select Salsa as ROM's default cache.
