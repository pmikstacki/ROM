# ROM 0.1.0 acceptance design

## Evidence boundaries

Investigation records identify producer and consumer revisions before implementation.
The maintained intake separates shared candidates from consumer-owned behavior and withdrawn requests.
Each selected candidate needs a regression and external acceptance; rejected or deferred candidates need a reason.
No source candidate is a confirmed producer defect until reproduction establishes its origin.

## Module boundaries

Captured validation remains in Resource definitions and the existing command pipeline.
Public Studio controls use a controls-only entry; the application entry retains its existing contract.
Mutation recovery composes the public client, exact Invocation, original principal, and host-selected pending-intent storage.
Do not persist credentials or silently treat cancellation as proof of rollback.

AI policy and typed flow composition belong in optional modules. OpenRouter belongs in its own adapter.
Reuse durable work, reactions, channels, receipts, and authorized progress Resources instead of creating a second scheduler.
Model eligibility, provider selection, aggregate spend reservations, actual accounting, and deadlines remain distinct.
Provider uncertainty remains explicit when accounting or reconciliation cannot resolve it.

## Acceptance order

First complete independent validator and public-control packages and their regressions.
Then compose reload recovery, durable flows, projections, and optional AI execution in external consumers.
Use Astral Plane and a non-AI maintenance portal with public interfaces and isolated data.
Complete identity, restoration, upgrade, telemetry, and load/fault acceptance before artifact production.
Freeze source before the full verifier, independent review, and complete release producer.
Deploy only matching verified artifacts. Record support limits before publication.

## Production limits

Begin with a single-instance profile and one database owner. SQLite and redb retain shared conformance.
Record actual tested workload, resource ceilings, RPO, RTO, identity profile, and unsupported conditions.
Do not imply multiwriter, universal identity-provider, strict external exactly-once, or power-loss guarantees without evidence.
Production acceptance uses complete application recovery, not only database archive integrity.
