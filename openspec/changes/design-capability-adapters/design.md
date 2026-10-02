# Capability contracts and interchangeable adapters

## Confirmed direction and evidence

Resource is ROM's sole domain entity. Its behavior does not depend on the host's database, blob store or notification provider. The host registers named native Rust implementations; core knows their semantic contracts and declared capabilities. Actions, events, receipts, delivery intentions and blob references are supporting values. WASM and a universal dynamic plugin ABI remain deferred.

The owner explicitly extended database independence to all integration seams. This is not one untyped `execute(command)` interface: persistence, blobs, notifications, identity verification and transports have distinct responsibilities. Existing field and auth contracts follow the same principle. Tokio/Rayon remain execution services.

Evidence: [relational research](../../../docs/research/storage-relational-backends.md), [non-relational research](../../../docs/research/storage-nonrelational-backends.md), [blob/channel research](../../../docs/research/storage-and-notification-adapters.md) and [executed adapter probes](../../../docs/research/capability-prototype-results.md). These sample families and bounded implementations, not every product. No production backend is selected.

## Ownership and composition

The core defines identities, typed values and field semantics, operation preconditions, action/event meaning, authorization, retry policy, query semantics and acceptable outcomes. Adapters implement storage layout, physical encoding, transactions, vendor protocol, connection lifecycle and native error translation. Protocol value encoding remains consistent with the accepted resource/field contract; a physical representation cannot reinterpret its semantics.

Host setup supplies implementations and credentials. Normal resource authors use resource operations and named capability handles without SQL, object-store SDK values or SMTP messages. Native adapters and callbacks are trusted code. Infrastructure access is not a public authorization bypass: transports and application resource operations still enter ROM's authorized path.

Register required capabilities and finite limits, check them against configured implementations, then freeze the registry. Optional capability support is explicit and may be configuration-dependent. Unsupported calls fail before effects; do not silently downgrade conditional writes, snapshot reads, durability or bounded execution. Runtime errors remain possible after successful registration.

The intended Rust packaging separates core-owned contracts from implementation crates, with a convenient facade for ordinary authors. Feature-enabled adapters may be reexported by the facade; the domain contract crate must have no database/cloud/provider driver dependency. Concrete async signatures, object safety and generic dispatch are follow-up interface choices; synchronous database calls must not block Tokio workers in the production runtime.

## Persistence protocol

Persistence is an operation-level protocol, not a generic SQL connection. The conceptual surface is:

| Operation | Core-owned input | Semantic result |
| --- | --- | --- |
| Read resource | Resource identity and required consistency | Resource/revision or absence, under a stated freshness contract |
| Commit transition | Expected revision/absence, scoped command identity, canonical request fingerprint, replacement/tombstone, events and pending effect intentions | Committed outcome, conflict, identity mismatch, definitely not committed, unsupported or unknown outcome |
| Resolve command | Scoped command identity | Found outcome, absent at this read, expired identity evidence or unavailable |
| Scan journal | Scope, opaque continuation and page budget | Retained committed records and safe continuation, or explicit gap/invalid token |
| Query | Typed predicate, projection, ordering, budget and consistency requirement | Supported bounded result/page, or explicit unsupported contract |

Names describe semantics, not stabilized Rust methods. Minimum durable mutation scope is one resource and its associated protocol records within the adapter's declared limits. Multi-resource transactions, predicate read-set validation, query snapshots and indexes are separate capabilities. An adapter may offer mutation persistence without supporting every reactive query shape.

The adapter atomically arbitrates command identity and revision, writes the complete bundle, then acknowledges according to its durability profile. A same-identity/same-request retry resolves the original outcome; changed input is rejected. Core reevaluates state-dependent behavior after revision conflict; an adapter must not silently rerun domain code against a different state. Cross-resource authorization/invariants require declared read-set protection or a separately specified policy freshness rule, not an assumption that a single-row CAS covers them.

Unknown outcome is first-class. A canceled future, network timeout or commit error may accompany a successful or still-running commit. An absent receipt is only absence at that observation. Recovery uses authoritative lookup and/or resubmission with the same atomically arbitrated identity. Receipt expiry requires an explicit retry horizon and retained identity evidence or rejection of expired commands; garbage collection must not silently re-enable old operations.

Durability profiles distinguish process recovery, power-loss persistence and selected replica/failover behavior. They depend on adapter configuration and deployment; no universal `durable=true` proves all three. Startup validates required settings where possible, with operational assumptions documented.

### Optional pre-persistence deduplication

The owner proposed rejecting duplicates before the write layer when it improves performance. Candidate optimization: bounded in-flight single-flight keyed by trusted command scope and canonical request identity, plus a bounded cache of confirmed outcomes. Reject different input under the same key rather than joining it; authorize every caller before disclosing an outcome. This is an optimization, not a replacement for atomic durable identity arbitration. Separate hosts, process restarts and uncertain commits bypass a process-local cache. Cache validity must respect receipt retention, namespace generation and current access policy.

No zero-overhead claim is made. Benchmark unique requests, duplicate bursts, multi-instance duplicates, payload hashing, cache misses, eviction and canceled waiters before adoption. Preserve work supervision when the first caller disconnects. A data lake may serve archival analytics but is not required for this request-path index. Do not deduplicate distinct intended operations merely because their payloads happen to match. This fast path is proposed, not implemented by the current adapter probes.

## Journal, queries and reactivity

Core treats continuation tokens as opaque, scoped and versioned. An adapter must not advance beyond a later-visible retained event. Allocated sequence numbers and wall clocks alone do not prove commit order; implementations may use serialized publication, native safe tokens or partitioned progress with fair discovery. Required ordering is per resource; a global total order is not mandatory.

Query semantics come from ROM's field contract: missing/null distinctions, exact comparisons, ordering and tie-breaking cannot inherit inconsistent vendor defaults. The adapter compiles these semantics or rejects them. Index-backed guarantees and bounded full-scan fallback are explicit execution capabilities, not hidden decisions. A paginated scan is not necessarily a snapshot.

Live-query initialization needs a proven snapshot/journal handoff or a documented reconciliation procedure. Native CDC and notifications are optional wakeups; they cannot reconstruct missing ROM actions, permissions or receipts from arbitrary direct database writes. Cooperating ROM writers use the same protocol. Bypass writes require a separate supported integration profile; they are not implicitly supported. Retention gaps require resync rather than silent cursor reset.

## Blob storage

Use logical keys and a separate blob contract for whole-object put, read, metadata and idempotent deletion. Optional capabilities include create-only, conditional replacement, ranges, listing, versions, signed access and resumable multipart. Capability combinations matter: ordinary conditional put does not prove conditional multipart completion.

Core owns accepted key and value semantics; adapters own paths, buckets, SDK streams, physical temporary objects and provider tokens. Opaque version tokens are not content hashes. Bounds cover bytes, chunks, outstanding operations and deadlines under an explicit profile. Atomic complete-object visibility and crash durability are separate guarantees. Cancellation before submission may be known not applied; cancellation after submission may be unknown.

A local folder requires either trusted exclusive ownership or a separately enforced filesystem confinement mechanism. Logical key validation alone cannot prevent symlink escape. S3-compatible endpoints pass their own conformance profile; testing MinIO does not certify every endpoint or AWS deployment.

Resource-plus-blob operations are not distributed transactions. A candidate strategy uploads immutable unique content before committing its resource reference, with orphan cleanup after a grace/reference check. Detachment commits cleanup intention before asynchronous deletion. Uploads do not hold database transactions open. Lifecycle/garbage-collection details remain a follow-up implementation contract.

## Notification channels and external effects

Register a named channel with a versioned payload codec and an ordinary typed async Rust function. A framework wrapper performs type erasure and validation; the durable queue stores data, never closures. Email, webhooks and custom functions implement the same delivery role without making one provider's address/message types universal.

Resource transition and requested delivery intentions commit atomically through the persistence adapter. A worker claims durable work after commit, calls the channel outside the database transaction and records the outcome. Supporting work-store operations include claim/lease, conditional acknowledgment, reschedule and inspection of terminal failures; their storage mechanics remain adapter-private. Claim generations fence stale acknowledgments, but do not fence an external provider.

Channels report accepted, retryable, permanent or unknown outcomes. Acceptance means downstream acceptance, not inbox delivery or human attention. Stable delivery identity survives retries. Attempt/deadline/backlog limits, backoff and inspectable terminal failure belong to shared runtime services. Unknown outcomes use explicit reconciliation, retry or review policy. Exactly-once external effects require stronger recipient support and are not a blanket ROM guarantee.

The notification probe's SQLite outbox and the database probe are separate experiments. Their production integration must prove that state, revision, receipt, events and effect intentions share one atomic commit. Prior tests of each half do not prove this combined bundle.

## Resilience and acceptance

Use identical semantic suites for each adapter and failure injections at actual boundaries: conflicting writers, rollback, process exit, response loss, in-flight commit with missing receipt, partial input, unsupported conditions, provider acceptance before local acknowledgment and stale leases. Include deliberately failing alternatives so tests demonstrate their discriminating power.

Evidence identifies source, lockfile, toolchain, deployment and exercised fault. Process restart is not a power-loss test; local S3 interoperability is not universal cloud certification; receiver dedup in a test is not a provider promise. Preserve public authoring simplicity while locating complexity inside ROM.

## Remaining choices

Exact async Rust interface and typed query subset; supported durability/consistency profiles; receipt/journal/outbox retention; distributed cursor generation and writer topology; production blob bounds and confinement; payload/privacy rules; retry-after and unknown-delivery policy. These need focused specifications and integration evidence, not a technology-specific core.
