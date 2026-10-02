# Reusable core: implementation readiness

Date: 2026-10-02. This is a work breakdown and uncertainty assessment, not a calendar estimate or a claim that production ROM exists.

## Direction supported by evidence

Combine the resource descriptor/shared mutation pattern with work-owned execution supervision. Use a derive for structural Rust bindings, fluent composition for behavior, and one runtime for dynamic guarantees. The [authoring trials](authoring-crate-trials.md) support this split; the [earlier execution and resource-flow experiments](prototype-results.md) support its execution and persistence seams. The subsequent transport-free library slice passed eleven tests for mutations, filtered reads and policy changes, including a reproduced stale-authorization defect and its fix. These are separate experiments, not yet one integrated typed library.

Application authors declare resources and write business functions. ROM owns the difficult registration, codecs, transactions, task supervision, policy checks and subscriptions. A production public interface should not expose the experiments' string field selectors, JSON mutation code or executor bookkeeping as the normal typed authoring path.

Subsequent [adapter experiments](capability-prototype-results.md) now add real SQLite/redb substitution, folder/MinIO blob interoperability and durable function-channel delivery. Production integration must still commit effect intentions with the full resource bundle, enforce public policy and combine these seams with typed authoring and supervision. No engine becomes part of the core contract.

## First reusable milestone

| Work package | Concrete acceptance | Uncertainty |
| --- | --- | --- |
| Resource/field contract and registration | One descriptor authority; derived and manual definitions agree; custom field works through public interfaces; unsupported capabilities fail before use. | Moderate: codec/presence vocabulary and field families still need specification. |
| Authoring facade and derive | One resource declaration plus ordinary custom action; typed selectors/inputs; source-local errors; packaged external consumer; no repeated field list. | Moderate: derive and builders passed separately, but their complete integration and IDE experience are untested. |
| Shared mutation and reference storage | Authorization and revision use authoritative state; atomic state/event/outcome transaction; no-op, rollback, concurrent conflict and idempotent retry tests. | Higher: reference database and adapter contract are not selected; prototype serialization is intentionally narrow. |
| Execution lifecycle | Shared Tokio/Rayon services, admission before submission, work-owned permits, bounded retained results and explicit drain behavior. | Moderate: cancellation accounting is demonstrated, continuous supervision and production failure handling remain. |
| Reactive reads and policy freshness | Generic query membership changes, race-free initialization, current authorization at delivery and explicit slow-consumer recovery. | Higher: supported query subset, row/byte limits, dependency invalidation and multi-process behavior determine scope. |
| Durable reactions and transport adapters | Reactions use the same action path; replay/deduplication; one generic HTTP adapter; default-deny core with trusted identity seam. | Higher: retry/retention/cycle budgets, real identity providers and field projection remain separate contracts. |

Implement the first five packages as a narrow integrated library slice before expanding field/query coverage. Add generic HTTP against that interface, then durable reaction hardening. Keep the demo application deferred until the core milestone meets its acceptance criteria.

## Decisions that change the estimate

The owner has selected the resource premise, Rust/native extension direction, Tokio/Rayon execution, derive/fluent authoring preference, backend scope and local NixOS verification. The following choices still materially affect implementation size:

- Reference database and deployment topology: embedded single-host versus server database with independent writers.
- Initial field/query subset, canonical encodings and compatibility behavior. Supporting every proposed type and arbitrary live queries is a larger milestone than a bounded initial subset.
- Policy contract: row and field projection, source of identity, and revocation freshness across processes.
- Durable history/receipt retention, reaction retries, failed-work inspection and cycle budgets.
- Minimum supported Rust version and public compatibility commitments; the upgraded development compiler alone does not settle these.

Recommendations can be prepared for each choice, but none is silently recorded as an owner-approved final selection. Current-state storage plus a durable journal remains the preferred proposal; SQLite is an experimental vehicle, not the selected production backend.

## What would make a calendar estimate defensible

Fix the first milestone's subset and acceptance tests, assemble one typed downstream application against the integrated library, and measure completion of that slice. Then estimate the remaining work using actual implementation/review throughput and an explicit uncertainty allowance. Separate core correctness from adapter breadth, documentation, packaging and operational hardening. A count of passing throwaway tests does not provide a reliable person-day estimate.

Human usability needs a real author walkthrough: define a resource, discover standard operations, add an action, observe a filtered result, restrict access and repair an intentional error using the normal documentation. Compiler-test success and agent-written examples are necessary evidence, but they do not replace that check.
