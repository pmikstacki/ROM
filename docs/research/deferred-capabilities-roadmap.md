# Deferred capability research and roadmap

Research date: 2026-10-02. Baseline: `54378c9`, the completed experimental MVP. This report covers the capabilities deferred by the [decision register](mvp-decision-register.md), [release acceptance](mvp-release-results.md), [completion index](completion-index.md), and the three broad OpenSpec proposals. It records research and recommendations, not implementation or executed conformance results. The owner's revised scope is core/API/resilience work, a CLI, and full research of deferred topics. Production Studio is excluded from current implementation. The deferred implementation backlog below stays explicit; completing this research does not complete that backlog.

The main recommendation is to strengthen shared contracts before adding deployment profiles: authorized discovery and client outcome semantics; format and identity retention; configuration/secret activation; then tenant and writer isolation. RabbitMQ can be evaluated independently against the existing invocation and journal interfaces. Cache and Salsa adoption should follow representative measurements. Keep one Resource domain, mutations through core, Tokio for asynchronous work, and Rayon for bounded CPU work.

## Evidence and decision boundaries

**Observed** means inspected source, local command output, or an existing report with its original limitations. **Documented** means a linked primary source establishes the vendor/library behavior. **Recommendation** means an engineering inference, not an owner decision or a demonstrated result. Every acceptance probe below is proposed and unexecuted in this research task.

Decision flags: **E** is a reversible engineering choice that can be tried locally; **H** requires an explicit host/product policy before a production profile is claimed; **U** needs user/participant judgment, external credentials, or operational validation beyond a local synthetic fixture. H/U do not prevent local research or contract tests. No external provider, production deployment, human usability study, or universal exactly-once guarantee is certified here.

The broad OpenSpec task lists intentionally remain open after a narrower MVP. Several unchecked entries already have scoped maintained evidence: frozen registration, atomic bundles, typed channels, native auth, HTTP, configuration ingestion, and blob lifecycle. Use the [MVP acceptance](mvp-release-results.md) for those facts. Do not interpret every unchecked broad task as absent code, or check off a broad production requirement merely because its MVP subset exists.

## Coverage and dependency map

All entries are **researched; implementation deferred**, except D26 where authorized discovery is a shared candidate for current CLI/core work, and D35 which is excluded from current implementation. Stage numbers express dependencies, not dates or commitments to implement every profile.

| ID | Deferred capability | Recommended stage | Principal dependency / decision |
|---|---|---:|---|
| D01 | RabbitMQ invocation and journal binding | 2, independent trial | Shared wire/outcome contract; E/H |
| D02 | WASM extensions | 5 | Versioned capability bridge and budgets; E/H |
| D03 | Supervised single-flight | 4 | Canonical identity and measurement budget; E |
| D04 | Completed outcome cache | 4 | D11 and current authorization; E/H |
| D05 | Salsa derived reads | 4 | Complete revision/policy dependencies; E |
| D06 | Richer typed predicates and ordering | 2 | Codec capabilities and disclosure rules; E |
| D07 | Joins and aggregates | 4 | D06, bounded cost, row authorization; E/H |
| D08 | Indexed execution and snapshot pagination | 3 | D06, adapter snapshot lifetime; E/H |
| D09 | Relationship integrity and deletion policy | 3 | Atomic invariant scope; E/H |
| D10 | Resource, receipt, event, payload and adapter migrations | 1 | Explicit compatibility matrix and recovery; E/H |
| D11 | Receipt expiry and retry identity reclamation | 1 | Admission horizon and retained identity evidence; E/H |
| D12 | Journal, work ledger and tombstone cleanup | 2 | D10/D11, dependency-aware collection; E/H |
| D13 | Tenant isolation and cross-tenant sharing | 3 | Identity scoping everywhere; E/H/U |
| D14 | Independent writers, leases and reconciliation | 3 | Storage fencing, D10/D11, journal ordering; E/H/U |
| D15 | Additional database and replica profiles | 3–5 | D14 and per-backend conformance; E/H/U |
| D16 | Cross-Resource atomic operations and compensation | 5 | Explicit atomic scope; E/H |
| D17 | Identity-provider discovery and key refresh | 2 | Trusted issuer configuration; E/H/U |
| D18 | Login, browser/device flow and sessions | 2 | D17, credential storage and host policy; E/H/U |
| D19 | Automated first-administrator bootstrap | 2 | Durable single-use claim; E/H |
| D20 | Privileged account recovery | 3 | Assurance policy and notifications; E/H/U |
| D21 | Self-service linking and JIT provisioning | 3 | Authority-qualified identities, D18; E/H/U |
| D22 | User-delegated background work | 4 | Revocable delegation and expiry; E/H |
| D23 | Secret resolvers and rotation | 2 | Protected reference and activation contract; E/H/U |
| D24 | Layered partial configuration and writeback | 2 | Ownership before precedence, D23; E/H |
| D25 | Real provider activation and reconciliation | 3 | D23/D24 and durable activation intent; E/H/U |
| D26 | Authorized Resource discovery and generic clients | 1 | Core policy seam and versioned descriptors; E/H |
| D27 | Controlled extra attributes and live evolution | 5 | Rust/native definitions remain authoritative; D10/D26; E/H/U |
| D28 | Physical blob cleanup and backup completeness | 2–3 | Reference/restore fencing; E/H/U |
| D29 | Advanced blob capabilities and confinement | 4 | Per-provider capabilities; E/H/U |
| D30 | Encryption at rest and physical erasure | 3 | Key lifecycle, backup and medium policy; E/H/U |
| D31 | Declarative authorization engines/action policy language | 4 | Existing native policy contract; E/H |
| D32 | Exported telemetry and operational SLOs | 2 | Bounded payload-free signals; E/H/U |
| D33 | Package publication, compatibility and support | 3 | Quality gates and explicit support policy; E/H/U |
| D34 | Human author study and production benchmarks | continuous | Real participants/workloads; U |
| D35 | Production Studio | excluded | Retained research only; U |
| D36 | Direct Serde codec mode and richer extension descriptors | 3 | One authoritative encoding contract; E |
| D37 | Real notification providers and external effect guarantees | 3 | Provider acceptance/idempotency contract; E/H/U |

## Transport and execution extensions

### D01 RabbitMQ

**Question/options.** Bind existing generic invocation to AMQP 0-9-1 using Lapin or amqprs; alternatively adopt AMQP 1.0 through fe2o3-amqp. These are different protocol choices, not interchangeable clients. Current primary API pages expose [Lapin 4.12.0](https://docs.rs/lapin/4.12.0/lapin/), [amqprs 2.1.5](https://docs.rs/amqprs/2.1.5/amqprs/), and an [AMQP 1.0 fe2o3-amqp client](https://docs.rs/fe2o3-amqp/latest/fe2o3_amqp/). Lapin offers Tokio, consumer streams, confirm futures, and optional topology recovery. amqprs is Tokio-based with channel/connection callbacks. These version observations are not lockfile selections or compatibility tests.

**Recommendation — E/H.** Trial an optional `rom-rabbitmq` using Lapin, explicit Tokio selection, and a host-supplied connection/authentication resolver. Lapin's confirm future fits the proposed reply lifecycle; amqprs is a credible smaller API comparison if dependency/build cost becomes material. Retain AMQP 1.0 as a separately requested profile. No AMQP crate enters `rom`. Do not generate per-kind consumers or write storage directly. The [existing invocation](../../crates/rom/src/invocation.rs) and [journal](../../crates/rom/src/journal.rs) provide the required semantic seams.

**Exact first trial contract.** A versioned, strict envelope contains an operation family and its existing core request. `invoke` carries `Invocation { kind, id, expected, idempotency, operation }`; `journal` carries `{kind, after}`; `journal_head` carries `{kind}`. Per-attempt AMQP correlation IDs are distinct from durable principal-scoped idempotency keys. A host resolver establishes `Actor` from verified delivery credentials/trust context; payload `subject` or arbitrary broker headers cannot establish identity. Authenticate before detailed request diagnostics and pass every mutation to `Runtime::invoke_projected`, matching HTTP's authorized response projection. Return that projected result or a bounded safe error category; never serialize an internal full row. Unknown commit outcome is distinct from rejection. Read/query can follow with the same vectors. Live streams and broker journal subscriptions require a later explicit streaming/checkpoint profile; the first trial rejects them rather than silently approximating them.

Use durable request/reply queues and persistent publications for the durability trial. Restrict reply destinations through host policy so a caller cannot redirect protected results arbitrarily. Enable publisher confirms and `mandatory` routing, handle returned/unroutable replies, and manually acknowledge each request on its receiving channel only after its reply publication is positively confirmed and routable. A broker confirm proves broker acceptance, not client consumption; loss between reply acceptance and request acknowledgement can duplicate replies. The client deduplicates correlation/result identity and recovers a missing reply by resubmitting the same semantic request/key. Reauthorization may cause a replay to return denial after a prior success; never store or blindly replay a serialized formerly authorized reply in the binding. These lifecycle choices follow the distinct [consumer acknowledgement and publisher-confirm contracts](https://www.rabbitmq.com/docs/confirms). Do not use [Direct Reply-To](https://www.rabbitmq.com/docs/direct-reply-to) for the durable-reply trial: its replies are at-most-once.

Malformed/unauthorized messages with no safe reply destination need a bounded reject/dead-letter policy. Transient failure must not create an immediate requeue loop. Close/cancel intake, use bounded retry/backoff or a declared broker retry topology, and leave uncertain requests recoverable. Bound prefetch, message body, metadata, in-flight invocation count, reply bytes, pending confirms and all deadlines. A body-size check after a client library has allocated a delivery is not a transport memory ceiling: also configure broker message limits and inspect the library's delivery buffering. Reconnection must not acknowledge a stale delivery tag on a new channel. Initial recovery can be host-supervised restart with explicit readiness; do not assume automatic topology recovery resolves application outcomes.

**Acceptance probe — local, unexecuted.** Run the same two-Resource invocation vectors embedded, HTTP, and through an actual broker, against both maintained storage adapters. Cover duplicate requests before/after restart, changed input under one key, equal input under distinct keys, principal isolation, expired/revoked actors, field projection and exact values. Inject loss after core commit/before reply, after broker reply confirmation/before request ack, unroutable reply, publisher nack/timeout, connection loss, broker restart, poisoned input, capacity exhaustion and shutdown during accepted work. Count durable revisions/receipts/events, not only returned responses. Journal probes resume from the last client-owned cursor, include filtered empty pages, revocation, stale generation and retention gaps. Assert no journal page is treated as a database acknowledgement or as a coalescing live snapshot. A later streaming bridge must persist its checkpoint only after its declared downstream acceptance and retain replay-safe event identities.

**Feasibility observed.** Read-only checks found the persistent `rom-dev` container, Rust 1.99.0, and NixOS `24.11.20250630.50ab793`. `nix` is available; `rabbitmq-server`, `erl`, Docker and Podman are absent from its PATH. Evaluating `(import <nixpkgs> {}).rabbitmq-server.version` failed because the configured channel paths do not exist. Therefore an explicit pinned nixpkgs input or a host-provisioned package is necessary. No broker was installed, started or tested. A future loopback harness should use a private writable data/log/config directory, unique node/ports, bounded Erlang resources, readiness checks and a cleanup trap; RabbitMQ documents [relocatable paths](https://www.rabbitmq.com/docs/relocate) and [configuration](https://www.rabbitmq.com/docs/configure). It needs no cloud credentials. Pin/test a supported broker/package independently of this old container OS; a local restart trial cannot establish clustered failover or power-loss durability.

### D02 WASM

**Question/options.** Trusted native extensions remain simplest; Wasmtime offers a compiled sandbox/runtime and component interfaces; [Wasmi](https://docs.rs/wasmi/latest/wasmi/) is an interpreter alternative worth measuring for small plugins. Wasmtime documents imported host capabilities as the outside-world boundary and exposes [fuel/epoch controls](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html); [sandboxing](https://docs.wasmtime.dev/security.html) does not automatically authorize host functions.

**Recommendation — E/H.** Start with pure validation/proposal plugins returning canonical bounded values, not direct mutation/storage/network access. Generate a versioned bridge from the accepted Resource/codec contract. Host imports submit authorized actions through core. Prefer Wasmtime for a first general component trial; compare Wasmi only if binary size/startup constraints justify a second engine. Keep compilation, execution, fuel, wall-clock, memory and output limits separate, running CPU work on the established bounded CPU service. Unknown ABI/capability versions fail registration.

**Probe/dependencies/risks.** After D10/D26, locally test infinite loops, allocation growth, traps, malformed results, missing imports, guest cancellation, nested calls, secret exposure and incompatible modules. No external account is needed. Host imports can reintroduce ambient authority; runtime upgrades and fuel tuning require their own maintenance/measurement. Do not claim hot replacement or a stable native Rust ABI.

### D03 Supervised single-flight

**Question/options.** Keep durable-only behavior, join concurrent matching retries, or use a cache library initializer. Existing [cache experiments](cache-prototype-results.md) reduced duplicate durable attempts with single-flight but did not establish negligible unique-request overhead. [Moka](https://docs.rs/moka/latest/moka/future/struct.Cache.html) and [quick_cache](https://docs.rs/quick_cache/latest/quick_cache/sync/struct.Cache.html) provide initialization facilities; neither substitutes for ROM work ownership.

**Recommendation — E.** Keep disabled by default pending an integrated trial. A private bounded flight table joins only identical canonical requests under the full durable identity and storage generation. Runtime-owned work outlives all observers; waiter limits are distinct from job limits. Current authorization applies to every waiter and disclosure. A different key never joins merely because inputs match.

**Probe/dependencies/risks.** Locally measure baseline versus supervised joining under unique, duplicate, mixed and adversarial keys; count proposal executions, storage attempts, commits, CPU, allocations and peak retained memory. Inject leader/follower cancellation, panic, overload, changed input and shutdown. Preselect the acceptable unique-traffic regression budget with the host before choosing a production default. No credentials are required; shared-host microbenchmarks are not production capacity evidence.

### D04 Completed outcome cache

**Question/options.** No cache, Moka, quick_cache, or a small private bounded map. Moka documents best-effort capacity bounding; cache capacity alone cannot enforce service memory limits. Native time-to-idle is not a durable retry horizon. Existing comparisons measured synchronous wrappers, not every library initializer or expiry mode.

**Recommendation — E/H.** Cache minimal confirmed outcomes only, with explicit byte admission and an absolute validity deadline no later than the durable identity horizon. Check storage/restore generation and current read/field authority before disclosure. Do not cache `Unknown` as success or extend validity on a hit. Keep the library private and replaceable; choose after measurement rather than selecting a universal winner.

**Probe/dependencies/risks.** D11 precedes finite-retention adoption. Test eviction, expiration, rehydration, restore, deletion/recreation, revoked fields, principal separation and uncertain commits against cache-disabled output. Measure policy/storage calls on hits: current authorization may erase the apparent hot-loop benefit. Entirely local until a production workload is selected.

### D05 Salsa derived reads

**Question/options.** Plain recomputation, generated invalidation/materialized reads, or Salsa incremental queries. [Salsa's overview](https://salsa-rs.github.io/salsa/overview.html) explains tracked input/query dependencies. ROM's [prototype](cache-prototype-results.md) demonstrated reuse and also reproduced stale authorization when a dependency escaped tracking.

**Recommendation — E.** Limit Salsa to expensive pure derived reads. Generate or centralize projections from the single Resource descriptor; do not ask authors to maintain a second schema. Track data revisions, membership, actor/policy/config generations and all external inputs explicitly. Rebuild on journal gaps. Mutation memoization remains excluded.

**Probe/dependencies/risks.** Compare with a recomputation oracle after every randomized update, deletion/reinsertion and policy change. Measure total latency including projection maintenance, memory plateau, cancellation and database lifecycle; skipped function bodies alone do not prove speedup. Local testing needs no credentials. D14 is required before distributed freshness can be claimed.

## Queries, relationships and durable evolution

### D06 Richer typed predicates and ordering

**Question/options.** Add comparisons/disjunction/presence/string operations incrementally, or expose a vendor query language. Current maintained semantics are equality/conjunction with canonical codecs. SQLite's [SELECT semantics](https://www.sqlite.org/lang_select.html) are vendor behavior, not ROM's portable field semantics.

**Recommendation — E.** Extend the shared typed AST with explicit comparison capabilities and a total ordering, including identity tie-breakers and null/missing rules. Reuse it across ordinary/live reads, CLI and future bindings. Reject predicates/order on unauthorized fields before execution; bound AST depth, clauses, scanned rows and output. Do not expose raw SQL through core.

**Probe/dependencies/risks.** Locally compare SQLite/redb results to an independent evaluator over exact decimals, large integers, dates, missing/null, Unicode and equal keys. Verify alias normalization, denial before scanning, and live membership/order changes. Implement each operator only after its codec capability and negative vectors are specified; collation and coercion defaults otherwise create silent drift.

### D07 Joins and aggregates

**Question/options.** Bounded core evaluation, adapter-compiled joins, explicit derived Resources, or an unrestricted analytical language. SQL provides aggregation but does not decide which rows/fields a ROM actor may influence.

**Recommendation — E/H.** First favor explicit derived Resources or a bounded authorized relation traversal; introduce join/aggregate capabilities only for concrete use cases. Define whether aggregation occurs over authorized rows and how count, grouping and errors avoid hidden-row disclosure. An aggregate's dependency set must drive live invalidation and any Salsa bridge.

**Probe/dependencies/risks.** After D06/D09, use two actors with identical visible data but different hidden rows; outputs/errors/cursors must not reveal those hidden rows. Test fan-out, cycles, empty groups, exact numeric overflow, concurrent changes and cost rejection. Local correctness is feasible; production index plans and acceptable latency require workloads. Never label bounded scan support as indexed arbitrary querying.

### D08 Indexes and snapshot pagination

**Question/options.** Keep moving keyset pages, hold bounded read snapshots, or materialize versioned result sets. SQLite WAL allows concurrent readers and a writer but long reads can inhibit checkpoint progress; it is not a multi-host network-filesystem protocol. [SQLite WAL](https://www.sqlite.org/wal.html).

**Recommendation — E/H.** Advertise semantic operators, available indexes, execution budgets and consistency as separate capabilities. Add optional snapshot tokens with explicit expiry/capacity only when a consumer requires repeatable traversal. Bind cursors to normalized query, ordering, scope and generation. A caller must knowingly restart when a snapshot expires.

**Probe/dependencies/risks.** Following D06, interleave insert/delete/reorder with pagination and assert either the declared moving-view behavior or a stable snapshot. Test query/actor token substitution, expired handles, bounded retained snapshots, index build failure and restore. Local adapter tests are feasible; growth/latency tradeoffs remain workload decisions.

### D09 Relationship integrity and deletion

**Question/options.** Identity-only references (current), restrict deletion, explicit cascade actions, or cross-store integrity. [SQLite foreign keys](https://www.sqlite.org/foreignkeys.html) require actual enabled/enforceable constraints; a typed reference alone supplies no enforcement.

**Recommendation — E/H.** Offer restrict semantics first within one declared atomic storage scope. Validate reference creation against concurrent deletion in that same scope. Treat cascade as explicit bounded authorized work with visible partial progress unless an atomic profile is selected. Cross-adapter references remain identity-only until coordinated enforcement exists.

**Probe/dependencies/risks.** Race parent deletion with child creation; test dangling references, self/cyclic references, denial on a child, retries, and crash midway through cascade. Run identical behavior on SQLite/redb, not only SQL foreign keys. Host policy chooses restrict versus cascade and how hidden references affect errors; avoid leaking inaccessible rows through an integrity diagnostic.

### D10 Version compatibility and migrations

**Question/options.** Reject unknown formats (current), offline copy-and-validate migration, or online expand/contract evolution. SQL DDL is only part of the problem: Resource codecs, receipts/fingerprints, journal rows, reaction/channel payloads, configuration and backup manifests all need compatibility rules. Generic Resource payload evolution may require no table alteration at all; distinguish wire protocol version, Resource encoding version and adapter-native layout. SQLite documents a constrained [ALTER TABLE](https://www.sqlite.org/lang_altertable.html) model; [SQLx migration support](https://docs.rs/sqlx/latest/sqlx/) cannot decide ROM semantic compatibility.

**Recommendation — E/H.** First implement a host-invoked offline migration to a new destination with a version manifest, deterministic transforms, bounded batches, validation and atomic publication where supported. Preserve stable receipt/delivery identities and pending obligations; change journal generation when replay meaning changes. Reject downgrades unless an explicit reverse migration exists. No ad hoc business mutation may bypass core: semantic changes are core actions; storage-format translation is a separately gated maintenance operation with intake stopped.

**Probe/dependencies/risks.** Local fixtures cover every persisted family and current format-3 headroom rules, interrupted migration, unsupported newer data, rollback to the untouched source, unknown payload codecs, mixed versions and backup restore. Compare counts/revisions/obligations and sample/full canonical decode as budget permits. Old experimental data compatibility is a declared choice, never an accidental decoder fallback. Online evolution follows D14/D27 and is not the initial recommendation.

### D11 Receipt expiry and old retries

**Question/options.** Retain all receipts under admission budgets (current), compact outcomes while retaining identity evidence, or accept only commands within an enforced epoch/horizon. Deleting a receipt does not prove an action never committed. Vendor deduplication windows also do not replace ROM retention; [DynamoDB's transaction token](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html) has a documented finite validity period.

**Recommendation — E/H.** Specify a server-enforced retry epoch/horizon before reclamation. Preserve compact identity/fingerprint tombstones until an old command is provably inadmissible; after that reject it as expired, never reinterpret it as fresh. A client-supplied timestamp without trusted admission rules is insufficient. Separate outcome payload retention from non-reexecution evidence and authorization metadata. Restore must not reopen retired epochs.

**Probe/dependencies/risks.** Locally retry immediately before/at/after expiry, after cleanup, after deletion/recreation and after older-backup restore; no expired identity causes a new mutation. Race admission, GC and commit; test clock skew/rollback and generation changes. Host policy chooses retention and recovery windows. This is prerequisite to D04 and long-running production services; finite memory/disk without admission semantics cannot provide unbounded deduplication.

### D12 Journal, work and tombstone cleanup

**Question/options.** Retain until capacity, time-based removal, or obligation-aware compaction. These records have different lifetimes: a journal cursor can expire while a receipt still prevents replay, and a pending delivery can outlive its source event.

**Recommendation — E/H.** Define independent retention floors and safe terminal states, then compact only records with no required recovery dependency. Preserve receipt evidence from D11, active leases, pending reactions/deliveries, causal budgets and reference restrictions. Publish an explicit history gap to stale subscribers; never silently reset their cursor. Cleanup itself must be restartable and bounded.

**Probe/dependencies/risks.** After D10/D11, crash at each collection boundary, restore a pre-cleanup backup, retain a deliberately stalled delivery and retry a deleted Resource action. Assert capacity returns without resurrecting obligations or deleting active work. Local fake time/storage fixtures suffice. Physical erasure is separately D30, not implied by logical compaction.

### D13 Tenancy and sharing

**Question/options.** Deployment/database separation, row-scoped tenancy, or explicit cross-tenant shares. The first remains a useful isolation deployment but does not demonstrate row-level multitenancy. PostgreSQL [row security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html) has bypass roles/owner behavior, and integrity checks can create disclosure channels.

**Recommendation — E/H/U.** Keep Tenant as an ordinary Resource. Add trusted scope to every key/receipt, query/cursor, journal, cache, reaction, channel, blob reference, configuration and backup operation before advertising tenant isolation. Cross-tenant sharing is an explicit authorized relation/action, never a missing filter. Database row security may add defense in depth but cannot replace core checks.

**Probe/dependencies/risks.** Run an adversarial two-tenant suite that deliberately reuses Resource IDs, idempotency keys, cursor positions and blob names; test joins, indirect fields, worker recovery, disabled membership and restore into the wrong scope. Local synthetic tenants require no accounts. Production policy chooses hierarchy/sharing/admin scope and leakage budget; legal/organizational separation is not established by these tests.

### D14 Independent writers and distributed observation

**Question/options.** Enforce one owner, coordinate multiple ROM writers through storage, or accept arbitrary bypass writes. SQLite serializes its own writers, but that does not synchronize ROM's in-memory policy/invalidation. PostgreSQL offers [transactional locks](https://www.postgresql.org/docs/current/explicit-locking.html); [NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html) is a signal to listeners, not ROM's durable journal. Even etcd distinguishes linearizable operations from [watch guarantees](https://etcd.io/docs/v3.6/learning/api_guarantees/).

**Recommendation — E/H/U.** First enforce/document an exclusive owner; later require durable monotonically fenced ownership/claims, atomic receipt arbitration, current-authority validation and polling/reconciliation from a safe journal cursor. A process-local gate is insufficient. Fence every protected storage write and work acknowledgement; an external recipient needs its own idempotency/fencing contract. Preserve the distinction between cooperating ROM writers and arbitrary SQL writers that omit receipts/events.

**Probe/dependencies/risks.** After D10/D11, run separate processes against one supported store: paused lease owner resumes after expiry, split connectivity, lost wakeups, clock jumps, out-of-order transaction completion, policy revocation on another owner and restore while an original process remains alive. Assertions cover no skipped retained event, no stale acknowledged claim and explicit resync. Local processes are feasible; multi-host/power/failover claims require that deployment. Do not turn a sequence allocated before commit into an assumed global commit-order cursor.

### D15 Additional databases and replicas

**Question/options.** PostgreSQL/SQLx, MySQL, MongoDB, DynamoDB, FoundationDB, RocksDB, libSQL/Turso or host-selected implementations. The existing [relational](storage-relational-backends.md) and [nonrelational](storage-nonrelational-backends.md) reports compare these families and their bounded transaction/query constraints. Current [SQLx](https://docs.rs/sqlx/latest/sqlx/) supports Tokio; [MongoDB transactions](https://www.mongodb.com/docs/manual/core/transactions/) and [DynamoDB transactions](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html) demonstrate why deployment and transaction limits must remain explicit.

**Recommendation — E/H/U.** Choose PostgreSQL as the first candidate when client/server and multiple cooperating processes are needed; reuse the complete atomic-bundle conformance suite. Keep vendor transactions and SQL outside core. Add other families on concrete host demand, preserving research coverage without promising to certify every product. Replica reads must declare freshness and authoritative receipt recovery; offline acknowledgement is a separate profile.

**Probe/dependencies/risks.** Local PostgreSQL/replica fixtures can test rollback, unknown commit, conditional revision, receipts, journal ordering and freshness barriers. Hosted services, managed failover and cloud durability need credentials and a budget; an emulator is not service certification. D14 precedes multiwriter claims. Database substitution must not change resource behavior, canonical field semantics or author-facing code.

### D16 Cross-Resource atomicity and compensation

**Question/options.** Keep accepted post-commit chains, add explicit compensation actions, or negotiate bounded multi-Resource atomic bundles. Ordinary [database locking](https://www.postgresql.org/docs/current/explicit-locking.html) can protect invariants inside one database but is not a distributed transaction across providers.

**Recommendation — E/H.** Preserve chains as default. Introduce compensation as another idempotent, currently authorized action with its own failures, not a claim that history was undone. Add atomic multi-Resource operations only within a declared adapter scope and with all read/write dependencies validated; never silently mix them with blob/provider effects.

**Probe/dependencies/risks.** Race two actions over a shared invariant; fail each bundle write; crash after an upstream commit; deny or fail compensation; detect cycles and budget exhaustion. All local. Business policy determines acceptable partial state and compensating behavior; D09/D14 provide prerequisite invariant/fencing work.

## Identity, configuration and client contracts

### D17 Provider discovery and key refresh

**Question/options.** Fixed host issuer/JWKS, allowlisted OIDC metadata discovery, or open user-entered discovery. [OIDC Discovery](https://openid.net/specs/openid-connect-discovery-1_0.html) defines issuer/metadata validation. The [openidconnect crate](https://docs.rs/openidconnect/latest/openidconnect/) includes discovery and warns against automatically following redirects because of SSRF.

**Recommendation — E/H/U.** Begin with a configured issuer allowlist, bounded HTTPS fetches, issuer equality, explicit endpoint policy and bounded key refresh. Keep trust roots outside untrusted source precedence. Do not accept token-supplied key URLs as authority. Discovery config remains a Resource, verifier implementation an adapter.

**Probe/dependencies/risks.** Local issuer fixtures cover mismatched issuer, redirects/private destinations, oversized metadata, timeout, missing/rotated keys and concurrent unknown-key traffic. D23 supports private credentials if needed. Real provider interoperability is an external acceptance gate; fixture signatures do not establish arbitrary OIDC compatibility.

### D18 Login and sessions, including a CLI

**Question/options.** Externally supplied access tokens, authorization code with PKCE and loopback callback, or device authorization for headless terminals. [OAuth Security BCP](https://www.rfc-editor.org/rfc/rfc9700) and [device grant RFC 8628](https://www.rfc-editor.org/rfc/rfc8628) define the relevant protocol controls and polling behavior.

**Recommendation — E/H/U.** For a future login-capable CLI use code+PKCE where a browser/callback is practical, device flow only when the provider supports it, and OS-protected credential storage or explicit ephemeral tokens. Avoid tokens in command-line arguments/history/logs. API access-token verification stays separate from ID-token login validation. A simple current CLI can operate on explicitly supplied verified host context without claiming login/session management.

**Probe/dependencies/risks.** After D17, synthetic servers test state/nonce/PKCE mismatch, callback collision, device polling slow-down, expiry, refresh rotation, logout/revocation and secret redaction. Host chooses session lifetime, refresh storage and client registration. Real browser/provider behavior requires accounts; building a CLI does not automatically authorize persisting user credentials.

### D19 First administrator bootstrap

**Question/options.** Keep explicit host provisioning (current), add a local administrative command, or expose a one-use bootstrap ceremony. Public first-caller-wins is not acceptable under ROM's [auth design](auth-architecture.md).

**Recommendation — E/H.** Prefer a host-local command with a protected, bounded lifetime capability. If a token ceremony is needed, store only appropriate verifier material and atomically consume the bootstrap identity with the authorized provisioning action; crash/retry must resolve the same result. Record when bootstrap authority ends. All User/provider/link changes remain core actions.

**Probe/dependencies/risks.** Race two bootstrap attempts, expire/reuse the token, crash after provisioning before acknowledgement, reopen storage and restore an earlier backup. No external provider is necessary. Host policy chooses who can obtain the capability and how a lost capability is replaced; backup restoration must not silently reopen public bootstrap.

### D20 Privileged account recovery

**Question/options.** Host-mediated recovery, prearranged recovery codes/contact, or repeated identity proofing. [NIST SP 800-63B-4](https://pages.nist.gov/800-63-4/sp800-63b.html) distinguishes recovery from ordinary authentication and describes recovery methods and notifications.

**Recommendation — H/U, local mechanics E.** Specify assurance, authority, delay, notification and audit policy before exposing recovery. Use explicit protected Resource actions and bounded single-use challenges; revoke/replace affected credentials under current authorization. Do not infer account ownership from email equality or turn bootstrap into a permanent bypass.

**Probe/dependencies/risks.** Synthetic tests exercise replay, rate/capacity limits, changed/revoked contact, partial notification failure, challenge expiry, concurrent recovery and session invalidation. Operational recovery and social-engineering resistance require host/security judgment and real participants; unit tests do not select an assurance level or certify NIST compliance. Depends on D18/D19/D23/D37.

### D21 Linking and just-in-time provisioning

**Question/options.** Explicit administrator links (current), authenticated self-linking, or controlled JIT users. OIDC identity uses an issuer-qualified subject; discovery/login specifications do not make matching email strings a safe merge key. [OIDC Core subject identifier](https://openid.net/specs/openid-connect-core-1_0.html#SubjectIDTypes).

**Recommendation — E/H/U.** Require proof of both identities or an explicitly privileged link action; preserve stable authority namespaces and uniqueness. JIT provisioning is an idempotent core action with an allowlisted issuer and deny-default initial rights. Unlinking the last recovery-capable identity needs a host policy.

**Probe/dependencies/risks.** Locally test identical subjects under different issuers, email collisions, concurrent first login, stale proof, unlink/relink, deleted/disabled User and revoked provider. Host selects consent and anti-lockout behavior; provider-specific claims need interoperability testing. D18/D20 define the surrounding ceremony.

### D22 User-delegated background work

**Question/options.** Service principals only (current), bounded stored delegation grants, or external token exchange. [RFC 8693](https://www.rfc-editor.org/rfc/rfc8693) distinguishes delegation and impersonation; token exchange does not itself define ROM's durable worker authority.

**Recommendation — E/H.** Model an explicit revocable delegation Resource specifying subject, permitted operations/targets, audience and expiry. Workers use a configured service identity plus current grant validation; attribution from an old event grants nothing. Keep tokens out of journals/receipts and recheck before execution/disclosure.

**Probe/dependencies/risks.** Locally revoke or expire grants between enqueue, claim, execution and retry; test narrower grants, tenant changes and unavailable policy. Decide whether an already accepted operation may finish after revocation. External token exchange remains a separate provider integration. D13/D18 must define scope and identity first.

### D23 Secret resolution and rotation

**Question/options.** Host-injected environment/file handles, local resolver, Vault, or a managed secret service. [Vault KV v2](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2) exposes versioned reads and conditional writes; versioned storage does not itself establish which secret revision an active adapter uses.

**Recommendation — E/H/U.** Persist a typed resolver reference with explicit pinned-version versus follow-rotation semantics. Resolve raw material only inside trusted adapter preparation, behind bounded time/size/concurrency, and retain safe provenance/version status. Discovery, errors, events, logs and low-entropy content fingerprints must not expose secret material. Configuration sources cannot grant themselves access to a new resolver namespace.

**Probe/dependencies/risks.** Use a local fake resolver and disposable Vault fixture if selected: rotation during prepare/publish, old-version deletion, resolver outage, revoked scope, malformed response and crash. Inspect all public outputs for marker secrets. Real managed stores and KMS credentials require host setup. Secret storage, runtime memory hygiene, encryption at rest and credential revocation are separate claims.

### D24 Layered partial configuration and writeback

**Question/options.** Complete single-target reload (current), authority-aware partial layers, explicit overlays, or source-file writeback. [config-rs](https://docs.rs/config/latest/config/) and [Figment](https://docs.rs/figment/latest/figment/) offer layered input mechanics; ROM's [source contract](configuration-resource-contract.md) owns authority, provenance and activation.

**Recommendation — E/H.** Keep config-rs as a narrow loader and implement authority before precedence. Specify omission, explicit null, removal, collection replacement/merge and source deletion. Record winning/shadowed safe provenance. A core action can target an explicit writable overlay; do not acknowledge a change to a file-owned value that the next reload immediately replaces. File writeback, if selected, needs compare-and-swap against the source revision and crash-safe publication.

**Probe/dependencies/risks.** Reuse deterministic local layers with untrusted high priority, false/zero/empty values, mixed owners, stale file revisions, removed sources, invalid merged values and concurrent actions/reloads. Verify protected trust roots cannot be overridden. Host policy selects layer ownership and conflict semantics; library merge defaults are insufficient. D23 precedes secret-bearing layers.

### D25 Provider activation and reconciliation

**Question/options.** Restart-only activation, prepared immutable generations, or live mutation of existing handles. Existing maintained configuration proves Resource reload/recovery, not every remote provider reconfiguration.

**Recommendation — E/H/U.** Prepare replacement handles under a bounded candidate generation, commit desired configuration through core, then reconcile durable activation intent into observed pending/active/failed/unknown state. Publish only if the expected configuration/source generation still matches, and drain old handles. Classify changes requiring restart. Local publication is not distributed rollback of remote actions; [configuration contract](configuration-resource-contract.md).

**Probe/dependencies/risks.** Crash between commit/prepare/publish/drain; delay stale candidates; rotate a secret; fail remote preparation; recover unknown activation. Local simulated/loopback providers prove mechanics without credentials. Each real provider needs an idempotency and outage contract. D23/D24 are prerequisites; avoid a second configuration entity engine.

### D26 Authorized Resource discovery and CLI clients

**Question/options.** Static compiled schema, a public descriptor dump, or authorized versioned discovery. Current `Descriptor` contains kind/version/fields, not a complete action/input/capability/policy discovery protocol; [source](../../crates/rom/src/resource.rs). The [client contract research](rom-studio-client-contract.md) is reusable for CLI clients even though Studio is excluded.

**Recommendation — E/H.** Expose a core-authorized discovery result and prefer one generic CLI over generated per-kind controllers. The selected current milestone is deliberately smaller: opt-in kind/version, visible fields/shapes and action names, wrapped by a contract version, with conservative omission of references to hidden kinds. It includes no action input schema, source/editability metadata or grant flags. Known-kind CLI commands must work without discovery. Action input schemas and supported query/transport capabilities are possible future descriptor extensions; current `Input` defines codecs without a public schema. Source/editability hints require a separate justified authorization/disclosure contract before adoption. Treat disclosed action names or any future availability hints as metadata, never operation authorization. Exact numbers/decimals need declared lossless codecs; [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259) explains numeric interoperability limits.

**Probe/dependencies/risks.** Two actors must see only their permitted kinds/fields/actions; inspect error text, counts and caches. Add a Resource without editing CLI routing, round-trip omission/null/large integers, fail visibly on unsupported codecs, and retry an unknown outcome with the same key. Invalidate discovery by registry/policy generation. Entirely local with synthetic actors; do not pass an unrestricted registry dump through an HTTP layer and call it authorized discovery.

### D27 Controlled extra attributes and live type changes

**Question/options.** Rust/native-plugin definitions with controlled extra attributes, or fully runtime-authored Resource kinds. The owner chose Rust/native-plugin definitions; fully runtime-authored kinds are rejected under that premise and are not recommended or queued as a future deliverable. Reconsidering them would require an explicit change to the owner's direction. Runtime configuration cannot silently alter the layout or executable methods of a compiled Rust struct. [Existing definition/extension boundary](rom-studio-client-contract.md).

**Recommendation — E/H/U.** Keep Rust/native-plugin definitions authoritative. If controlled extra attributes are needed, use a versioned extension field contract with accepted codecs, policy/query capability validation and explicit generations. Publish only validated extension configuration around an existing compiled definition; changing stored meaning needs D10 migration. Native behavior remains compiled or executes through a separately approved D02 bridge. This does not authorize a second runtime schema-authoring system.

**Probe/dependencies/risks.** After D10/D26, test incompatible field changes, unknown codecs, old clients, stale live subscriptions, rollback, migration interruption and secret-field reclassification. Local mechanics are feasible; user demand and acceptable complexity should justify this product profile. Zero-downtime arbitrary schema changes remain a separate unproved promise.

## Operations, providers and maintenance

### D28 Physical blob cleanup and backup completeness

**Question/options.** Retain detached objects (current), grace-period mark/sweep, or durable cleanup intentions. [S3 lifecycle rules](https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lifecycle-mgmt.html) operate on object storage; they do not understand ROM references or backup restore requirements.

**Recommendation — E/H/U.** Fence physical deletion with current reference/reservation checks, an immutable object identity, grace period and a restore/backup retention rule. Track database and blob inventories separately; backups must identify the referenced immutable bytes they require. Never delete from a stale mark result after a concurrent attachment. Restoring an old database may otherwise point to already deleted bytes.

**Probe/dependencies/risks.** Local folder/MinIO tests race attach/detach/cleanup, inject delete response loss, restart, restore old backups and retain references from pending work. Account for orphan uploads and versioned objects. Real backup media and cloud lifecycle policies require host validation. D10/D12/D14 constrain migration, retention and original-host fencing.

### D29 Advanced blob capabilities and confinement

**Question/options.** Existing bounded whole-object store, ranges, conditional replacement, signed access, versions, resumable multipart, or untrusted filesystem paths. The [blob capability research](storage-and-notification-adapters.md) separates these guarantees and warns that ordinary conditional put does not prove conditional multipart completion.

**Recommendation — E/H/U.** Add each capability independently with explicit combinations/limits. Bind signed access to an authorization snapshot and short expiry only when that loss of immediate revocation is acceptable. Keep trusted exclusive folder ownership until real filesystem confinement is implemented; logical key validation alone does not prevent symlink escape.

**Probe/dependencies/risks.** Local tests cover interrupted multipart, stale version conditions, corrupt ranges, expired signed URLs, symlink replacement and response loss; advertised capabilities must match the actual endpoint. MinIO is a useful fixture, not AWS or every S3 service certification. Host chooses revocation, object versioning and filesystem threat model.

### D30 Encryption at rest and physical erasure

**Question/options.** Host disk encryption, adapter/database encryption, envelope encryption with external keys, or per-object keys. [SQLCipher](https://www.zetetic.net/sqlcipher/design/) is an encrypted SQLite implementation, not portable encryption for redb/blob/backup data. SQLite [secure_delete](https://www.sqlite.org/pragma.html#pragma_secure_delete) has specific page-level behavior and limitations, not a guarantee covering backups, WALs or storage hardware.

**Recommendation — E/H/U.** Define protected artifacts and key ownership first; prefer host-managed encryption where sufficient, optional adapter encryption where required, and explicit backup/key rotation procedures. Distinguish logical deletion, application unreadability, cryptographic erasure and physical erasure. Do not claim erasure from deleting a Resource or rotating a reference.

**Probe/dependencies/risks.** Local tests scan known artifacts for marker plaintext, reopen under correct/wrong/rotated keys and recover backup/key combinations. Production media, KMS access, crash dumps and retained backups need host validation. D23/D28 determine secret and backup lifecycles; key loss is irreversible data loss and requires product policy.

### D31 Declarative policy engines and action roles

**Question/options.** Native Rust policy (current), [Cedar](https://docs.rs/cedar-policy/latest/cedar_policy/), or [Casbin](https://docs.rs/casbin/latest/casbin/). Each engine offers policy evaluation mechanics; integrating one does not automatically supply ROM field projection, predicate safety or freshness.

**Recommendation — E/H.** Keep native Rust as the default seam. Trial Cedar if schema-validated centrally managed policies are a real requirement, or Casbin when an existing RBAC/ABAC policy ecosystem is required. Model actions with stable identity and versioned policy inputs; keep all Resource mutation/read/field decisions in the shared core pipeline. Do not add a parallel authorization endpoint that the embedded API bypasses.

**Probe/dependencies/risks.** Differentially test engine and native policy over missing policy/data, errors, changed fields, custom actions, queries, historical delivery, policy update races and tenant scope. Entirely local. Policy editing/rollout, administrator rights and explainability are host decisions; exposing engine explanations can leak hidden attributes.

### D32 Telemetry and operational service objectives

**Question/options.** Existing bounded status counters, structured [tracing](https://docs.rs/tracing/latest/tracing/), [OpenTelemetry exporters](https://opentelemetry.io/docs/languages/rust/), or application-specific metrics.

**Recommendation — E/H/U.** Keep core instrumentation provider-neutral and payload-free: admission, queue/byte capacity, latency stages, retry classes, work age, history gaps and drain state. Export via optional adapters. Bound label cardinality; arbitrary Resource IDs/principals/inputs do not become metric labels. Use safe correlation identities rather than raw tokens or serialized requests.

**Probe/dependencies/risks.** A local collector/recording subscriber can test marker-secret exclusion, overload, exporter outage, bounded buffers and shutdown. Measure exporter overhead with disabled controls. Production SLO thresholds, paging and retention need actual workload/operations judgment; a working exporter is not an availability guarantee.

### D33 Publication, compatibility and support policy

**Question/options.** Retain local alpha packages, publish selected crates, or declare broader support. Existing release packaging passed under its stated Rust 1.99.0/lockfile profile; this report does not rerun or broaden it.

**Recommendation — E/H/U.** Keep optional adapters and documented tested toolchain/features; establish a compatibility matrix for library API, descriptors, wire, persisted formats and plugins before a supported release. Run downstream consumer/compiler fixtures and all affected adapter gates on every supported combination. Publication requires an explicit release decision, package credentials and review of licenses/advisories. Native Rust extension compatibility is source compatibility, not a stable binary ABI; [existing release evidence](mvp-release-results.md).

**Probe/dependencies/risks.** Local package extraction/consumer builds and version-skew fixtures are feasible without publication. Test old-client/new-host rejection and persisted downgrade. Feature growth can silently import a transport/DB into core, so retain dependency-tree gates. Broader platform/MSRV claims require those platforms/toolchains; D10 precedes stable persisted compatibility. Windows/macOS, alternate architectures and older Rust floors were not tested in this research. A future support matrix must exercise filesystem durability/permissions, process signals, terminal encoding, TLS/native builds and package installation on each claimed target; compilation alone does not prove those runtime profiles.

### D34 Human authors and representative performance

**Question/options.** Compiler fixtures and agent-written consumer examples, observed human walkthroughs, or representative production workload trials. The [authoring trials](authoring-crate-trials.md) and [cache measurements](cache-prototype-results.md) explicitly delimit what was tested.

**Recommendation — U.** Keep machine-checkable ergonomics regressions, then recruit real Rust users for a task protocol: declare two Resources, custom action, query/live observation, config ownership and a failure diagnosis without per-kind infrastructure. Record completion, errors, documentation lookups and API misunderstandings. For performance preselect concurrency, payloads, duplicate mix, retention sizes, durability, latency goals and regression budgets before comparing alternatives.

**Probe/dependencies/risks.** Local fixtures, scripts and benchmark scenarios can be prepared autonomously. Human performance, satisfaction and operational throughput cannot be fabricated from agent execution. User recruitment and representative workloads are explicit external evidence gates, not blockers to preparing the study or improving demonstrated defects.

### D35 Production Studio

**Question/options.** Retain historical mock/research, later implement a generic frontend, or never require Studio. The owner's revised goal selects CLI work now and excludes Studio implementation.

**Recommendation — U.** Preserve [product](rom-studio-product-research.md), [frontend](rom-studio-frontend-research.md), [controls](rom-studio-controls-research.md) and [client-contract](rom-studio-client-contract.md) research. Reuse D26 discovery/codec/outcome work in the CLI. Do not implement, deploy or claim completion of production Studio in this goal.

**Future probe/dependencies/risks.** If separately authorized later, require generic two-Resource rendering, authorization-aware metadata, accessible controls, exact-value editing, current policy and uncertain-action recovery, plus real user validation. D26/D27 must not acquire UI-specific semantics merely to preserve the mock.

### D36 Direct Serde codecs and descriptor extensions

**Question/options.** Existing ROM-derived codecs, manually implemented codecs, or a restricted direct-Serde profile. The [resource specification](../../openspec/changes/establish-rom/specs/resource-model/spec.md) requires one authoritative public name/presence/shape contract and explicitly calls for rejection of incompatible serialization settings.

**Recommendation — E.** Retain ROM codecs as authority. Add a direct-Serde mode only with a documented allowlist of attributes/representations and generated consistency checks. Richer nested/custom descriptors must retain stable type identity, encoding version and supported operations; an unknown custom type is not an unconstrained JSON editor or universally queryable value.

**Probe/dependencies/risks.** Compiler/runtime fixtures cover renamed fields, aliases, flattening, skipped values, internally/externally tagged enums, custom serializers, missing/null and query equality against actual wire bytes. Reject unsupported combinations at registration/compile time. Entirely local; D10/D26 determine evolution/discovery compatibility.

### D37 Real channels and external effect guarantees

**Question/options.** Named local Rust channels (maintained), SMTP/email, webhooks, cloud messaging, or a provider-specific idempotent API. ROM already commits durable intentions and bounds retry; provider acceptance, recipient delivery and human attention are different outcomes. RabbitMQ's [confirm contract](https://www.rabbitmq.com/docs/confirms) is one concrete example of that distinction.

**Recommendation — E/H/U.** Keep the typed named-function seam and require each provider adapter to document accepted/retryable/permanent/unknown results, identity propagation, deduplication horizon and reconciliation. Store stable delivery identities, never executable closures or credentials. Receiver deduplication can reduce duplicate effects; a local claim fence cannot force an unrelated provider to reject a late stale sender.

**Probe/dependencies/risks.** Local SMTP/webhook/broker receivers can reproduce acceptance followed by lost acknowledgement, retries after restart/restore, expiry, rate limiting and terminal failure inspection. Real provider credentials, quotas, delivery evidence and their deduplication window are separate host gates. No adapter-independent exactly-once external effect promise follows; D11/D12/D22/D23 define relevant retention, authority and secret constraints.

## Dependency ordered execution recommendation

1. **Stage 1 — shared correctness and clients.** Finish the current core/API/resilience and CLI work with authorized discovery (D26), while specifying migration/compatibility (D10) and retry identity horizons (D11). Keep existing maintained semantics as the oracle. This stage resolves the contracts most later features depend on.
2. **Stage 2 — independent local profiles.** Prototype RabbitMQ (D01), additional simple query operators (D06), cleanup rules (D12/D28), secret/configuration layers (D23/D24), configured discovery/login/bootstrap mechanics (D17–D19), and optional safe telemetry (D32) in isolated fixtures. Each is a separate future implementation decision; this report does not enqueue unrequested provider writes.
3. **Stage 3 — deployment and lifecycle guarantees.** Implement selected migrations/retention, enforce relationship integrity (D09), indexed/snapshot reads (D08), tenancy (D13) and cooperative writers (D14) with one chosen server adapter (D15). Complete provider activation, recovery/linking, backup/key policy and packaging evidence (D20/D21/D25/D28/D30/D33/D36/D37). Declare exact tested topologies.
4. **Stage 4 — measured breadth.** Adopt single-flight/cache/Salsa (D03–D05) only after correctness and workload gates. Add selected joins/aggregates (D07), delegation (D22), advanced blobs (D29) and policy engines (D31) where a concrete consumer warrants them.
5. **Stage 5 — extension/product profiles.** Consider WASM (D02), controlled extra attributes around Rust/native definitions (D27), additional backend families and explicit atomic multi-Resource operations (D16), with their prior stages satisfied. Fully runtime-authored Resource kinds are not queued under the accepted premise. Continue human/workload evidence (D34) throughout. Studio (D35) remains excluded unless separately requested.

No stage number claims equal effort, a delivery date or a commitment to implement every alternative. Research completion means each known deferred topic has a question, options, evidence, recommendation, acceptance probe, dependencies and remaining judgment. Implementation completion requires the selected capability's executable evidence and updated supported-profile documentation.

## Local feasibility and remaining external gates

Local work can prepare contract fixtures, synthetic OIDC/secret/provider servers, two-tenant adversarial inputs, process crash/restart tests, cache/Salsa measurements, offline migrations and disposable PostgreSQL/RabbitMQ/MinIO/Vault environments. Package availability, writable persistent scratch and unique bounded ports must first be verified; the observed missing `nixpkgs` channel prevents assuming a one-command broker setup. No local broker or new capability conformance trial ran for this report.

External gates are narrowly identified: real provider accounts/client registrations and credentials; managed database/cloud/failover configurations and cost limits; physical media/KMS/backup operations; production workload and service-objective selection; real author participants and product choices about tenant sharing, recovery assurance and retained data. Synthetic tests can prove local mechanics while these remain explicitly unclaimed. No GitHub Actions, package publication, production external writes or Studio deployment are part of this research.
