# Generic transport layer review

Date: 2026-10-02. Source review and proposed contract, not implementation. HTTP is an optional extension. Resource remains ROM's only domain entity; invocations, receipts and cursors are protocol or infrastructure values. The owner's Wolverine comparison is treated only as context for the question; no Wolverine architecture is adopted or needed for this proposal.

## Assessment of the current design

ROM already places HTTP and RabbitMQ outside the core dependency graph and requires a shared mutation path. The [architecture](../../openspec/changes/establish-rom/design.md) also describes reads, live reads and committed changes, but the [boundary specification](../../openspec/changes/establish-rom/specs/boundary-adapters/spec.md) tests equivalence only for mutations and describes descriptor-driven exposure specifically through HTTP. That leaves the generic transport contract implied rather than defined.

Promote the semantic interface to a core-owned boundary used by embedded callers and every transport. Adapters translate protocols into that interface; they do not independently assemble authorization, query execution, mutation dispatch or subscriptions. Preserve the existing [action](../../openspec/changes/establish-rom/specs/action-runtime/spec.md), [event](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md), [execution](../../openspec/changes/establish-rom/specs/execution-engine/spec.md) and [authentication](../../openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md) invariants.

This needs shared semantic operations and explicit binding profiles, not a universal wire format or message bus. HTTP methods/statuses, AMQP exchanges/delivery tags, network listeners and client-library types belong in extensions. Core descriptors name resource kinds, actions, query capabilities and allowed observation forms. Each binding derives its supported public surface from those descriptors and host exposure policy, without per-kind handlers.

## Evidence that constrains the design

| Source fact | ROM consequence proposed here |
| --- | --- |
| HTTP `202` means processing is incomplete and may never happen; its representation should describe status and a monitoring mechanism. [RFC 9110 §15.3.3](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.3.3). | A queued acceptance response cannot represent committed resource success. Any stronger ROM durable-acceptance promise needs its own persisted obligation and recovery behavior. |
| RabbitMQ publisher confirms and consumer acknowledgements are independent; delivery tags are channel-scoped. Manual acknowledgements allow redelivery after connection loss. [RabbitMQ acknowledgements](https://www.rabbitmq.com/docs/confirms). | Keep transport delivery identity separate from action/idempotency identity and durable commit receipts. Expect duplicate attempts. |
| Direct Reply-To replies have at-most-once delivery and can disappear. [RabbitMQ Direct Reply-To](https://www.rabbitmq.com/docs/direct-reply-to). | A missing reply must remain an unresolved observation; a durable action lookup or same-key retry can recover a committed outcome. |
| Broker authorization checks access to virtual hosts, exchanges and queues. [RabbitMQ access control](https://www.rabbitmq.com/docs/access-control). | Inference: queue permission alone does not establish the business principal represented by a payload. ROM still needs its trusted identity integration and core policy checks. |
| gRPC describes cancellation as lost client interest and requires application cooperation to stop a handler. Deadlines bound waiting, with timeout propagation accounting for elapsed time. [Cancellation](https://grpc.io/docs/guides/cancellation/), [deadlines](https://grpc.io/docs/guides/deadlines/). | Separate caller lifetime from supervised action execution and commit knowledge; never reset a command's lifetime on every hop. These are protocol lessons, not a recommendation to require gRPC. |
| A gRPC stream write can enter framework buffering before network transmission. [gRPC flow control](https://grpc.io/docs/guides/flow-control/). | Inference: transport flow control alone cannot prove bounds on ROM's queued projections, retained payloads or CPU work. Bound the complete delivery path. |
| EventSource sends `Last-Event-ID` when reconnecting. [WHATWG server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html#the-last-event-id-header). | Inference: this reconnect mechanism does not supply ROM journal retention, application processing acknowledgement or gap recovery. An adapter may encode a ROM cursor there only under a defined stream profile. |

## Compact conceptual contract

These are logical shapes, not proposed final Rust signatures or additional domain entities:

```text
invoke(TrustedContext, Operation, AttemptControl) -> UnaryOutcome
observe(TrustedContext, LiveQuery, StreamLimits) -> LiveSubscription
subscribe(TrustedContext, JournalRequest, StreamLimits) -> JournalSubscription

Operation = Discover | ExecuteAction | ReadResource | QueryResources | ResolveAction
```

| Element | Core-owned meaning |
| --- | --- |
| Operation | Registered kind/action identity and schema version, resource target where applicable, descriptor-consistent decoded input; read/query shape and projection for reads. Standard and custom actions enter the same execution contract. |
| TrustedContext | Host-established principal, authority, isolation scope and bounded validity/policy inputs. Constructed outside deserialization and passed separately; contains no credentials. A trusted Rust extension is not a sandbox. |
| Mutation identity | Stable idempotency key scoped to principal, isolation and operation, plus canonical semantic input and expected revision. Reusing a key with different semantic input fails. Transport correlation IDs do not replace this identity. |
| AttemptControl | Request/trace correlation, bounded admission, response-wait budget and cooperative cancellation. Retry-specific controls do not redefine a committed command. A business command expiry, if supported, is separately defined semantic input rather than a renewable wait timeout. |
| UnaryOutcome | Protected read result, committed action receipt/result, safe rejection, or explicitly unresolved action observation. An action receipt identifies the durable outcome and relevant revision; it does not imply that reactions or remote effects finished. |
| Subscription | Bounded delivery handle, distinct live/journal frames, explicit terminal reasons and the corresponding recovery rules. Cancellation closes observation without undoing resource history. |
| BindingProfile | Supported operation families, codec/schema versions, size/stream limits, wait/acceptance behavior, resume and checkpoint facilities. Registration validates required combinations; unsupported operations fail explicitly. |

Use focused methods/types behind this conceptual gateway where that improves Rust usability; avoid an untyped `send(anything)` public API. Core errors retain semantic categories such as invalid input, denied, conflict, unsupported, overloaded and unresolved outcome. Bindings map them to appropriate wire representations without exposing protected values. An unresolved outcome must not be flattened into a definite no-commit rejection or unconditional safe-to-retry hint.

Authentication remains adapter/host work and authorization remains core work. HTTP may verify an access token; a broker binding may use a configured service principal or independently verified delegation. Serialized actor fields, routing keys and event attribution never create authority. Delayed messages need still-valid authority at execution; durable replay/outcome lookup needs current authorization at disclosure. Discovery and subscription delivery receive the same protection as ordinary reads. These follow the existing [provider-neutral authentication requirements](../../openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md).

## Outcomes, acceptance and execution lifetime

Distinguish four observations: received by a transport; durably accepted for later work if that profile exists; resource action committed or definitely rejected; reply received by the caller. None implies the next. Core remains responsible for atomic resource/event/outcome persistence under the selected storage contract.

For a first RabbitMQ command binding, acknowledge a delivery only after a recoverable terminal action outcome or a durable handoff with a documented recovery owner. Reject/quarantine malformed or permanently denied requests under a bounded policy instead of retrying forever. Crash after commit but before acknowledgement must lead to same-key outcome resolution. Keep terminal response availability separate from broker acknowledgement: use authorized lookup, or an explicit durable response mechanism if guaranteed reply delivery is required. This is a proposed ROM binding policy, not a RabbitMQ guarantee.

Durable acceptance is optional and should not be introduced by returning a generic `Accepted` variant without an inbox/job persistence contract. If supported, define who owns recovery, how status is resolved, expiry and retention, and what terminal rejection means. An HTTP `202` or broker confirm alone cannot supply that contract.

Before action execution, expired wait budgets can reject admission. Once execution is supervised, a disconnected waiter can stop receiving without orphaning work or releasing permits for running CPU jobs. At the commit boundary, preserve uncertainty until durable resolution; after commit, cancellation cannot relabel the action as rolled back. A retry with the same identity resolves the outcome without repeating the transition, subject to receipt retention and current authorization. A missing or expired lookup record is not proof that an action never happened.

## Two different stream contracts

Live reads deliver the current authorized query result. They may coalesce intermediate states, but their initial snapshot and updates must have a consistent boundary or force a refresh. An overflow can produce an explicit resync requirement or terminate; never silently retain an obsolete projection as current.

Journal subscriptions deliver committed facts with stable identities, per-resource revision order and declared retention. Resume tokens identify this history and its filter/scope contract, not a socket or AMQP delivery. Retention loss produces an explicit gap; it must not silently skip to the present. Transport send completion, browser receipt, consumer processing checkpoint and ROM journal progress are separate notions. Durable consumer checkpoints require their own authenticated acknowledgement operation when that profile is supported.

Both paths need limits on item count, bytes, individual frame size, pending production and active subscriptions. Backpressure must propagate or invoke a defined overflow policy. Actor expiry and policy freshness constrain subsequent disclosure even if frames were previously prepared. Broker prefetch can bound unacknowledged deliveries, but must be combined with ROM's own byte/admission limits. [RabbitMQ prefetch](https://www.rabbitmq.com/docs/confirms#channel-qos-prefetch).

## Capability profiles rather than a common denominator

Every binding uses the same meaning for each operation it supports. It need not expose every operation or mimic another protocol's interaction pattern. For example, an HTTP profile can offer request/reply and SSE observation, while a RabbitMQ profile initially offers durable command delivery and correlated outcome retrieval. Neither requires a global `streaming=true` claim that conflates live snapshots with replayable journal history.

Validate required host capabilities at setup, publish only authorized discovery metadata, and reject unsupported requests at invocation. Validate combinations such as journal resume plus retention and live-query overflow plus resync; a collection of independent flags cannot establish them. Wire codec versions are adapter concerns constrained by core descriptor semantics. No adapter can weaken authorization, revision checks, canonical input comparison or committed-fact rules through capability negotiation.

## Two-transport acceptance before stabilizing the interface

1. **Equivalent semantic invocation through HTTP and RabbitMQ.** Register one resource and a custom action once; configure both bindings without per-kind handlers. Run shared vectors through embedded calls, actual HTTP encoding and actual RabbitMQ delivery: presence/null/value handling, invalid input, forbidden fields, stale revision, same-key replay, changed-input replay and forged actor metadata. Exercise read/query/discovery for each profile that declares them; unsupported families must fail explicitly. Compare authorized results, state revisions and durable event/outcome counts rather than wire statuses. Lose the HTTP response and crash the broker consumer after commit before acknowledgement: retry must expose at most one committed transition, with outcome disclosure denied after permission revocation. The core-only build must require neither server nor broker dependencies.

2. **Honest lifecycle and capability behavior across those bindings.** Saturate admission and slow consumers under measured item/byte limits; expire a wait budget before admission, during CPU work and around commit; restart after each boundary and resolve outcomes. Verify broker acceptance never becomes a fabricated commit receipt. For each declared live/journal profile, race startup with writes, disconnect/resume, exceed retention, revoke access and test overflow: live results refresh/converge, journal consumers recover facts or receive an explicit gap. A binding without a requested stream or checkpoint profile must reject it before starting work. Observe wire traffic and persisted state to distinguish reply loss from action failure. No claim of stream parity is earned merely by exposing a generic stream type.

The next design change should make this semantic boundary and its profiles explicit in OpenSpec, then test a narrow integrated slice through two real bindings. Final Rust signatures, exact profile/version vocabulary and durable acceptance machinery remain implementation decisions; this review does not select them.
