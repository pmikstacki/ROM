# ROM prototype findings

Date: 2026-10-02. The initial execution/resource-flow batch and subsequent authoring/library experiments are complete and independently verified. These are disposable experiments, not a ROM release. The prototypes exercise complementary layers. They are not competing implementations of the complete framework. The fixed product premise is one resource definition that supplies generic storage, operations, endpoints and reactive behavior.

## Execution contract: verified

Source: [execution prototype branch](https://github.com/pmikstacki/ROM/tree/prototype/execution-contract/prototypes/execution-contract). Its README retains the exact commands, dependency lock and limits. The implementer's [written assessment](https://github.com/pmikstacki/ROM/blob/prototype/execution-contract/prototypes/execution-contract/ASSESSMENT.md) compares the exercised alternatives.

The implementer and an independent parent-agent run both passed formatting, Clippy with warnings denied, and the locked release executable in the persistent NixOS container. The parent repeated the run offline using the retained lockfile. Rust/Cargo 1.82, Tokio 1.41.1 and Rayon 1.12.0 were exercised; these are experimental pins, not a production toolchain decision.

| Question | Executed observation | Conclusion |
| --- | --- | --- |
| Where does CPU admission belong? | Correct bridge rejected a third job before submission at capacity two. Cancelling both reply waiters left both permits occupied until their CPU jobs completed. | The actual CPU job owns its permit. Caller interest is a separate lifetime. |
| Does the probe detect the wrong alternative? | A deliberately waiter-owned permit was released on cancellation, allowing a third CPU closure to start while the original two remained blocked. | The alternative is observably incorrect for this capacity contract. This is a tested comparison, not merely a preference. |
| What owns completion? | An independently retained supervisor collected both abandoned replies without panicking; shutdown remained pending until the jobs were released. | Work supervision and draining must outlive individual caller futures. |
| How does a CPU panic behave? | An injected unwind became an explicit error, restored its permit, and allowed a subsequent job to succeed. | An unwind boundary can preserve accounting; it does not establish that arbitrary plugin state is safe to reuse or recover process-aborting panics. |
| Can async work progress alongside CPU jobs? | Both computed outputs matched sequential references; all 20 timer observations overlapped the CPU workload in the independent run. | The separation works for this workload. No throughput, fairness, latency guarantee or comparison against another runtime was measured. |

The independent timing observation was 68 ms for the two CPU jobs and approximately 1.33 ms p95 timer lateness. The shared host, tiny sample and synthetic work make this an observation only, not a performance claim.

**Recommended pattern:** host-owned shared executors, admission before submission, permits owned by actual work, explicit completion supervision, and shutdown that stops intake before draining. The finite probe still retains completed results until drain; a long-running implementation needs continuous reaping and independent limits for ingress, payloads and retained results.

## Resource flow: end-to-end evidence

Source: [resource-flow prototype branch](https://github.com/pmikstacki/ROM/tree/prototype/resource-flow/prototypes/resource-flow). Its README records authoring examples, the executable probes and the implementer's assessment.

The implementer and parent independently passed the final twelve real loopback HTTP/SSE probes, formatting and Clippy with warnings denied. The parent repeated the checks offline from the retained lockfile. The test harness starts the Rust server, uses a scratch SQLite database, exercises HTTP operations and streaming, kills/restarts the process, and removes its data. It does not run the private Go RIM code.

| Question | Executed observation | Conclusion |
| --- | --- | --- |
| Does one declaration provide a useful backend surface? | Three unrelated kinds used the same routes, SQL tables and event path. The third kind required three declaration lines and zero new repositories, handlers or stream functions. | The central authoring premise works for this small contract. The macro API and dynamic JSON representation are provisional. |
| Do extensible fields work without kind-specific plumbing? | A custom Rust field type rejected invalid values through the same validation path; undeclared fields were rejected and exported metadata preserved nullability. | Native field registration can remain independent of HTTP/storage code. Canonical codecs and rich query capabilities still need proof. |
| Are patch semantics explicit? | Omission preserved values, false overwrote true, zero was retained, nullable null was stored and null on a nonnullable boolean was rejected. | Transport decoding can preserve distinctions throughout the generic mutation path. |
| Can actions be safely retried? | No-op kept revision/history unchanged while retaining its receipt; the same request identity returned the stored outcome and conflicting reuse was rejected. | Persisting outcomes alongside transitions is a useful generic contract. Network loss after commit was not separately injected. |
| Can state and events diverge on rejection? | Injected failure after state/event/receipt SQL, before commit, rolled back all three; retry under the same identity succeeded. | The transaction boundary works in this SQLite adapter. This is not a crash-during-commit or cross-database guarantee. |
| Can concurrent requests silently overwrite? | Two HTTP actions with the same expected revision produced one success and one conflict. | Revision checking prevents that overwrite within this serialized single-connection host; independent database writers were not tested. |
| Does replay survive process memory loss? | SIGKILL/restart preserved resources and SSE replay using a durable cursor. Forced exit after a reaction target committed but before cursor checkpointing did not duplicate its event after restart. | Persisted history and action receipts handle the exercised replay window. This is not exactly-once external delivery. |
| Does a missed wake-up lose committed data? | The process suppressed notification hints; an already connected SSE client still received the committed event from journal polling. | Notifications can remain disposable wake-ups while the journal owns recovery. No broker or network-partition protocol was tested. |
| Is provider-neutral auth implemented? | Claimed-owner checks scoped reads, writes, receipts and event results. | Only a scoping stub was exercised. Caller-supplied identities are not authentication; service-principal policies, field redaction and revocation remain unimplemented. |

**Recommended pattern:** one descriptor registry, one action transaction and one durable journal. Generic HTTP handlers translate to that action path; reactions use it too. SQLite and validated JSON made the authoring experiment inexpensive, but neither establishes the final storage/query/typed API design. Tiny validators do not need Rayon for speed; its use here demonstrates an extension boundary only.

Independent review found an actual counterexample: a valid maximum-length source ID produced an invalid longer reaction target ID, blocking later work on the shared cursor. The regression reproduced zero target events instead of two. The corrected implementation uses a bounded, startup-validated declaration namespace plus journal sequence. The final probe confirms processing of both that event and a later short-ID event. Review also caught lost nullability metadata and an earlier fault location that did not exercise receipt insertion; both were corrected and verified. This distinguishes exercised weaknesses from speculative production gaps.

## Reusable library and live queries: subsequent experiment

Source: [library slice](https://github.com/pmikstacki/ROM/tree/prototype/library-slice/prototypes/library-slice), with its [implementer assessment](https://github.com/pmikstacki/ROM/blob/prototype/library-slice/prototypes/library-slice/ASSESSMENT.md). The coordinator independently passed formatting, Clippy with warnings denied, nine public integration tests, two internal regressions and the application example on Rust/Cargo 1.99.0. The example observes one open task, completes it through a custom action, and observes an empty result without application subscription plumbing.

- A second resource kind uses the same storage, actions and reads with only declaration/registration.
- Create, update, custom action and deletion change filtered query membership through one mutation path.
- Default-deny host policy governs operations. Policy replacement without a data event removes rows or terminates access; this is a trusted actor/policy seam, not authentication.
- Snapshot creation and subscription registration share the mutation lock. An explicit gate exercises concurrent initialization; its scheduling signal does not prove which OS instruction a writer reached.
- Two subscribers share one actor/query registration; 24 commits coalesce into a latest snapshot while the journal retains all 24 events. Channel versions/query count are bounded; row counts, bytes and history retention are not.
- An injected SQL trigger failure during event insertion rolls back the preceding resource write and publishes no changed snapshot.
- Shutdown joins the shared worker, closes streams and rejects further operations. Temporary admission pressure preserves a retryable pending delivery.

Review caught stale authorization: the first implementation checked current policy against a buffered historical row. Changing a visibility/ownership field could therefore leave an old result readable before refresh. Delivery now recomputes from authoritative state whenever its generation or policy epoch is stale. The coordinator removed that refresh temporarily: the deterministic stale-buffer test failed at the assertion that revoked data is absent. Restoring it passed the full verifier. Journal inspection additionally requires both current and historical row visibility under current policy.

This is a transport-free library, with no HTTP dependency, but it remains disposable: a declarative descriptor macro, JSON business functions and string selectors are not the final typed derive/fluent interface. It supports one boolean equality predicate, a single runtime owning writes and serialized SQLite access. Worker-panic propagation, total memory bounds, real identity providers, field redaction, idempotency receipts and reactions are not implemented in this slice. Separate earlier prototypes are not automatically integrated guarantees.

The [derive/builder trials](authoring-crate-trials.md) supply complementary authoring evidence. They too were independently rerun on Rust1.99. The earlier execution executable and twelve resource-flow probes also passed after that toolchain upgrade; their original lockfiles remain experimental.

## What the research adds

[The six-framework comparison](state-of-art-resource-frameworks.md) supports borrowing different patterns for different jobs: Ash for declarative resource/action/type contracts, Feathers for uniform in-process and transport invocation, and Convex for automatically refreshed queries and transactional effect separation. This is a judgement from primary documentation; upstream frameworks were not installed or benchmarked.

The subsequent reusable-library experiment below exercises filtered live queries. A durable event cursor alone did not prove membership changes, setup races, policy freshness or slow-consumer behavior.

## Decisions and remaining evidence

- Fixed: resource is the sole domain entity; ordinary application plumbing is derived; Tokio and Rayon execute work; extensions begin as Rust contracts.
- Supported by execution evidence: work-owned permits and independent supervision are preferable to waiter-owned capacity.
- Experimental: SQLite/shared JSON storage, declaration macro shape and dependency pins. These remain replaceable.
- Still needed before claiming a production core: complete query/type contracts, a reference persistence decision, policy freshness, durable reaction lifecycle, bounded queues/results, realistic load and failure testing. The resource prototype also lacks the execution prototype's independently retained supervisor and explicit drain protocol; combining their lessons requires new implementation and verification.

Implementation tasks remain uncompleted in OpenSpec. The completed prototype task records experimental evidence only.
