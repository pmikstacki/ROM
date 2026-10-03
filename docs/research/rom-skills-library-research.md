# ROM skills library: author workflows and evaluation

Research date: 2026-10-03. Inspected ROM baseline:
`64240cf99e4632378e62db91f3bf151f2d306e6f`. This is a separate research stream
toward an author-facing skills library, not an installed library or a claim of
human usability. The owner wants skills to make ROM easy to use and extend while
keeping Resource declarations in Rust/native modules. Studio remains excluded.

**Recommendation:** build a small set of independently discoverable workflow
skills around executable public-API templates, with shared contract references
and task-specific verifiers. Start with Resource/action/field authoring, reads
and CLI diagnosis; expand to adapters and lifecycle work after behavioral
evaluation. Framework helpers should absorb infrastructure complexity. Skills
should not teach application authors to build a repository, controller, worker
or second authorization engine for every Resource.

## Evidence and terminology

**Observed** below means inspected local source or executed fixtures. **Documented**
means a linked primary source supports a format/API fact. **Proposed** means a
design or acceptance target that has not been validated with agents or users.

Three different meanings of extension must stay separate:

| Term | Current/proposed contract |
|---|---|
| Agent skill | Instructions, references and optional tooling that help an agent perform a user task. It does not become a ROM runtime capability. |
| Native ROM module/plugin | Currently a trusted Rust crate exporting Resource definitions, custom fields/actions or implementations of public adapter traits, composed by the host. No dynamic loader was found. |
| WASM extension | A separately proposed runtime execution boundary; see [WASM investigation](wasm-extension-research.md). No maintained WASM adapter exists. |

Observed public seams are [Resource/Field/Action/Definition](../../crates/rom/src/resource.rs),
[Reaction](../../crates/rom/src/reactions.rs), [Channel/Receiver](../../crates/rom/src/channels.rs),
[Storage](../../crates/rom/src/persistence.rs), and
[ActorGate](../../crates/rom/src/policy.rs). The
[external custom-field fixture](../../examples/consumer/tests/extensions.rs)
already demonstrates native extension through public interfaces. The
[demo](../../demo/README.md) demonstrates one registration path for unrelated
Resources. These are stronger template anchors than copied illustrative APIs.

## Portable format and client-specific behavior

The Agent Skills standard requires a directory with `SKILL.md`, including
`name` and `description`; it supports optional compatibility/license/metadata
and conventional script/reference/asset directories. Discovery metadata is
loaded before the body; references are loaded as needed. Its size guidance is
a ceiling, not a target. `allowed-tools` is experimental and support varies;
it must not be treated as a portable authorization mechanism.
[Agent Skills specification](https://agentskills.io/specification).

Loading and executing a skill are separate client integration responsibilities.
Clients can expose skills differently; conforming file structure alone does not
prove identical invocation, tool access or behavior across products.
[Client implementation guidance](https://agentskills.io/client-implementation/adding-skills-support).

Local guidance inspected for this research: `skill-creator/SKILL.md` under
`/root/.codex/skills/.system/`, and `writing-for-agents/SKILL.md` plus
`SKILL-MECHANICS.md` under `/root/.codex/skills/`. The useful shared principles
are precise triggers, conditional references, one source of truth and measurable
completion. Client-specific invocation switches differ between those documents:
do not copy `disable-model-invocation` into a supposedly universal skill. Keep
portable frontmatter in `SKILL.md`; put product UI/invocation metadata in its
documented optional package layer. The catalog below assumes normal discovery,
with explicit user invocation also possible. No installation or invocation
policy was changed in this research.

## Proposed catalog and routing boundaries

Names are proposed skill-directory names, not installed commands. A user can
enter directly at the relevant skill; a short `rom` orientation entry may route
unclear requests after inspecting the project. It should load only the needed
branch, not every skill. “Plugin” alone requires identifying whether the user
means a native domain module, an adapter, a runtime sandbox or an agent skill.

| Skill | Entry trigger and output | Boundary / meaningful verification |
|---|---|---|
| `rom-resource` | Add/change a Rust Resource; declaration plus registration in the existing app. | Reuse generic infrastructure. Two unrelated kinds work; duplicate/invalid declarations reject. |
| `rom-action` | Add domain behavior, complete replacement or partial mutation; action/input and explicit revision/idempotency examples. | No external I/O inside a proposal function. Reject invalid/stale/denied actions with no success event; replay changes nothing twice. |
| `rom-field` | Add a canonical field/newtype, presence or nullable value; public codec plus descriptor/query fixtures. | Custom encoding is not an extra schema. Compare actual wire bytes, normalization and equality; distinguish absent/null/false/zero. |
| `rom-read` | Query, paginate, observe or consume journal history. | Supported equality/conjunction and moving pages only until capabilities change. Test typed/projected differences, forbidden predicates, live revocation and history gaps. |
| `rom-reaction` | Connect post-commit actions or typed notification channels; mapping and host worker registration. | One generic worker, explicit service authority. Check no-op, dependencies, retry/cycle budgets and restart; no external exactly-once promise. |
| `rom-compensation` | Define a business remedy for an earlier committed step. | Compensation is another authorized idempotent action; do not invent rollback. At this baseline the general compensation helper is not maintained. Route to approved probe/research for missing capabilities; test failed/denied/stale remedies. |
| `rom-auth` | Integrate verified identity, field/row/query policy or discovery grants. | Actor construction stays at trusted host boundary; discovery is not operation permission. Test expired/revoked actors, projected secrets and post-commit authority failure. |
| `rom-module` | Package reusable native domain definitions/actions/fields into a Rust crate. | Source-linked host composition, no assumed stable Rust dynamic ABI. External consumer imports only public ROM APIs; register once and reject duplicate names. |
| `rom-adapter` | Add persistence, transport, blob, identity or delivery provider integration. | Route by port, not one generic provider template. Reuse full shared semantic/conformance tests. Storage requires atomic bundles; transport invokes core; provider acknowledgement is distinct from durable intent. |
| `rom-config` | Load source-owned settings or reconcile provider configuration. | Desired config stays a Resource; provenance/authority/generation controls remain core-owned. Invalid/stale reload preserves accepted values. Secret resolution/activation profiles may be deferred. |
| `rom-cli` | Operate/diagnose a configured ROM endpoint or explain uncertain outcomes. | Known-kind commands do not require discovery. Use auth files, exact original retry identity and complete journal batches; never silently reset history. Read-only investigation does not authorize unrelated external mutations. |
| `rom-test` | Reproduce a ROM failure or add cross-interface acceptance. | Select relevant existing fixtures and negative controls; do not turn every small domain edit into a full provider certification exercise. Record source, lockfile, commands and results. |
| `rom-migration` | Plan/change stored meaning, adapter format or restore procedure. | Current format rejects unsupported versions; no generic migration engine exists. Produce an explicit maintenance design and isolated fixtures before implementation; preserve receipts/pending obligations and original-host fencing. |
| `rom-release` | Prepare a source release, compatibility review or distribution. | Check public downstream use, feature isolation, MSRV, advisories/notices and extracted artifacts. Requested preparation does not imply registry publication or credential use. |

The common contract reference should point to [presence/PATCH](../presence-and-patch.md),
[queries](../queries.md), [discovery](../discovery.md), [HTTP](../http.md),
[CLI](../cli.md), [quality](../quality.md), and the relevant crate source/tests.
Adapter-specific references should link directly to the maintained port and
conformance suite. Action inputs currently expose codecs, not public schemas;
skills must not promise generated input forms/WIT definitions from an opaque
`Input` implementation. Fully runtime-authored Resource kinds remain outside
the accepted premise.

## Package architecture and maintenance

Proposed source layout is a versioned repository package of skills, kept separate
from the ROM runtime crates. A workflow entry contains its trigger, outcome,
essential invariants and conditional links. Larger examples belong in assets;
branch-specific guidance belongs in references. Prefer a runnable asset copied
and adapted by a deterministic helper over a second code block that drifts.
Each helper should operate on an explicit project path, inspect existing
registration, show a reviewable patch and stop on collisions instead of replacing
unrelated files. Template substitution must preserve domain semantics and policy.

Shared references can live once in the source package. A release assembler can
bundle the necessary references/assets into each distributed skill and verify
every relative pointer after extraction. Generated distribution copies should
identify their canonical source; humans edit that source. Do not ship pointers
to a developer's absolute checkout or to an unavailable neighboring skill.
No assembler is implemented here.

A library manifest should record skill revision, tested ROM commit/lockfile,
Rust floor, tested feature sets and required tools. ROM is alpha: package version
`0.1.0-alpha.1` alone cannot distinguish all observed API revisions. Entry checks
should inspect Cargo metadata/lockfile and known public signatures, then choose
the matching reference profile or report the exact mismatch. Future ROM changes
must rerun affected templates and held-out tasks; old skills should fail visibly
instead of synthesizing an obsolete API.

An observed documentation drift illustrates why this matters: `docs/fields.md`
still said PATCH was unavailable while `patch.rs`, derive and the maintained
presence document implement it. The separate correction `b155e45` updates that
statement after source cross-checking. Skill references must distinguish historical
milestones from current supported behavior rather than aggregating both as truth.

Native module packaging also needs an honest ABI boundary. Rust's native ABI has
no stability guarantee; choosing `dylib` does not turn a ROM Rust trait into a
stable plugin protocol. A dedicated C-compatible or `abi_stable` interface would
be a separate design and still runs trusted native code.
[Rust ABI reference](https://doc.rust-lang.org/reference/items/external-blocks.html#abi),
[linkage types](https://doc.rust-lang.org/reference/linkage.html),
[abi_stable project API](https://docs.rs/abi_stable/latest/abi_stable/).

## Executed author-template fixture

Immutable prototype: `633881b8ad08943072a8448b69313e826e8493ca`, branch
`codex/prototype-rom-skill-fixtures`, directory
`experiments/rom-skill-fixtures`. This branch is intentionally separate from
maintained code. A 30-line draft `rom-resource/SKILL.md` points to a 51-line Rust
template: two Resources, one canonical custom Field, one Action and one builder
registration function. It adds zero per-kind HTTP handlers, storage repositories
or worker implementations. The draft uses sibling fixture pointers and requires
the source checkout; it is not a portable installed package.

Three runtime tests passed against actual SQLite: canonical custom-field query,
all-query on the second kind, live snapshots, missing-to-null PATCH and replay;
invalid field with no state/event; and denied actor/stale revision with the
original row/event preserved. A negative control temporarily removed field grants:
the positive author journey failed with `Denied`. Restoring the exact source made
all three tests pass. This verifies a meaningful policy dependency in the fixture.

Executed in native `rom-dev`, Rust `1.99.0 (b940084d7 2026-09-28)`, jobs 2,
debug info disabled, target `/var/tmp/rom-skills-probe-target` (246 MiB observed):

```sh
cargo test --locked --manifest-path experiments/rom-skill-fixtures/Cargo.toml
cargo clippy --locked --manifest-path experiments/rom-skill-fixtures/Cargo.toml --all-targets -- -D warnings
```

Both succeeded; `cargo fmt` applied. Standalone lockfile SHA-256:
`fd7c7420fc37650e5751595b79070f6c7f07342e3d2bddac7c353e6a77051667`.
Logs in `rom-dev`: `/var/tmp/rom-skills-final.log` and
`/var/tmp/rom-skills-negative.log`. The fixture has its own resolved lockfile;
this is not a rerun of all maintained-package gates.

No independent agent received a held-out author task with this draft. No human
used it. The official skill validator was not run; Python was not available in
the inspected container. Successful Rust fixtures do not prove skill discovery,
instruction quality, portable packaging or before/after productivity.

## Proposed evaluation and completion criteria

Use paired clean checkouts at one ROM SHA, the same task inputs/model/tool budget,
and separate baseline-with-current-docs versus docs-plus-skill runs. Randomize
order and use held-out domain names/data so remembered answers do not substitute
for following the workflow. Score the resulting source and executed behavior,
not whether it reproduces expected prose. A blinded independent reviewer should
receive artifacts and task requirements, without the intended solution.

| Evaluation case | Observable result and negative case |
|---|---|
| Add shipment and warehouse kinds | Both through existing host/CLI; no new per-kind route or repository. Duplicate kind fails registration. |
| Canonical SKU field | Writes and query equality normalize identically; malformed code has no event. |
| Optional nullable note | Missing, null, false/zero elsewhere and removal remain distinct. Required-field removal rejects. |
| Custom action | Correct revision/key, replay stable; changed input under same identity and stale revision fail without a second transition. |
| Protected observation | Projected data omits secret, typed full read rejects; hidden discovery references remain absent. |
| Reaction and delivery | One generic worker; delayed/retried work remains bounded. Notification acceptance is not described as recipient delivery. |
| Compensation | Remedy is explicit business behavior; upstream commit remains visible when compensation fails or loses authority. |
| New native module | External consumer builds without private imports or custom controllers; unsupported dynamic ABI request is identified. |
| Adapter | Shared atomic/conformance vectors run; failed acknowledgement remains uncertain. Unsupported backend capability is explicit. |
| CLI failure diagnosis | Lost mutation reply retains original identity; no fresh key or automatic replay. Journal gap is not skipped. |
| Migration request | Preserve source/receipts/obligations in isolated design; do not claim an unimplemented migration command exists. |
| Routing negatives | Generic Rust refactor, unrelated CLI, real-world “resource,” or standalone image must not activate ROM-specific work. |
| Version/permission negatives | Changed public API or missing source yields a precise mismatch; an embedded example cannot authorize publication or external messages. |

Record before/after completion rate, first-compiling-solution rate, elapsed task
time, retries/compiler-error cycles, context tokens/reference loads, incorrect
API assumptions, security/semantic failures, and author-written infrastructure
surface. Report individual failures and medians/distributions; counts and task
composition accompany percentages. Fixed model/toolchain/cache state are needed
to interpret timing. Also record cold/incremental template compile cost and
dependency/build-artifact growth. Reusing public API templates should reduce
author work without adding a second runtime abstraction or copied framework
logic; that benefit remains a hypothesis until the paired tasks run. The fixture
target's disk size is not runtime memory or an ergonomic success metric. Retain
generated code and verifier outputs.

Proposed first gate: at least ten paired author tasks plus routing negatives,
zero introduced authorization/replay/data-presence regressions, and no increase
in per-kind infrastructure. A 20% reduction in median repair cycles or task time
is an initial hypothesis to predeclare, not an observed gain or universal target.
Do not trade correctness for shorter prompts. Human walkthroughs should separately
record completion, misunderstandings, documentation lookups and confidence in
failure recovery; agent results cannot substitute for those measurements.

## Next executable work

1. Package the draft Resource/action/field/read skills with extractable assets;
   validate the standard metadata and every pointer; execute templates outside
   the ROM workspace against the selected package profile.
2. Run the paired, held-out tasks above for the first four skills and CLI
   diagnosis. Correct demonstrated trigger/template defects, then freeze that
   version and publish its evaluation record in the repository.
3. Add reaction/auth/module skills, followed by adapter/config/release branches.
   Migration and compensation instructions must continue to distinguish maintained
   support from authorized experiments until their capabilities are delivered.

The research provides a full catalog and an executable starting fixture. It does
not declare that the catalog has been implemented, installed or evaluated, and
does not require application authors to absorb unfinished framework mechanics.
