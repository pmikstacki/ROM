# Restrict-reference integration preparation

Status: **proposed, not implemented**. Read-only review of main `100aff7066be9fa2abf8634c2ed655f974f8d848`. The owner accepted restrict as the default deletion behavior; the mechanism below is an implementation recommendation, not an additional accepted product policy.

## Existing seams and limits

- `crates/rom/src/resource.rs:122`: `ResourceRef<R>` is explicitly an identity reference without foreign-key enforcement. `Shape::Reference` participates in recursive shape validation at `resource.rs:662`.
- `crates/rom/src/execution.rs:154`: registration verifies target kinds exist in the current registry, not target instances. `Builder::build` constructs a separate Runtime commit gate; it does not enforce one Runtime owner per store.
- `crates/rom/src/execution.rs:748–861`: final receipt lookup, current authority/revision/field checks, transition validation, bundle construction and adapter commit share the Runtime gate. This is the core insertion area, but that gate is local to one Runtime.
- `crates/rom/src/persistence.rs:53`: `Bundle` contains row/receipt/effects/reaction work, with no reference constraints or edge updates. The single-owner requirement is documented at line 68, not enforced by this trait.
- `crates/rom-sqlite/src/lib.rs:254`: commit opens an immediate transaction, checks receipt and expected revision, then writes the complete bundle.
- `crates/rom-redb/src/lib.rs:327`: commit uses an adapter gate and immediate-durability write transaction. It has the equivalent insertion point before durable changes.
- Native formats are currently SQLite/redb **3**. `crates/rom-backup/src/lib.rs:126,352` writes/requires archive **1**, storage **3**. Adapter `maintenance.rs` exports/restores fixed row, receipt, event, effect and state structures; no edge data exists.

## Alternatives

| Approach | Correctness boundary | Cost |
|---|---|---|
| Bounded core scans under the Runtime gate | One enforced owner, complete accepted schema and authoritative scans; maintenance cannot mutate concurrently | Least initial code, repeated reverse scans, explicit failure when bounds prevent proof |
| In-memory edge map rebuilt at startup | Same ownership requirements; complete startup rebuild and uncertain-commit reconciliation | Faster lookup, but startup and `Unknown` results make cache consistency another correctness mechanism |
| Persisted edges checked inside adapter commit | Target existence, incoming restrict, edge replacement and Resource bundle share the native transaction | More format/migration work, direct atomicity boundary and reusable index lifecycle |

**Provisional recommendation:** persisted outgoing/reverse edges and a generic integrity obligation in the commit contract. Core derives edges from descriptors; adapters perform transactional checks and persist them without domain-specific controllers. Do not treat an in-memory index as authoritative integrity state.

A core scan is a valid bounded first slice only with enforced ownership and complete-schema checks. It is not globally atomic with today's gate. Example: Runtime A reads target T as live; Runtime B scans no incoming references and deletes T; A writes source S referencing T. Both per-row CAS checks succeed because they affect different rows. SQLite's serialized transactions do not fix checks that happened outside those transactions.

## Proposed contract

1. Compile recursive reference extraction from accepted descriptors, including nullable/optional/list/map fields; deduplicate target identities. Validate canonical values through existing codecs.
2. Compute candidate outgoing edges for every mutation family. Deletion has no outgoing edges.
3. In the adapter's existing atomic commit: resolve matching receipt first; check source revision; verify candidate targets are live; enforce incoming restrict on deletion; replace source edges; write row, event, receipt and pending work.
4. Remove old edges using persisted authoritative state rather than trusting supplied old edges. Handle self-reference against the candidate row; deleting a self-referencing row removes its own edge, while another source still blocks deletion.
5. Integrity checks inspect all relevant live rows, independent of caller visibility. Public errors must not reveal hidden referencing Resources.
6. Historical receipts/events and protected tombstone policy snapshots are not live edges. Matching receipt replay remains historical replay under current disclosure checks, not a second mutation or revalidation of an obsolete reference.

The contract must specify how adapters reject unsupported mandatory integrity semantics. An index is an optimization; silently omitting integrity is not an acceptable fallback. Native direct `Storage::commit` remains a trusted adapter boundary, so conformance must verify all declared integrity obligations there.

## Ordering and migration dependencies

1. Settle and document the ownership boundary first. Single-owner enforcement was in release stage 4, but a core-scan implementation requires it earlier; the existing gate alone cannot satisfy it. Transactional edge checks avoid the cross-row check/commit race but do not by themselves make multiwriter a supported full-runtime profile.
2. Define persisted schema/reference-layout identity and startup compatibility checks. Omitting a source kind from a later registry must not erase knowledge of its persisted incoming references.
3. Design explicit format migration/index rebuild alongside integrity implementation. Read live rows with accepted descriptors, reject unknown kinds or dangling references, then publish the new format/index generation atomically. Do not silently rewrite invalid references.
4. Extend backup collection, validation and restore. Either carry validated edges/schema metadata or rebuild them before publishing the destination. Existing checksummed archive readers require an explicit version transition; merely adding tables would lose them during restore.
5. Preserve receipts, reaction attempts/outcomes and journal behavior during migration; retain restore cursor/claim fencing. Do not rebuild edges from historical receipts or tombstones.
6. Integrate the reference application with typed references and demonstrate unlink/restrict/restart/upgrade/restore through public operations.

Heavy online rebuild, hot schema replacement and multiwriter execution are not required by this recommendation. Short offline migration is a proposed initial operational profile, not an owner-approved downtime policy.

## Acceptance matrix

Run on SQLite and redb, including typed and generic invocation entry points:

- Direct, optional, nullable, list and map references; missing/tombstoned target rejection.
- Create, replace, patch and action cannot introduce dangling references; removing an edge releases restrict.
- Target deletion fails for any live incoming source, including one hidden from the caller; source deletion removes its outgoing edges.
- Self-reference behavior and two-Resource cycles, requiring explicit unlink before deleting cross-referenced rows.
- Concurrent target delete versus source create/update: no schedule may leave a dangling edge. Test duplicate Runtime ownership according to the supported profile.
- Replayed receipts, CAS conflict, injected rollback and lost acknowledgment preserve exact edges and event/work counts.
- Restart, explicit index rebuild, format migration and backup/restore preserve enforcement and pending-work identity.
- Missing index generation, schema mismatch, unregistered persisted source kind, corrupt edges and exceeded scan/rebuild bounds fail closed.
- Edge writes and rebuild cost are measured separately from query speed; no performance claim follows from this read-only review.
