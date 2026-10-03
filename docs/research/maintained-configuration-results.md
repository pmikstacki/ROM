# Maintained configuration ingestion

Date: 2026-10-02. This is maintained library integration, following the disposable
[loader trials](configuration-trial-results.md) and
[Resource configuration contract](configuration-resource-contract.md).

## Question and implemented answer

Can bounded JSON/TOML input update the actual Resource runtime and preserve source authority and ownership? Can it also preserve current validation and durable accepted provenance?

Yes for **one complete externally owned Resource per reload**. `rom-config` uses
config-rs only as an input parser. Core has a native trusted SourcePermit, compiled
whole-Resource ownership, a generic revision precondition and protected provenance.
It has no dependency on config-rs. SourceActivation is an ordinary derived Resource,
with shared actions, policies, revisions and storage. It is not another entity engine.

The source grant binds a service authority/subject and target kind/id. An explicit
host policy owns activation, scope and worker fields. A worker can request a new
generation through the registered action but cannot grant itself another target
or enable itself. Source content contains only candidate fields; it cannot supply
registry definitions, identities, priorities or permissions.

Before a fetch, the request records its original target revision and create/replace mode. The target's value and accepted provenance are committed in one native
bundle. Requested and accepted generation therefore remain distinct without an
unsafe second active-state write. `resume` reconstructs the same request after a
restart; input substitution under that identity is rejected. Metadata-only changes
advance the Resource revision and journal; unchanged value/provenance remains a no-op.

## Reproduction and observations

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/configuration-trials && CARGO_NET_OFFLINE=true ./crates/rom-config/verify'
```

The package verifier runs **3 parser tests, 7 ingestion tests, 3 core source tests,
and 1 compiled public documentation example**. Formatting, Clippy with warnings
denied, documentation with warnings denied, and core loader-dependency isolation
are part of the same command. Rust/Cargo 1.99 and two build jobs are required.
The combined repository `scripts/check` also passed after integrating Presence/
PATCH, durable channels, deletion authorization and HTTP keepalive fixes: strict
OpenSpec validation, workspace tests/clippy/docs, compile-fail and external
consumer checks, plus the auth and identity package verifiers.

| Evidence | Result |
| --- | --- |
| JSON/TOML bool, zero, empty string, arrays, dotted literal field names and JSON null | Types and literal names preserved before native Resource decoding |
| Duplicate JSON keys, including nested duplicates | Rejected before last-value-wins parsing can conceal an invalid contribution |
| Malformed JSON/TOML, non-finite TOML, oversized source, unsafe origin label | Rejected with fixed categories, without raw document/path disclosure |
| Deliberately unsafe custom Resource codec diagnostic | Raw supplied value redacted at ingestion boundary |
| Settings, User and IdentityProvider | Existing native definitions, field codecs and shared actions used; unknown privilege fields and unsupported provider profiles rejected |
| Readable source configuration under another service identity | Regression reproduced before explicit worker binding; request/resume now deny it |
| Invalid newer request and stale older completion | Last accepted target/provenance remains unchanged; old completion conflicts, including when the newer attempt fails |
| Source disable/re-enable, wrong owner/target, expired permit | Old or incorrectly scoped authority denied |
| Metadata-only reload, exact replay, equal value/provenance | Revision advances only for a real value/metadata change; receipt replay does not repeat it |
| Explicit delete and recreate | Lifecycle intent remains separate from omission or fetch failure; original create/replace mode survives recovery |
| SQLite and redb failure after a native write | Old value and provenance both remain intact |
| SQLite and redb lost acknowledgment after actual commit, then reopen | Original receipt resolves at the same revision; exactly two target journal facts for two successful generations |

The persistence tests run against native storage format 3 after integration with
the maintained channel work. Neither adapter needs loader-specific storage logic:
provenance lives in the protected part of the already atomic Row/receipt bundle.
Public typed/projected results and raw invocation remove protected metadata;
source-state inspection requires its own permission. Deletion authorization
metadata remains intact and uses the independent journal security fix.

## Source and dependency evidence

The locked adapter selects **config 0.15.27**, with default features disabled and
only JSON/TOML enabled. The exact downloaded crate source was inspected for
[`Source::collect`](https://docs.rs/config/0.15.27/config/trait.Source.html),
[`File::from_str`](https://docs.rs/config/0.15.27/config/struct.File.html), and
[`ValueKind`](https://docs.rs/config/0.15.27/config/enum.ValueKind.html). Collection
is direct rather than through Config's path merge layer; identity stays outside
parser key handling. Safe host labels are captured before conversion instead of
claiming that string-backed sources carry trustworthy filesystem provenance.
The earlier comparative report records official-version and license evidence.

The full lockfile audit passed with **230 dependencies**, using cargo-audit
`--no-fetch --deny warnings` and the supplied 1280-advisory database at commit
`117edb3bed98e9be112f277b7615eea3252e7c43` (2026-10-02T10:58:33+02:00).
This is a dated dependency check, not a universal security certification.

## Critique and bounded recommendation

Keep this narrow config-rs adapter. Descriptor validation, explicit ownership,
current policy and durable publication do most of the work; loader merge/profile
vocabulary should stay outside the Resource contract. Direct collection also
avoids the literal-key path interpretation found in the disposable trial. Duplicate
JSON checking performs an additional bounded parse; no throughput claim is made.

The trusted SourcePermit constructor is a native host boundary, like Actor::trusted;
it is never a deserializable transport parameter. Register row/field policies as
well as the source owner. Source version labels must identify immutable input for recovery. Secret values or low-entropy secret hashes are not valid version labels.
Explicit worker binding supplements host policy rather than replacing it.

Whole-Resource ownership is deliberate. Atomic multi-Resource publication,
field-level source ownership, precedence/overlays, removing an override, environment
discovery, external writeback and remote adapter activation remain unsupported.
The root PATCH implementation is not silently treated as configuration merging.
Source expiry stops writes; effective-value freshness and live external-handle
activation need separate host decisions. Secret resolution/rotation and multi-host
coordination are still host responsibilities. No Studio/VPN, real external account,
production login, secret-store deployment or human usability claim is made.
