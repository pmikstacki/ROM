# Maintained Blob Attachments Implementation Plan

> Execution: inline implementation using the executing-plans and TDD workflow;
> parent explicitly delegated this package and approved the described design.

**Goal:** maintain generic authorized Blob Resources over folder and S3 stores.

**Architecture:** provider-neutral `rom-blob` owns manifest, port, Resource and
supervised service. `rom-blob-object-store` implements atomic create-only objects
through object_store; the same contract tests run on filesystem and real MinIO.

**Tech stack:** Rust 1.99, existing ROM/Tokio/Serde, SHA-256, futures-util,
object_store 0.14.2. MIT packages; retain dependency attribution.

**Spec:** [design](../specs/2026-10-02-maintained-blobs-design.md).

## Constraints and failure focus

- No Row, Bundle, query, configuration or notification ledger changes.
- Client Resource input cannot fabricate a Ready attachment.
- Failed, oversized, stalled or digest-mismatched input never publishes partial bytes.
- Lost provider/metadata acknowledgments remain distinct and recoverable.
- If a caller cancels, its work retains capacity until completion. Shutdown waits for actual work completion.
- Revocation during content retrieval prevents disclosure.

## Tasks

- [x] Define the port, manifest and ordinary Blob Resource; add failing lifecycle
  tests for forgery, caller authorization and conflicting content.
- [x] Implement bounded staging, supervised service, conditional finalization,
  read reauthorization and detached/orphan outcomes; pass those tests, including
  interruption/restart and caller cancellation.
- [x] Implement folder and S3 constructors with explicit endpoint policy and
  immutable atomic publication; run common success/failure and restart cases.
- [x] Run real disposable MinIO suite and acknowledgment-loss fixture where
  available; distinguish executed evidence from unsupported provider claims.
- [x] Run formatting, Clippy, workspace/auth checks, core dependency boundary and
  dependency audit; document limits, commands and results; commit package.
