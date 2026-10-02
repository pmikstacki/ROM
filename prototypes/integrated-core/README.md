# Integrated typed Resource probe

Disposable library experiment, not production ROM. SQLite is a replaceable reference capability used for this experiment; no production database is selected. One runtime owns writes. Rust 1.99.0 is the tested prototype floor, not an approved product MSRV.

## Run

From the host checkout:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core && ./verify'
```

From this directory inside `rom-dev`, run `./verify`. The verifier pins `CARGO_BUILD_JOBS=2`, uses its own target directory, checks formatting, Clippy, tests, rustdoc, an external application, core dependency isolation, five compile-failure diagnostics and a renamed-dependency consumer. Both workspace and fixture lockfiles are committed. No server is started. Scratch SQLite files use `PROTOTYPE-wipe-me-integrated-*` names and the reopen test removes its file.

## Layout

- `core`: `rom-probe`, descriptor/codec/field contracts, registration, typed commands/actions, storage capability seam, Tokio/Rayon supervision, live query signals and shared authorization.
- `derive`: `rom-probe-derive`, generates the structural descriptor, executable field codec and typed selectors from one declaration.
- `sqlite`: `rom-probe-sqlite`, actual scratch SQLite adapter with an atomic state/event/receipt/effect-intention transaction and fault injection.
- `consumer`: a separate application crate. `src/lib.rs` declares Task and Setting with ordinary business functions; `src/main.rs` runs both through one runtime. `tests/extensions.rs` implements a custom field and an equivalent manual Resource through public interfaces.
- `fixtures`: a separately locked downstream crate using a renamed dependency; compile-failure cases assert both diagnostic content and primary source line.

The core forbids unsafe code and its normal dependency graph contains no concrete database, HTTP, broker, Tower or Bevy dependency. The derive is currently an unconditional facade dependency; macro-free packaging is still follow-up work.

## Authoring

```rust,ignore
#[derive(Clone, Resource)]
#[resource(name = "tasks")]
struct Task {
    owner: String,
    title: String,
    done: bool,
}

fn complete(task: &mut Task, _: ()) -> Result<Vec<Intent>> {
    task.done = true;
    Ok(vec![])
}
const COMPLETE: Action<Task, ()> = Action::new("complete", complete);

let rom = Runtime::builder()
    .resource(Task::definition().policy(policy).action(COMPLETE))
    .build(storage, shared_rayon_pool)?;
let open = rom.live(&actor, Task::done_field().equals(false))?;
rom.execute(&actor,
    Command::action("one", COMPLETE, ())
        .at_revision(1)
        .idempotency("complete-one")
).await?;
```

The actual runnable consumer has all imports and uses two distinct resource definitions. Authors write no repository, standard controller, broadcaster, query worker or per-resource subscription resolver.

## Exact provisional contract

Fields: String, bool, u64, nullable fields, and downstream newtypes implementing `Field`. Create/replace codecs require every declared field, including nullable fields; `null` is a present nullable value and omission is rejected. Unknown fields and independent Serde annotations are rejected. There is no partial PATCH operation. Registered descriptors are frozen; dynamic normalization checks descriptor field names and shapes against codec output.

Actions: typed ordinary function pointers compute proposals on a shared Rayon pool. They must not perform external effects themselves; return intentions instead. Concurrent retries can execute a pure proposal more than once, but durable arbitration commits one transition. No-op replacement writes a receipt without advancing revision or emitting an event. A no-op proposing effects is rejected.

Persistence: synchronous semantic trait `Storage::{load,snapshot,receipt,commit}`. Mutation work runs off Tokio workers. One runtime gate protects authoritative reads, policy checks, conditional commit and live freshness for the single-owner topology. Conditional commit also checks identity and expected revision in an actual SQLite transaction. Real native commit errors conservatively return Unknown. The injected lost acknowledgment occurs after actual commit. Reopen plus same-key retry returns the original outcome.

Supervision: bounded fail-fast admission before task/CPU submission; actual work owns its permit. A retained Tokio JoinSet supervises Rayon completions independently of the response waiter. Completed supervisor tasks are reaped on later admission; shutdown closes admission and drains retained work. Hosts must explicitly await shutdown. This is not a complete cancellation-safe shutdown API.

Authorization: trusted host Actor(authority,subject), per-resource row policy, default-deny missing policy, before/after mutation checks, recheck of authoritative state after CPU proposals, and current checks before replay/disclosure. Host revocation and row-owner changes share the gate. This is not a credential verifier, an expiry policy, field-level authorization or cross-process policy service. The descriptor helper is trusted-host metadata inspection, not an authorized public discovery gateway.

Live reads: typed scalar equality, full conservative recomputation, authorize rows before matching predicates, one coalescing generation signal per subscription. Subscribe before initial snapshot; refresh at actual delivery. No protected result payload is buffered by ROM between live polls. The returned data becomes caller-owned; subsequent revocation cannot erase data already delivered. Query result size, number of subscriptions and synchronous read work are not bounded yet.

Reactions: state, event, receipt and effect intentions are atomic. One integration test reads a committed intention, submits a downstream action, injects a rollback, verifies the upstream Resource stays committed, then retries the same downstream identity twice and observes one downstream transition. There is no production continuation dispatcher, claim/ack protocol, persisted retry budget, backoff or loop bound in this probe. The test proves the lower-level composition, not a durable chain implementation.

## Known limits before MVP promotion

The [full report](../../docs/research/integrated-core-probe-results.md) is authoritative for observed evidence and gaps. Most urgent: bounded async read/query execution and live lifecycle; actor validity/field projection/public discovery; durable continuation scheduling and chain budgets; explicit PATCH and query AST contracts; packaged macro-optional facade; generic transport interface; cross-process write/authorization protocol if that topology is required. No production OpenSpec task is completed by this experiment.
