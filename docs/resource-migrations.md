# Migrate Resource fields

ROM can convert a Resource representation into the next declared version. The
operation preserves Resource keys, revisions, receipt identities and fingerprints.
It writes a fresh database and leaves the source unchanged.

## Declare versions and a conversion

Keep the old type available for migration and receipt replay. Do not change its
codec while keeping its version number.

```rust
#[derive(Clone, rom::Resource)]
#[resource(name = "tasks")]
struct TaskV1 { title: String }

#[derive(Clone, rom::Resource)]
#[resource(name = "tasks", version = 2)]
struct TaskV2 { label: String, done: bool }

let plan = rom_backup::MigrationPlan::new(vec![
    rom_backup::ResourceMigration::new::<TaskV1, TaskV2>(|old| {
        Ok(TaskV2 { label: old.title, done: false })
    })?,
])?;
```

The default Resource version is 1. Versions are positive `u32` values. A step
must retain the kind and advance its version by one. The persisted source descriptor
must match exactly. A plan can contain ordered steps across several kinds.
Repeated, missing or reversed steps fail. The converter must be pure and bounded.
ROM catches a converter panic, but it cannot stop an infinite native loop.

## Stop, migrate and start

1. Stop source writers and workers.
2. Supply a complete ordered plan and explicit `BackupLimits`.
3. Call `Sqlite::migrate_from(source, destination, &plan, limits)` or the redb equivalent.
4. Start the destination with its new Resource definitions and retained replay codecs.
5. Run application acceptance checks. Preserve the original source until they pass.

The source can use native formats 4 through 8. The destination uses format 8.
For an older format-3 source, first use the [native format upgrade](native-upgrade.md).
A destination must be new. Existing files and symlinks are not replaced.
Native rebuild and validation finish before publication. A post-publication error
can return `Unknown`; inspect the destination before retrying.

Migration uses a bounded logical snapshot and memoizes repeated values. It checks
input and output budgets, journal capacity and reserved work capacity. It never
removes obligations to make an expanded representation fit. The byte budget is
serialized data, not a precise heap limit. Budget memory for decoded data and copies.

## Preserve retry and authorization

Register the old codec on the new definition:

```rust
let definition = TaskV2::definition().replay_from::<TaskV1>();
```

Add the new definition's normal policy and field permissions before registration.
Retain every old codec version still required by stored receipts.

Each receipt records the version that interpreted its original request. ROM checks
current authority and disclosure rules before using that exact codec. It does not
try several codecs until one matches. New mutations use only the current codec.
A retry returns the migrated representation at the original revision. It creates
no new event or work. A missing legacy codec fails explicitly.

Receipt identities and fingerprints remain exact. Field representations in current
rows, historical receipts and events change consistently. This is a representation
migration, not an action that changes business state. The source database preserves
the previous physical representation. Historical values remain subject to current
authorization, including the existing rule that hides live history after deletion.

## Validate unfinished work

Resource conversion includes reaction and notification source snapshots. It also
includes protected deletion authorization values. It preserves source provenance.
It does not change frozen action invocations, notification payloads or effect intentions.

If unfinished work exists, attach `plan.validate_work(check)` with a pure
`fn(&rom::PendingWork) -> rom::Result<()>` callback. Check definition/version,
service identity, target routing and input compatibility with the destination's
consumers. Reject unknown contracts. Stopped work also requires validation because
an operator may later reconcile it. Done work needs no active consumer.

The callback attests application compatibility; a descriptor cannot prove that an
arbitrary action or external receiver understands its payload. Keep compatible
handlers while their obligations exist. Changing action names, service identities
or possibly delivered payloads is not part of Resource schema migration.

Restore changes journal generation and fences active claims. Attempts, causal
budgets and uncertain delivery outcomes persist. An unknown notification outcome
keeps its delivery identity and exact payload. Migration does not grant another
attempt budget or guarantee exactly-once external delivery.

## Compatibility

Native format 8 and archive version 6 also preserve work revisions, delivery profiles and operator receipts. Older tools
reject these formats. Use explicit archive/native upgrade methods for older formats.
This profile supports offline field transformations. Resource kind renames, identity
changes, online cutover and concurrent writers require different contracts.
