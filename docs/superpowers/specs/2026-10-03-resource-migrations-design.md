# Offline Resource schema migration

## Intent

Resource authors need to rename fields, add fields and change field representations
without custom repositories. Migration must preserve receipts and outstanding work.
The release goal authorizes implementation and independent review of this lifecycle.

## Selected design

An offline migration reads one bounded snapshot, converts it in memory and publishes
a fresh validated native database. The source remains available. Each typed
`ResourceMigration::new::<Before, After>(convert)` binds exact source and target
descriptors. Their kinds must match and the target version must be the next version.
A `MigrationPlan` can contain ordered steps. Each step checks its exact input catalog.
Repeated or missing steps fail; no step is silently skipped.

Conversion changes representations, not Resource identities or business revisions.
It covers current rows, receipt rows, retained events, journal copies, reaction source
snapshots, notification source snapshots and protected deletion authorization values.
Repeated copies use the same converted value. Descriptors and current reference edges
are rebuilt. Historical references do not become live edges.

Receipt identities and fingerprints remain unchanged. Receipts retain the codec
version used for request comparison. A receipt without this marker receives its source
catalog version during migration. New commits record their current descriptor version.
`Definition::replay_from::<Before>()` supplies an older codec only for an existing
receipt with that exact version. Current authorization precedes codec execution and
disclosure. New mutations cannot use an old codec. This avoids interpreting the same
wire input under several versions until one happens to match.

Frozen action invocations, notification payloads, effect intentions, work identities,
service identities and definition versions are not rewritten. The host must explicitly
validate compatibility of unfinished work with the new application. Without that
validator, a plan with unfinished work fails. In particular, an uncertain external
notification retains its payload and delivery identity. Changing action/channel
contracts requires retaining compatible consumers; this operation does not guess them.

Restore fences old claims and journal cursors. Attempts, causal budgets and outcomes
retain their meanings. Conversion errors, panics, invalid output, dangling references,
incompatible work or exceeded limits prevent publication. The host supplies pure,
bounded native conversion and compatibility functions. These are not sandboxed plugins.

## Alternatives

Keeping old payloads and applying virtual upcasters on each runtime read would add
version routing to storage, policies, queries and workers. Converting only current
rows would break history, retry and pending callbacks. The selected snapshot conversion
keeps the live runtime representation uniform while preserving the original source.

## Scope and proof

Implement typed version declarations, shared conversion, exact replay codecs and native
entry points for SQLite and redb. Test field rename/default/type conversion, historical
disclosure, source preservation, retry, references, work recovery and process exit before
publication. Test mismatch, panic, limits and missing work compatibility. Backup/restore
must retain the result. This does not implement Resource kind renames, identity changes,
online cutover, arbitrary callback-code upgrades or retention policies.

## Format compatibility

Native format 5 and archive version 3 guard the new receipt codec metadata.
Earlier readers must reject these markers. Explicit upgrades cover native formats
3 and 4 and archive versions 1 and 2. A format-4 source requires an exact persisted
catalog; an empty catalog with live rows is corruption, not a format-3 fallback.
