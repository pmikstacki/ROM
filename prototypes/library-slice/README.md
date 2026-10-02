# Disposable reusable library slice

Question: can ROM own generic mutation, transactional state/events, filtered live results and authorization while an embedding application supplies only resource definitions, ordinary business functions and trusted host policy?

**Observed answer: yes for this deliberately small, single-runtime slice.** This is estimation evidence, not a production API or database selection. SQLite is a scratch persistence probe. No HTTP types or dependencies occur in the public API. This project uses a small declarative descriptor macro and a handwritten fluent builder; it does **not** prove a combined derive + Bon API.

## Run

From this directory, with Rust/Cargo 1.99:

```sh
./verify
```

From the host used for the experiment:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/library-slice/prototypes/library-slice && ./verify'
```

The command checks formatting, runs Clippy with warnings denied, runs tests and executes `examples/app.rs`. The example prints `open tasks: 1` then `after complete: 0`. Tests create uniquely named scratch SQLite directories and remove them afterward; the example uses an in-memory database. Nothing serves a port.

Direct dependencies are exact-pinned: Tokio 1.53.1, rusqlite 0.40.2, serde 1.0.229 and serde_json 1.0.151; the complete resolution is in Cargo.lock. Actual verification used rustc 1.99.0 / Cargo 1.99.0. An older compiler floor was not tested.

## Application surface

A readable declaration registers fields and names ordinary action functions:

```rust
resource! {
    Task("tasks") {
        title: String,
        done: bool,
    } actions {
        "complete" => complete,
    }
}
```

`complete` is an ordinary `fn(&mut Value, &Value) -> Result<(), String>`. The example contains its implementation and a host policy. The setup is `Runtime::builder().resource(Task::definition()).authorizer(HostPolicy).open_sqlite(...).await?`. Callers create/update/delete or invoke custom actions through `execute`, read through `get`, and subscribe through `live(&actor, Query::equals(Task::KIND, "done", false))`.

Adding the tested second kind needs one declaration plus one builder registration. It needs **zero** per-kind repository, CRUD, controller, broadcaster or query-worker functions. Using the expanded style above, a one-field kind takes six declaration lines; physical lines are an authoring choice, not a quality metric. Application authors still write domain action behavior, host policy, authentication in their host, and calls consuming results.

The macro creates a marker and a visible `Definition`. Actual Rust field traits supply validation. The current narrow field set is bool/i64/String; neither typed action payloads nor generated typed query fields are implemented here. This removes framework plumbing but does not yet provide the desired final type-guided authoring experience.

## Executed evidence

Nine integration tests use only the library's public API, apart from deliberately installing a SQLite trigger to simulate an event-write failure. Two internal regression tests force states without exposing testing controls to applications.

| Probe | Observed result |
| --- | --- |
| Second unrelated resource declaration | Same generic create/read path; false survives; default policy denies reads, actions and live queries |
| Filter membership | Create adds; custom complete removes; update adds back; delete removes |
| Custom action checks | Same write policy and expected-revision checks as built-in mutations; rejected input changes nothing |
| Policy replacement without mutation | Query revocation terminates delivery; buffered delivery cannot bypass replacement policy |
| Row access revocation | Snapshot recomputes to empty with unchanged durable event cursor |
| Slow consumers and shared registration | Two subscribers share one query entry; 24 changes coalesce to the latest snapshot while all 24 events remain in SQLite |
| Business and storage failure | Business error and SQL trigger failure after current-state write produce no new visible snapshot or event; current state rolls back |
| Initial snapshot/subscription race | A condition-variable policy gate holds the runtime lock after rows are read; a concurrently scheduled writer cannot complete until release; the subscriber observes the resulting latest state |
| Explicit shutdown | Worker joins; streams close; further operations reject |
| Forced stale buffer after data-dependent revocation | Internal test replaces receiver with a known old snapshot after committed visibility change; delivery refreshes, and current authorization also excludes historical journal rows |
| Admission pressure during delivery | Internal test occupies all operation permits; Busy is retryable and preserves the pending delivery |

The race test uses explicit channels and a condition variable, not sleeps. Its writer-attempt signal establishes scheduling before release, not that a specific OS thread reached `Mutex::lock`. The source invariant is stronger: snapshot computation and registration use the same mutex as every runtime write. That is the boundary being probed. Five-second timeouts are deadlock guards only, not synchronization.

The coordinator independently ran the full verifier. As a negative control, removing stale-generation refresh made the forced-buffer regression fail because revoked data remained visible; restoring it passed the verifier. This demonstrates that the regression detects the reviewed defect.

## Contracts implemented

- Generic mutations open an immediate SQLite transaction, load the authoritative row, evaluate host action policy, compare revision, validate and apply the action, then atomically write current state and one event. Invalidation occurs only after commit. Unchanged data creates neither a revision bump nor an event. Identity tombstones prevent accidental recreation.
- One runtime mutex serializes database and policy state. Synchronous SQLite work runs through bounded blocking-task admission. The query worker is shared across the runtime, and duplicate actor/query keys share one watch channel.
- Initial query snapshot and subscription registration are one locked operation. Commits and policy replacement invalidate results. A slow consumer receives the latest full result, not a promise to observe every intermediate membership change. The durable journal keeps the separate mutation history.
- Delivery checks current policy and recomputes from authoritative state if its buffered generation or policy epoch is stale. Query permission loss terminates the stream; per-row loss removes rows. History inspection requires both the current row and the historical row to pass current read policy.
- Host-supplied `Actor` is a trusted identity seam, **not authentication**. `Authorizer` defaults to deny. Policies must be immutable, synchronous, nonblocking and non-reentrant; replace them with `set_authorizer` to notify subscriptions. Mutating policy internals outside that method is outside the invalidation contract.
- `shutdown` stops and joins the shared worker. `settle_queries` is a host diagnostic barrier used for deterministic tests.

Tokio documents watch as retaining the latest value; that is the desired snapshot semantics, not an event bus. The downloaded Tokio 1.53.1 `src/sync/watch.rs` was exercised directly by the tests. rusqlite transactions roll back unless explicitly committed, and the SQL-trigger probe verifies the boundary in this implementation. See [rusqlite Transaction documentation](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Transaction.html).

## Explicit limits

One runtime owns writes. Other processes or direct SQL changes do not generate invalidations; no distributed consistency claim follows. Every active query is conservatively recomputed on every mutation/policy change. The query language supports one equality predicate over a boolean field, with no pagination, joins, ordering controls or query planner.

The query count and retained channel versions are bounded, **not result bytes, rows, history size or memory overall**. There is no throughput or scaling evidence. Policy and action callbacks run under the serialized critical section; application side effects cannot be rolled back by SQLite and must not occur there. These callbacks are trusted native code, with no panic isolation. A query-worker panic can leave `settle_queries` waiting because its sender is retained; production supervision remains work.

No idempotent retry receipts, reaction engine, migrations, external providers, JWT/OIDC, field redaction, encrypted storage, tracing, cancellation-after-commit semantics or transport adapters are implemented here. Earlier prototypes cover some separate concerns, but this project does not claim those concerns are integrated. An authorized snapshot may remain with its caller after policy changes; revocation governs subsequent library deliveries, not recall of already-delivered data.

See [ASSESSMENT.md](ASSESSMENT.md) for implementer conclusions and estimation boundaries.
