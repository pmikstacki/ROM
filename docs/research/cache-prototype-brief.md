# Internal cache and incremental-read experiments

Date: 2026-10-02. The owner authorized research and execution of the proposed deduplication comparison. Cache behavior is an internal ROM feature, not a public interchangeable-provider contract. The initial implementation uses a dependency. Vendoring or specialization needs evidence of a benefit.

## Shared question

Does a pre-persistence fast path reduce end-to-end work without weakening durable idempotency, current authorization or lifecycle control? Compare the same action path with four configurations:

- No optimization.
- ROM-owned single-flight only.
- Single-flight plus Moka.
- Single-flight plus quick_cache. Canonical request semantics, durable storage, policy checks and workload must remain equivalent.

Salsa has a separate question: can tracked dependencies reduce pure derived-read recomputation while preserving result correctness and policy invalidation? Its numbers must not be compared as if it implements the same mutation-deduplication workload.

## Correctness before timing

- Concurrent retries with one scoped key and canonical input yield one committed mutation/event set and equivalent outcomes.
- Different input under the same key is rejected both while work is in flight and after its result is cached. Distinct intended operations with identical payloads are not merged.
- The loss of the first caller does not cancel shared admitted work or strand other waiters. Work owns its admission and lifecycle; the cache does not.
- Expiry, eviction and process restart fall back to authoritative durable arbitration. Two independent caches do not undermine one durable identity boundary.
- Every result delivery respects current access policy. Tenant, principal, operation and storage-generation scope must not be accidentally conflated.
- Unknown outcomes do not become cached success or terminal rollback. Retries resolve through the same durable identity.
- Admission, entry weight and payload limits are explicit. Best-effort cache capacity is not a strict process-memory guarantee. In-flight entries are not evicted while their work can still commit.

Use deterministic gates for races, scratch storage and failure controls. Keep production cache choice private. No existing production behavior is replaced by this probe.

## Measurement protocol

Build release binaries separately from verification. Retain lockfiles, source revision, compiler, parameters and raw machine-readable observations. Avoid competing benchmarks during measured runs. Rotate the implementation order. Repeat samples. Distinguish warm-up, cold starts, and measured requests.

Workloads include unique operations, repeated completed operations, concurrent duplicate bursts and mixed traffic, with explicit key distributions and payload sizes. Report request counts, successful operations, actual mutations/events, durable-store calls, end-to-end latency quantiles and throughput. If instrumentation is present, record CPU, memory, and allocation measurements. State which metrics are unavailable. Do not infer them. A simulated store and real SQLite must not be labeled interchangeably.

Measure both cached benefits and unique-request overhead. Cache-hit throughput alone is not an adoption argument. Report dispersion and workload boundaries, not universal speed multipliers. No strict latency budget or zero-overhead promise has been adopted. A valid result may favor single-flight without retained outcomes, or retaining no optimization by default.

## Salsa acceptance

Use authoritative resource facts as inputs, then observe repeated reads, relevant edits, unrelated edits and explicit policy changes. Count actual tracked-function execution. Compare the outputs with the expected results. The test should deliberately omit or leave a dependency untracked to demonstrate the invalidation hazard. Application code must not silently depend on external mutable state inside a memoized query.

The prototype must say what dependency registration and memory retention ROM would need to own. Salsa does not replace persistence, the action pipeline, durable events or external-effect recovery. Any follow-up integration is justified by the read workload, not by its prior use in another project alone.
