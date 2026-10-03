# Interchangeable adapters and resilience: executed findings

Date: 2026-10-02. The coordinator independently verified three disposable workspaces in the persistent NixOS container. These extend the [earlier prototypes](prototype-results.md); they are not one integrated production library. The question was whether core-owned contracts can support different providers without changing resource behavior.

## Persistence: relational and transactional key-value

Source: [persistence adapter workspace](https://github.com/pmikstacki/ROM/tree/prototype/persistence-adapters/prototypes/persistence-adapters), [implementer assessment](https://github.com/pmikstacki/ROM/blob/prototype/persistence-adapters/prototypes/persistence-adapters/ASSESSMENT.md).

The same core and application execute against rusqlite **0.40.2**, bundled SQLite **3.53.2**, and redb **4.3.0**. Core contains no SQL, database transaction or driver dependency. It requests conditional atomic transitions, resolves receipts and scans journal pages through semantic values. These are real scratch databases, not an in-memory fake presented as another backend.

The independent verifier passed formatting, Clippy with warnings denied, a default-feature build, both application configurations and **18 tests**: ten shared conformance cases, six failure cases and two process-recovery tests. Each recovery test exercises four subprocess exits that bypass destructors.

Observed guarantees include complete state/revision/receipt/event persistence after reopen; one winner in a revision race; same-command deduplication; rejection of changed requests under the same identity; rollback after each physical write boundary; and scoped journal continuation. SQLite uses independent connections for concurrent/in-flight checks; redb uses separate read/write transactions.

The most useful resilience case pauses an actual native transaction before commit. An independent reader sees no receipt. Then the original and a racing retry resolve to one committed transition. **Absent receipt does not prove rollback.** A second case commits successfully but returns an injected unknown response; reopen plus same-identity retry retrieves the original outcome without another event set.

The journal's serialized counter is safe for the two tested embedded engines. It is not a recipe for distributed commit ordering. This synchronous probe lacks query/live reads, authentication, no-op action policy, multi-resource operations, retention and effect intentions. Its complete-transition comparison is an experimental idempotency representation, not final fingerprint/privacy design. Real process exit was tested; power loss, replication and every native commit error were not.

**Carry forward:** operation-level conditional commit and outcome lookup, opaque cursors, explicit limits/capabilities and driver-free contracts. Reject a core-facing SQL connection as the universal interface. redb is evidence of model independence, not a production backend selection.

## Blob storage: folder and real S3-compatible server

Source: [blob workspace](https://github.com/pmikstacki/ROM/tree/prototype/blob-adapters/prototypes/blob-adapters), [implementer assessment](https://github.com/pmikstacki/ROM/blob/prototype/blob-adapters/prototypes/blob-adapters/ASSESSMENT.md).

Core owns a small async blob interface; adapters privately wrap **object_store 0.14.2**. Its only core dependency is futures-core. The independent standard verifier passed formatting, Clippy, documentation with warnings denied, dependency isolation and six local cases. Two S3 tests are visibly ignored there. A separate run started disposable loopback **MinIO RELEASE.2024-09-22T00-33-43Z** and passed **all eight tests**, then removed its scratch data and stopped the server.

Both implementations pass shared round-trip, overwrite, missing-delete and failure cases. Byte/chunk/count limits bound the probe's staging buffers. Failed or canceled input before provider submission preserves the old object. Unsupported conditional writes fail before mutation. The experiment deliberately does not emulate compare-and-swap using a racy read followed by overwrite.

A TCP relay lets MinIO commit an upload and then drops its successful HTTP response. The caller gets Unknown; a separate read obtains the complete object. This is actual protocol-level lost-acknowledgment evidence. It proves that a failed response cannot safely be equated with no stored effect.

The folder implementation's symlink behavior was also tested: its prefix is **not a security sandbox**. The supported experiment uses a trusted directory; production confinement needs an explicit mechanism or an equally explicit trust contract. Logical key validation alone is insufficient.

**Carry forward:** provider-private SDKs, declared capabilities, bounded streams and honest uncertain outcomes. The probe stages small blobs, has no multipart support, admission control, resource/blob garbage collection or crash-durability proof. It certifies the exercised MinIO setup, not all S3-compatible systems. MinIO is a separate local test server, not linked into the MIT core.

## Notifications: ordinary functions with durable work

Source: [notification workspace](https://github.com/pmikstacki/ROM/tree/prototype/notification-channels/prototypes/notification-channels), [implementer assessment](https://github.com/pmikstacki/ROM/blob/prototype/notification-channels/prototypes/notification-channels/ASSESSMENT.md).

The core registry accepts named ordinary async Rust functions with checked/versioned payloads. One dispatcher invokes differently typed channels without provider branching. SQLite is confined to a separate scratch outbox implementation. No real email or external service was contacted.

The independent verifier passed formatting, Clippy, build, the custom-function example and **12 tests**. They cover actual SQL rollback/reopen, durable intent recovery, bounded attempts and backoff, permanent failures, callback timeout classified as Unknown, stale acknowledgment fencing, schema mismatch and two different registered function types.

A deterministic receiver applies its effect and pauses before acknowledgment. The test then aborts the worker. After reopen and lease expiry, retry uses the same delivery identity. Without receiver deduplication there are **two effects**; with receiver-owned deduplication there is **one effect across two attempts**. The outbox guarantees recoverable attempts within policy; it cannot manufacture recipient idempotency. Repeated worker loss consumes the durable attempt budget rather than creating unlimited retries.

**Carry forward:** atomic intention creation, stable delivery identity, typed functions, explicit outcomes and generation-fenced claims. The probe uses caller-driven ticks and a synchronous scratch SQL implementation; it does not establish production scheduling, provider receipt lookup, jitter, clock discipline, privacy controls, real email semantics or OS/power-crash recovery. Worker abort and database reopen are the exercised interruption.

## Combined conclusion and remaining integration

The owner's architectural premise is supported for these bounded examples: define semantic capability contracts in ROM, supply providers from host configuration, and keep technical machinery out of resource authors' code. Persistence, blobs and channels need distinct contracts rather than a universal untyped plugin command.

Idempotency has separate boundaries. Resource mutations arbitrate durable command identity with state/events. External deliveries preserve stable identity, but preventing duplicate receiver effects requires supported recipient behavior. Blob overwrite is not universally safe to retry when another writer can intervene; immutable keys, conditional operations and reconciliation have separate roles.

The [capability-adapter proposal](../../openspec/changes/design-capability-adapters/proposal.md) records the contract, supported by the [relational](storage-relational-backends.md), [non-relational](storage-nonrelational-backends.md) and [blob/channel](storage-and-notification-adapters.md) research. Production tasks remain unchecked.

All three workspaces ran on Rust/Cargo **1.99.0** and retain lockfiles and exact commands. There are **38 tests across the three suites**; that count is evidence bookkeeping, not a quality score. Combining the persistence commit with outbox intentions, full typed resource authoring, policy enforcement, live queries and shared execution supervision is still a new integration milestone. The separate probes do not prove those guarantees compose automatically.
