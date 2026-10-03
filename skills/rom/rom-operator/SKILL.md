---
name: rom-operator
description: Use when a ROM work control times out, loses its response, reports an unresolved outcome, or needs authorized inspection, retry, or reconciliation.
---

# Diagnose durable work

Read the [native contract](../../../docs/native-extensions.md) for protocol and replay version ownership.
Use the [bundle procedure](../README.md) to run `operator` preflight and its [fault example](../assets/operator/run.mjs).

1. Retain the exact submitted request file, principal, WorkVersion, retry epoch, and idempotency key.
2. Record the file digest and the unresolved outcome.
3. Read the current host's capabilities with its protected credential file.
4. Inspect authorized work with `work list` and `work show HANDLE`.
5. Recheck current authority and the host's applicable replay floor.
6. Resolve the old identity through an explicit retry of the unchanged request as the same principal.
7. Keep provider reconciliation references opaque and free of credentials.
8. Execute compensation only through a deliberate domain action after its cause is established.

A lost response does not establish rollback.
The included example submits once and expects the CLI's unresolved-outcome result.
Its loopback fault fixture cannot establish whether a native transaction committed.

Completion requires preserved request identity, inspected current authority, and evidence that distinguishes unknown from confirmed outcomes.
Report future recovery commands separately from commands actually executed.
