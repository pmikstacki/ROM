# Durable named channels implementation plan

> Execute inline using superpowers:executing-plans, with one coordinator-owned independent review after the package.

**Goal:** Application-provided typed async Rust channels consume notification intentions from ROM's existing durable work lifecycle.

**Architecture:** Explicitly versioned channel intents are validated and added to the same Resource Bundle and WorkLedger already used by reactions. One bounded supervisor dispatches both work kinds; notification outcomes and uncertain delivery are persisted under the same fenced lease semantics. No second outbox schema or provider SDK.

**Tech stack:** Rust 1.99, current locked Tokio/serde/Rayon, existing SQLite/redb adapters.

**Spec:** `openspec/changes/integrate-resource-mvp`, `docs/research/mvp-decision-register.md`, notification section of `capability-prototype-results.md`, prototype branch `prototype/notification-channels` (12 tests).

## Constraints and ownership

Baseline main `2518493`; isolated branch `codex/durable-channels` in mvp-library. Host owns Tokio/Rayon. Resource remains the sole domain entity. No real notifications, provider SDK, new database or multi-resource transaction. Root owns query/presence changes; configuration/reviewer own protected Row metadata. This package changes Builder and commit routing but avoids the mutation proposal match and Row structure.

## Planned API and decisions

`const NOTICE: Channel<Message> = Channel::new("notice", 1)` creates typed versioned `Intent`s inside normal actions. `Builder::channel(NOTICE, service_actor, async_function)` performs erased registration and payload decoding; functions return `DeliveryOutcome::{Accepted, Retryable, Permanent, Unknown}`. Worker-produced Timeout/Panicked observations are also inspectable. Delivery carries a stable id, attempt and typed payload. No receipt string or external exactly-once claim is introduced.

Legacy `Intent::new` remains an opaque stored effect. New channel intents carry an explicit version marker; an unknown/mismatched registered channel fails the upstream commit. Channel work identities have a tagged namespace distinct from reaction IDs. The source snapshot is authorized for complete historical/current fields under the configured service before dispatch. Only the typed payload reaches the function; credentials stay in the host's function closure.

All work consumes existing root/attempt/age/depth/backlog budgets. Delivery timeout is a nonzero host setting below the lease duration, default five seconds. Shared bounded blocking work owns the actual async attempt through the host Tokio handle; timeout aborts and joins its cooperative Tokio task. This conservatively retains an I/O permit while waiting but avoids untracked tasks or caller-owned cancellation. A future that blocks without yielding remains trusted native code and cannot be forcibly stopped.

## Task 1: typed registration and atomic intentions

- [x] Add failing downstream test for a normal Resource action emitting a typed channel intent, actual adapter reopen, pending notification work and no precommit send.
- [x] Implement Channel/Delivery definitions, Builder registry and validation, explicit Intent version marker, Notification work payload and shared commit routing.
- [x] Preserve ordinary effects and reject invalid/unknown/mismatched payloads atomically.
- [x] Verify both native adapters and commit if independently reviewable.

## Task 2: shared lifecycle dispatch

- [x] Add failing tests for accepted/retryable/permanent/unknown attempts, durable observations, finite attempt/backoff/age limits and service revocation.
- [x] Add WorkLedger delivery-start/finish updates with claim fencing; persist Unknown before invoking external code, then bounded outcome afterward.
- [x] Dispatch async functions from the existing supervised worker. Add process_work/start_work aliases while preserving reaction APIs.
- [x] Test cooperative timeout, function panic, caller cancellation, shutdown drain and stale acknowledgment protection.

## Task 3: real recovery and report

- [x] Exercise effect-before-lost-ack then actual database reopen with and without receiver dedup. Both retries keep the same id; receiver effects differ as expected.
- [x] Cover explicit payload versions/types, definition changes, backlog pressure and atomic rollback on both adapters.
- [x] Run full verifier including identity/auth/HTTP, update report with public contract, exact defaults and remaining limits, and commit for independent review.

## Review focus

External completion may precede acknowledgment persistence; the worker must not call that rollback. Leases fence local status but cannot fence an external receiver. Unknown after the final attempt remains inspectable. Typed channel and reaction names must not collide. Cancellation must not orphan an active callback. Version/payload mismatch must reject before an upstream commit. Current service/source authority must be rechecked before external disclosure; no source field filtering is silently lost. Completed work capacity and serialized metadata amplification remain explicit limits inherited from the reaction profile.

Implementation notes: delivery IDs are 64 lowercase hexadecimal SHA-256 characters from a domain-tagged committed identity, using already-locked sha2 0.10.9. This bounds receiver keys and avoids forwarding canonical actor/resource tuples. Both storage format markers advance to3 because prior binaries cannot decode Notification work. Protected Row metadata remains coordinator-owned. Unknown/native seed error was observed in one early run; identical reruns and 20 focused persistence repetitions passed without code changes. It is not classified as rollback and no unsupported cause is claimed. Final report records this observation.

Ruling: include adapter-lifetime drain fix because actual notification restart tests exposed it. Work guards must not own Runtime; drop actual job Runtime before signaling active=0. An amplified redb DatabaseAlreadyOpen reproduction failed before and passed after the change. Permanent 32-cycle reopen regression added; temporary instrumentation removed. Runtime status follows as a separate coordinator-requested commit.
