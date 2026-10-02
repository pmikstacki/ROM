# ROM research and prototype program

Status: this research/prototype batch is complete. See [verified findings and limits](prototype-results.md) and the [ranked framework comparison](state-of-art-resource-frameworks.md). The resource premise is fixed; the experiments test implementation choices, not whether to replace that premise. Runtime: Tokio for async execution and I/O, Rayon for CPU work. All experimental code is disposable and isolated on prototype branches. Main remains a specification/research repository; the later `demo/` application has not started.

## Fixed product premise

A single resource declaration supplies standard persistence, operations, queries, transport endpoints and committed-change subscriptions through generic machinery. Applications add resource definitions and domain behavior rather than bespoke repositories, CRUD handlers or publication plumbing. Actions and events are messages concerning resources, not competing domain entities. Custom Rust field types and provider-neutral access control are extensibility contracts. RIM's original broader vision is the starting point, not an application-specific monitoring system.

## Research tracks

| Track | Question | Evidence required |
| --- | --- | --- |
| State of the art | Which declaration-driven frameworks solve parts of this premise well? | Primary-source comparison of declaration, operations, data layer, live behavior and policy; explicit differences and gaps |
| Resource flow | Can multiple unrelated kinds obtain useful end-to-end behavior from declarations alone? | Runnable Rust prototype with shared storage/HTTP/stream path, concrete declarations and contract probes |
| Execution contract | Can Tokio/Rayon keep work bounded and correctly accounted for through cancellation and failure? | Deterministic executable probes, including an intentionally broken negative control |

## Resource-flow experiment

Branch: [`prototype/resource-flow`](https://github.com/pmikstacki/ROM/tree/prototype/resource-flow/prototypes/resource-flow). Project: `prototypes/resource-flow`. Twelve real HTTP/SSE integration probes, formatting and Clippy passed in the NixOS container and were independently repeated.

Use synthetic domain data and a clearly marked scratch SQLite database. The database choice is an experiment convenience, not a locked production adapter decision. A declaration macro or registry API may be provisional.

Acceptance observations:

1. Add a third unrelated resource kind through its declaration; generic persistence, CRUD endpoints and event subscription need no per-kind implementation.
2. Explicit false and zero survive mutation; missing fields remain unchanged; null follows the declared field rule.
3. A successful action commits state and event together; failure injection leaves neither partially committed.
4. Two actions using the same revision cannot silently overwrite one another.
5. Same-identity retry does not repeat a transition, and different input under that identity is rejected.
6. A subscriber can recover committed events after reconnect; ordinary notification loss is not permanent data loss.
7. A reaction changes another resource through the same action contract and can safely retry.
8. Restart restores persisted state and replayable history.

Record which observations actually execute and which remain untested. Local event-stream recovery does not automatically establish reactive query dependency tracking, distributed delivery, multi-process scalability, or complete provider authorization.

## Execution-contract experiment

Branch: [`prototype/execution-contract`](https://github.com/pmikstacki/ROM/tree/prototype/execution-contract/prototypes/execution-contract). Project: `prototypes/execution-contract`. Locked release executable, formatting and Clippy passed in the NixOS container and were independently repeated. The deliberate incorrect control exposed over-admission as intended.

Acceptance observations:

1. Shared executor instances serve many jobs without per-resource pool creation.
2. Admission is bounded before job submission.
3. Cancelling a reply waiter does not release capacity while its CPU closure still runs.
4. Dropping a reply receiver does not panic or leak permits.
5. An unwinding CPU panic produces an explicit outcome and releases capacity.
6. Shutdown waits for admitted work's actual completion.
7. A deliberately naive implementation violates the cancellation/capacity invariant, proving that the probe can detect the mistake.
8. Async timers progress while CPU work runs; report workload and observed timings without claiming a production benchmark.

## Promotion rule

A passing prototype proves only its recorded observations. Preserve source and lockfiles on the prototype branches. Main receives evidence, conclusions and requirements, not a claim that the prototype is the released ROM core. Before implementation, resolve the remaining interface and persistence decisions using these results and the owner's priorities.
