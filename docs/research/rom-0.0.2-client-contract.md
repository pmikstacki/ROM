# ROM 0.0.2 browser client contract

Date: 2026-10-03. Status: implemented metadata and tested native/HTTP fixtures; Studio implementation remains separate.

## Public metadata

Discovery retains version 1. Existing action names remain unchanged.
Each Resource adds an `action_inputs` collection. Each disclosed action descriptor has `name`, version 1, and `input`.
An opaque input uses explicit JSON null. A hidden referenced kind suppresses the complete input descriptor entry.
The permitted action name can remain visible. Absence of its input description does not authorize inference or invocation.

Input descriptions use these forms:

- Unit: `{"type":"unit"}`. The invocation input is JSON null.
- Scalar: `{"type":"scalar","value":{"shape":Shape,"codec":CodecIdentity?}}`.
- Object: `{"type":"object","value":[{"name":string,"shape":Shape,"codec":CodecIdentity?}]}`.

`CodecIdentity` contains a nonempty `name` of at most 256 bytes and positive `version`.
It identifies a trusted codec binding. It is not executable code or an independent global Field registry.
Unknown renderer identities require explicit client registration or a precise unsupported-presentation diagnostic.

`Field::codec_identity()` defaults to none. Derives generate `Resource::field_codecs()` from typed field bindings.
The default Resource method returns an empty list. Manual Resources can supply named `FieldCodec` entries.
Registration validates binding names, duplicates, shapes, registered references, and codec identities. Accepted definitions freeze their metadata.
Persisted `FieldDescriptor` and native layout identity remain unchanged. Codec presentation changes do not migrate stored values.

`Input::descriptor()` defaults to opaque. Derived named inputs describe their actual field bindings and aliases.
The Field blanket input implementation delegates to `Field::input_descriptor()`.
Override that method when standalone input encoding differs from normal field encoding.
Standalone `Presence<T>` retains its tagged conditional envelope and remains opaque.
A derived named `Presence<T>` member uses its existing optional-member syntax.

Registration validates structural metadata. It cannot prove every behavior of arbitrary trusted manual codecs.
Derived bindings and explicit conformance fixtures compare descriptions with actual encoded/decoded values.
Discovery runs no action callback and decodes no stored Resource values.
Metadata grants remain separate from read, query, field, and mutation permissions.

## Executed HTTP fixtures

The examples below came from `studio_wire_fixtures_use_actual_http_values_and_live_frames`.
The test uses real loopback HTTP and SQLite. Native metadata acceptance separately exercises SQLite and redb.
The [captured output](evidence/rom-0.0.2/task1/http-wire-fixtures.log) preserves exact emitted JSON and HTTP chunk framing.
JSON member order is not a semantic identity check.

### Discovery

```json
{"resources":[{"action_inputs":[{"input":{"type":"object","value":[{"name":"count-value","shape":{"type":"u64"}},{"name":"note","shape":{"type":"nullable","value":{"type":"string"}}}]},"name":"details","version":1},{"input":{"type":"scalar","value":{"shape":{"type":"bool"}}},"name":"toggle","version":1}],"actions":["details","toggle"],"fields":[{"name":"display-title","shape":{"type":"string"}},{"name":"done","shape":{"type":"bool"}},{"name":"note","shape":{"type":"nullable","value":{"type":"string"}}},{"name":"owner","shape":{"type":"string"}}],"kind":"tasks","version":1}],"version":1}
```

### Create invocation

```json
{"kind":"tasks","id":"one","expected":null,"idempotency":"create","operation":{"type":"create","input":{"display-title":"visible","done":false,"note":null,"owner":"alice"}}}
```

### Authorized projected view

```json
{"key":{"id":"one","kind":"tasks"},"revision":1,"value":{"display-title":"visible","done":false,"note":null,"owner":"alice"}}
```

### Structured query request

```json
{"kind":"tasks","query":{"filters":[{"field":"done","value":false}],"limit":1}}
```

### Query and live snapshot value

```json
[{"key":{"id":"one","kind":"tasks"},"revision":1,"value":{"display-title":"visible","done":false,"note":null,"owner":"alice"}}]
```

### Denied error response, HTTP 403

```json
{"error":"denied"}
```

### Scalar action invocation

```json
{"kind":"tasks","id":"one","expected":1,"idempotency":"toggle-one","operation":{"type":"action","input":{"name":"toggle","input":true}}}
```

### Named action invocation with exact u64 boundary

```json
{"kind":"tasks","id":"one","expected":1,"idempotency":"details-one","operation":{"type":"action","input":{"name":"details","input":{"count-value":18446744073709551615,"note":null}}}}
```

## Wire lifecycle and mutation intent

Fetch handles HTTP chunk framing. Parse bounded SSE events from the decoded response body.
A live snapshot is `event: data` with a JSON array of projected views.
A terminal failure is `event: error` with a safe category. Keepalive comments are not snapshots.
Current live frames contain no descriptor version or server query-generation field.
Bind them to their requested kind and the client's own session/query generation. Never fabricate a server generation.
Journal cursors contain `generation`, `kind`, and `position`; query anchors have a separate contract.

Projected views can omit unauthorized fields. A partial view must not become a complete replacement by adding defaults.
Delete results have null values. Row IDs and revisions remain request-bound.
Use lossless JSON for i64/u64 values and revisions. JavaScript Number cannot retain the whole domain.

Create and replace use their complete allowed Resource encoding.
Patch includes only changed members; absence of a member means no requested change.
A set operation uses `{"op":"set","value":value}`. Removal uses `{"op":"remove"}`.
False, zero, empty strings, and null are explicit values. Removal requires an optional Resource field.

An action invocation nests `{"type":"action","input":{"name":name,"input":value}}`.
Unit actions use null input. An omitted retry epoch always means original epoch zero.
Unknown outcomes retain the exact operation, expected revision, original epoch, and idempotency key.
Current authority still applies when a durable receipt resolves the request.
A conflict is not permission to overwrite silently or generate a new mutation identity.

## Acceptance record and limits

The metadata API first failed with missing types/methods. The retained RED log records the intended missing-contract failures.
Native tests verify hidden nested references, exact byte limits, denied invocation, revocation, and metadata-only reopen on both stores.
HTTP fixtures verify aliased named inputs, scalar inputs, exact u64 action values, discovery, views, queries, live frames, and denial.
Maintained CLI discovery accepts additive metadata while preserving its existing names and version validation.
Renamed-crate compile fixtures inspect the generated input descriptors.

An initial HTTP expected-catalog assertion predated the additive metadata; its failure is retained in `http-before-fixture.log`.
The corresponding fixture now includes versioned input descriptions.
An initial live-fixture assertion compared JSON member order; parsing the actual SSE JSON corrects that test oracle.
This correction changes no product behavior.

No browser session, OIDC host, Studio control, storage-format change, or global Field registry belongs to this task.
ASD-STE100 is a writing guide. This document does not claim certified compliance.

## Focused verification

All Cargo commands ran in `rom-dev` from `/workspace/ROM/.worktrees/release-query-index`.
They used Rust/Cargo 1.99, two build jobs, and `/var/tmp/rom-release-measured-verification-target`.
Development and test debug information was disabled. No private Cargo target was created.

| Command or check | Result and retained scope |
| --- | --- |
| `cargo test -p rom --lib input_descriptor --locked` | Six final metadata tests passed; `final-acceptance.log` records this before a later fixture compile error. |
| `cargo test -p rom-storage-conformance --test studio_discovery --locked` | Three tests passed in `final-native-cli.log`; two tests exercise both stores. |
| `cargo test -p rom-http --test loopback discovery --locked` | Three actual HTTP cases passed in `bounded-metadata-final.log`. |
| `cargo test -p rom-cli --test loopback --locked` | Three tests passed in `final-native-cli.log`; existing discovery and both-store generic CLI paths remain compatible. |
| `cargo test -p rom -p rom-derive -p rom-http -p rom-cli --locked` | Affected suites and doctests passed in `affected-final.log`, before final test-only additions and borrowed metadata budget accounting. |
| `node tests/compile/check.mjs` | Expected diagnostic fixtures and renamed-crate descriptor assertions passed in `focused-checks.log`, before the subsequent Clippy warning. |
| `cargo clippy -p rom -p rom-derive -p rom-http -p rom-cli -p rom-storage-conformance --all-targets --locked -- -D warnings` | Passed in `final-native-cli.log`. |
| Scoped Rust formatting and `git diff --check` | Passed on owned source. |

The native compatibility case creates a custom-field Resource without presentation metadata, closes ownership, and reopens with codec metadata enabled.
Both SQLite and redb retain the same catalog layout, field value, row revision, journal head, and exact mutation replay.
Adding presentation metadata does not create a migration, another row change, or another event.

The coordinator owns the full combined `./scripts/check` after concurrent source stabilizes.
This task does not claim that the complete release or Studio acceptance has passed.

Earlier failures remain visible. They include missing metadata APIs, an outdated discovery expectation, and a Clippy nested-if warning.
The final native fixture first attempted the adapter-only `counts` helper through `dyn Storage`.
Its corrected public assertions compare snapshots and journal heads instead; no storage behavior changed.
