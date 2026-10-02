# Independent integrated MVP review

Reviewed code: `463f78c`, 2026-10-02. The review began at `1dc7951`, advanced to the blob integration at `9c58fc6`, and followed the backup, executable demo and corrective commits into the final code baseline. Work ran in the isolated `.worktrees/final-mvp-review` checkout; no implementation edits were made to the coordinator's workspace.

**Result:** no unresolved material correctness or security finding in the reviewed, documented MVP profile. Three independently reproduced defects were corrected and their regressions verified. This supports the scoped reusable MVP; repository-wide release verification, packaged-consumer evidence and dependency review remain separate evidence owned by the coordinator.

## Scope and criteria

The review used [the integrated requirements](../../openspec/changes/integrate-resource-mvp/specs/integrated-mvp/spec.md), [design](../../openspec/changes/integrate-resource-mvp/design.md), [quality gates](../quality.md) and [decision register](mvp-decision-register.md). It inspected maintained authoring, runtime supervision, current identity/field authorization, invocation/query/live/journal boundaries, both persistence adapters, configuration ownership, reactions/channels, blob lifecycle, native backup and the assembled demo.

The fixed premise was one Resource declaration and accepted descriptor, ordinary User/provider/settings Resources, Tokio/Rayon ownership, driver/protocol-independent core, atomic durable bundles and bounded post-commit chains preserving upstream commits. Deferred production UI, multi-writer coordination, automatic migrations, indexing, caches and per-key computation coalescing were not treated as missing MVP features.

## Reproduced findings and resolutions

### P1: accepted work could become permanently unclaimable at its byte limit

At `9c58fc6`, `crates/rom/src/reaction_work.rs:206–218` charged actual serialized bytes after every transition but reserved only the smaller Pending representation during admission. A scratch regression searched for the smallest accepted cap: one ordinary work record fit at **525 bytes**, then its first Claim returned `Overloaded`. It remained Pending; even an age-exhausted resolution lease needed additional space. This violated recoverable bounded work after the originating bundle had already committed.

Correction `463f78c` (authored as `b1801a8`) reserves maximum-width mutable counters, timestamps, lease/resolution state and delivery outcome during admission/materialization. Terminal records retain that reservation. The original regression now passes, as do seven maintained boundary/progression tests in [ledger_capacity.rs](../../tests/persistence/tests/ledger_capacity.rs), including failed child admission followed by terminal parent progress and restore-generation growth.

Compatibility is explicit: old experimental ledgers without sufficient reserved headroom return `Unsupported` on open/update/archive validation, rather than silently increasing limits. There is no automatic migration or capacity-edit tool. The [maintenance documentation](../../crates/rom-backup/README.md) states this limitation and preserves source data/obligations.

### P2: blob shutdown advertised quiescence before releasing backend ownership

At `9c58fc6`, `crates/rom-blob/src/service.rs:62–67,169` decremented active work before dropping a task-local `Arc<Inner>` that retained the Runtime and blob providers. After canceling an upload observer, draining both services and dropping all host handles, a weak provider reference could still upgrade. Immediate exclusive database reopen could therefore race a supposedly drained task.

The reproduction injected only a 200 ms scheduling yield immediately after `drop(work)`, making the lifetime ordering deterministic. It failed before `f509d10` and passed with the same injection after that correction. Work now retains lifecycle/watch state independently and the task drops its backend-owning reference before publishing drain completion. All twelve maintained [blob lifecycle tests](../../crates/rom-blob/tests/lifecycle.rs) also pass. Scheduling instrumentation was removed from the review checkout before final integration checks.

### P2: current identity denial occurred after registration lookup and codecs

At `9c58fc6`, `crates/rom/src/execution.rs:599–628` looked up the kind and normalized Create/Replace/Patch input before its first authoritative actor check. With an ActorGate returning `Denied`, malformed input for a registered kind returned `Invalid { kind, field }`; an unknown kind returned `Unregistered`. This exposed registration/validation distinctions and invoked native codecs for an identity already denied by current authority.

Correction `42cc572` checks current authority inside bounded I/O and the commit gate before lookup/normalization, retaining later checks for concurrent changes. The original scratch probe now returns Denied. The maintained [initial-authority regression](../../examples/consumer/tests/initial_authority.rs) additionally covers both invocation forms and verifies that a denied caller executes no custom decoder, with an authorized positive control.

## Independent executed evidence

Environment: native `rom-dev` container, Rust `1.99.0 (b940084d7 2026-09-28)`, two Cargo build jobs, offline locked dependencies, persistent target `/var/tmp/rom-final-review-target`. Commands ran from the review checkout mounted under `/workspace/ROM`.

Before correction, the three scratch regressions above were observed failing; they were not inferred from existing passing tests. The blob reproduction required the documented scheduling injection; the authority and ledger failures required no production modification. The scratch artifacts were retained locally under `/var/tmp/rom-final-review-repros`; maintained regressions are linked above.

At `9c58fc6`, all 67 consumer tests and 34 selected native persistence/channel/reaction tests passed independently, demonstrating that the newly discovered boundary failures were absent from the old coverage. At combined `42cc572`, native backup tests, all twelve blob lifecycle tests and the actual TCP demo tests passed.

After the ledger correction, these commands all exited zero:

```sh
cargo test -p rom-consumer --tests
cargo test -p rom-storage-conformance --tests
cargo test -p rom-backup -p rom-blob -p rom-demo
```

They ran at isolated commit `5efdc56`, the reviewer cherry-pick of the ledger fix onto `42cc572`. Its maintained implementation and Cargo.lock are identical to final `463f78c`; the difference contains only documentation/notices, which were reviewed separately. The native subprocess fixture is intentionally ignored as a standalone test and executed by its parent crash-boundary test. Original authority/ledger probes and the maintained initial-authority/seven ledger-capacity regressions were then rerun successfully at exact `463f78c`.

This review did not independently rerun every historical research experiment, the final package builder, advisory database audit or MinIO suite. Their coordinator/agent reports must remain separately attributed; the independent review does not convert reported evidence into locally executed evidence.

## Positive observations and readiness boundaries

Current actor checks, commit-gate serialization and post-I/O generation validation consistently protect observation handoff. Historical receipts/journal views require both historical and current row/field permission; complete reads include absent fields. Query predicates are authorized before scanning. Source permits bind exact targets, current activation revisions and expiry. Blob publication validates complete content and prevents generic callers from forging Ready metadata.

Both storage adapters share the same durable state machine and actual-database conformance. Backup uses coherent native transactions, bounded private archives and fresh-destination publication; restore preserves receipts/uncertainty while rotating journal history and fencing prior claims. The executable demo exercises the maintained Resource, HTTP, configuration, reaction, notification and folder-blob path on both databases. No material documented-standard violation remained after correction; no style-only restructuring was requested.

Stale auth documentation and an overstatement of enforced multi-owner registration were corrected in `666b85a`. Exactly one Runtime owning a deployment remains a **host precondition**, not an automatically enforced global registry of shared Storage handles.

Readiness remains limited to the documented local MVP. Native callbacks and adapters are trusted; finite receipt/work capacities can reject new obligations; scans and serialized ledger updates have explicit cost limits. External notification deduplication, external blob recovery and original-host cutover fencing remain host duties. The demo uses synthetic loopback authentication and supplies engineering walkthrough evidence, not a human usability study or production provider/UI certification. Automatic migration, multiple writers and machine-power-loss certification remain unclaimed.
