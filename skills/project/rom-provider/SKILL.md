---
name: rom-provider
description: Use when adding a ROM database provider, delivery integration, search index, vector projection, or real-backend conformance tests.
---

# Select and verify a ROM provider

Use this project guidance for the `0.1.0` source candidate. It is not an executable bundle workflow.
Read [release support](../../../docs/release-support.md) before claiming provider compatibility.
Read [quality gates](../../../docs/quality.md) before implementation and [writing rules](../../../docs/writing.md) before documentation changes.

## Select the port and repository

| Provider role | Starting point |
| --- | --- |
| Atomic Resource state, events, receipts, and effects | [Native extension skill](../../rom/rom-native-extension/SKILL.md), [Storage port](../../../crates/rom/src/persistence.rs), and [ownership contract](../../../docs/storage-ownership.md) |
| Native query candidate selection | [Query adapter contract](../../../docs/query-adapters.md) |
| External immutable blob bytes | Native extension skill and the optional BlobStore profile |
| Delivery integrations and external databases | The explicitly supplied [ROM-extras](https://github.com/pmikstacki/ROM-extras) checkout and its provider-specific contracts |
| OpenSearch or Qdrant projection | ROM-extras projection checkpoint, authorized-history, and search contracts |

In the selected ROM-extras checkout, read `README.md`, `docs/support.md`, and the affected crate's public entry and tests.
For search or vectors, also read `docs/projection-checkpoints.md` and its linked current verification records.
Read that checkout's `AGENTS.md` and `docs/goal.md` for applicable workflow and research requirements.
Resolve different status claims against their exact source and evidence. A roadmap does not establish implemented support.
Select the provider's primary documentation for the actual version before choosing consistency, query, or acknowledgement behavior.

## Implement and qualify

1. Select the port, source revision, locked ROM dependency, and exact backend version.
2. Define atomicity, ownership, ordering, limits, and unknown-outcome behavior before implementation.
3. Reuse the selected port's conformance cases through public APIs.
4. Test the actual backend's commit, rollback, stale revision, restart, and lost acknowledgement behavior where applicable.
5. Verify an independent consumer with explicit features and a consumer of the packaged archive.
6. Run the repository's applicable full verifier before integration.
7. Record source, lockfiles, features, backend identity, commands, and unresolved failures separately.

## Projection cases

A search index supplies candidates. Current Resource state and authority control returned data.
Keep exact Resource keys and revisions. Validate mapping/profile identity and active generation.
Test changed and deleted Resources, tombstones, duplicates, and permission revocation during a delayed response.
Advance a durable checkpoint only after the selected acknowledgement contract is satisfied.
After response loss, resolve the original delivery identity before claiming progress or rollback.
Bound response size, candidate work, retries, and the total request deadline. Reject partial or malformed success responses.
Test generation replacement, rebuild, and crash recovery before advertising those capabilities.
For vectors, verify dimensions, distance metric, normalization, and revision fencing against the real backend.

## Completion limits

A Field/Storage baseline does not qualify a search or vector provider.
A numeric probe does not establish an integrated Rust adapter or current authorization.
Keep real backend, fixture, independent-consumer, and packaged-consumer results distinct.
Retain failures and unknown outcomes even when a later focused run passes.
Report unsupported and pending capabilities explicitly. Do not extend ROM's release matrix from ROM-extras experiments.
