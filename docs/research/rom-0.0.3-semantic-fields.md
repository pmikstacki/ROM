# ROM 0.0.3 semantic Field contracts

Date: 2026-10-05. Status: implemented catalog; focused verification passed.

`rom-fields` uses the public `rom::Field` contract. Constructors and decoding run the same validation.
The crate does not infer meaning from field names. It does not fetch URLs, mailboxes, or unit definitions.

## Version-one catalog

All codec identities have version `1`. Every scalar uses `Shape::String`.
`rom.unit-value` uses `Shape::Map(Shape::String)` with exactly two keys.
These identities identify trusted compiled codecs. They do not add a global Field registry or a persistence migration contract.

| Rust type | Identity | Accepted and canonical representation |
| --- | --- | --- |
| `Date` | `rom.date` | Gregorian `YYYY-MM-DD`, years 0001 through 9999; invalid calendar dates fail. |
| `Time` | `rom.time` | `HH:MM:SS` with optional 1–9 fractional digits; trailing fractional zeros are removed. No timezone or leap seconds. |
| `DateTime` | `rom.datetime` | Explicit known RFC3339 offset input; canonical UTC `YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`. |
| `Color` | `rom.color` | sRGB `#RRGGBB` or `#RRGGBBAA`; canonical lowercase. No other CSS syntax. |
| `Email` | `rom.email` | ASCII dot-atom local part and DNS labels. Local case remains; domain becomes lowercase. |
| `Url` | `rom.url` | Absolute HTTP/HTTPS URL, parsed and normalized by `url`; no credentials, whitespace, controls, or backslashes. |
| `Multiline` | `rom.multiline` | Exact UTF-8 string, including empty text and original line endings; maximum one MiB. |
| `JsonDocument` | `rom.json-document` | Valid JSON document carried as exact text; maximum one MiB. Lexical numbers and whitespace remain unchanged. |
| `Decimal` | `rom.decimal` | Exact decimal string, optional minus, mandatory integer digits, optional nonempty fraction; maximum 1024 bytes. |
| `UnitValue` | `rom.unit-value` | `{"value":"1.2","unit":"kg"}`. Decimal magnitude plus explicit case-sensitive opaque unit token. |

`DateTime` rejects `-00:00`, missing offsets, leap seconds, and lowercase or space separators.
UTC conversion must remain within years 0001 through 9999. Exactly nine fractional digits support chronological string ordering.
Offsets establish an instant. They do not validate geographic timezone rules or infer daylight-saving transitions.

`Email` limits the local part to 64 bytes and the mailbox to 254 bytes.
DNS labels have 1–63 ASCII letters, digits, or internal hyphens; domain length is at most 253 bytes.
This deliberate subset excludes quoted local parts, display names, comments, address literals, and internationalized mailboxes.
Syntax validation does not establish mailbox ownership or delivery.

`Decimal` removes leading integer zeros and trailing fractional zeros. Negative zero becomes `0`.
It does not accept exponent notation, whitespace, a plus sign, JSON numbers, or floating-point conversion.
The type supports exact storage and editing. It does not provide decimal arithmetic or numeric query ordering.
Generic string range queries remain lexical for `Decimal` and `UnitValue`; authors must not use them as numeric comparisons.

Unit tokens contain 1–64 ASCII letters, digits, `_`, `-`, `/`, `.`, `%`, or `^`.
The token is an application label. It does not establish dimensional compatibility, currency scale, or conversion rules.
Application actions must enforce their allowed units and arithmetic rules.

`JsonDocument` validates through `serde_json::value::RawValue` without converting its numbers into `f64`.
The parser's normal nesting limits apply. Duplicate object keys remain in the exact source text.
This is a JSON text field, not a typed nested Resource descriptor or canonical JSON equality contract.

## Composition and existing lifecycles

Use `Option<T>` for explicit null. Use `rom::Presence<T>` for omission.
Lists and maps propagate codec identity and wrapper paths through the existing core implementation.
The semantic types do not change mutation intents, field defaults, or missing-field behavior.

`rom_fields::ResourceRef` reexports `rom::ResourceRef` with the same public type identity.
Use `ResourceRef<rom_blob::Blob>` for a typed Blob reference.
Target existence and deletion restrictions remain persistence responsibilities. Blob upload and authorization remain in the existing Blob lifecycle.
No new URL or byte-storage mechanism is introduced.

## Primary-source review

Chrono provides compiled Gregorian validation and fixed-offset timestamp parsing.
Default clock, local timezone, and WebAssembly features are disabled; only `std` is enabled.
Its official documentation describes ambiguous local time and the absence of bundled timezone data.
[Chrono 0.4.45](https://docs.rs/chrono/0.4.45/chrono/).

RFC3339 distinguishes known UTC offsets from the `-00:00` unknown-offset convention.
ROM deliberately rejects that convention and leap seconds in this codec.
[RFC3339](https://www.rfc-editor.org/rfc/rfc3339).

CSS defines hexadecimal RGB and RGBA ordering. ROM's subset accepts only the six- and eight-digit forms.
[CSS Color 4 hexadecimal notation](https://www.w3.org/TR/css-color-4/#hex-notation).

SMTP specifies case-sensitive local parts and case-insensitive domains. ROM preserves that distinction within its documented ASCII subset.
[RFC5321](https://www.rfc-editor.org/rfc/rfc5321).

The `url` crate implements WHATWG URL parsing. Parsing has no network side effect.
ROM adds its explicit HTTP/HTTPS and credential restrictions.
[url 2.5.8](https://docs.rs/url/2.5.8/url/).

`rust_decimal` supplies a finite fixed-precision arithmetic type. Its limits would impose a different accepted-value contract.
ROM's bounded text Decimal therefore uses a compiled grammar and has no arithmetic dependency.
[rust_decimal documentation](https://docs.rs/rust_decimal/latest/rust_decimal/).

Serde JSON exposes `RawValue` for complete JSON text validation without intermediate `Value` conversion.
The `raw_value` feature preserves ROM core's existing numeric `Value` semantics.
[RawValue](https://docs.rs/serde_json/latest/serde_json/value/struct.RawValue.html).

## Dependency and verification limits

Chrono `=0.4.45` and url `=2.5.8` already occur in the workspace lockfile.
The crate reuses workspace `rom` and `serde_json`; `rom-conformance` is a development dependency.
Chrono uses MIT OR Apache-2.0, and url uses MIT OR Apache-2.0.
The project's full dependency, license, compiler, and advisory gates remain required before release.
This source review does not substitute for an executed advisory scan or minimum-compiler check.

Focused tests exercise positive, rejected, canonical, boundary, numeric-precision, wrapper, and public Resource normalization cases.
They do not establish Studio interaction coverage, database durability, or release readiness.

Executed on 2026-10-05 in the `rom-dev` machine at `/workspace/ROM`:

```sh
cargo fmt -p rom-fields --check
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p rom-fields
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy -p rom-fields --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo doc -p rom-fields --no-deps
```

All commands passed. Nine integration tests passed; no unit tests or doctests were declared.
The negative profile test rejected revision 2. A deliberately incorrect Decimal canonical fixture failed the shared conformance assertion.
The shared catalog fixture passed the compiled backend decoder for all ten identities.
Studio can consume [`semantic-codecs-v1.json`](../../crates/rom-fields/tests/fixtures/semantic-codecs-v1.json) directly.

Compiler: `rustc 1.99.0 (b940084d7 2026-09-28)`.
Source baseline: `6d3b6b24b1447a78abb96c13e5957b3654745f07`, with concurrent 0.0.3 work uncommitted.
Relevant additions: `crates/rom-fields/**`, its workspace registration, and this report.
Executed lockfile SHA-256: `00ff7691b2b549d3fcd80dea2d671b821559be5c20a1fdeb88598ffe36af1c66`.
Shared fixture SHA-256: `e2f852cc17513723b68275bf3a22e12fbdc815f978d2be2cd2f424e06bd1530d`.
No full local verifier, browser journey, real-database semantic mutation, or packaged-consumer acceptance was executed in this focused task.

## Executed database stories

The later [`semantic_resources.rs`](../../tests/persistence/tests/semantic_resources.rs) test executes the same journey on file-backed SQLite and redb.
It uses generic Resource declarations, raw Invocations, a structured action input, and ordinary row, field, query, and sort policies.
`Measurement` exercises all ten semantic types. The unrelated `Equipment` Resource retains ordinary string, false, and zero values.

Create canonicalization covers lowercase colors, normalized email/URL, UTC timestamps, exact Decimal, unit magnitude, and preserved JSON text.
Different raw and canonical create requests reuse the same idempotency receipt.
Timestamp equality accepts an equivalent explicit offset. Timestamp range queries compare normalized instants.
The Decimal sort test explicitly confirms lexical order: `10` precedes `2`.
Studio must not describe this generic string order as numeric Decimal order.

Rejected multi-field patches include a valid title change and an invalid semantic value.
Each rejection preserves the exact stored row, revision, event count, and receipt count.
The journey also rejects invalid create and action input without writes.
Optional dates cover valid values, omission, explicit null, and removal. Lists and nullable maps retain their codec rules.

Field denial hides the secret in projected reads and authorized journal events.
Whole-record reads, forbidden predicates, private field writes, and unauthorized actions fail.
An unauthorized actor receives no query rows or journal events.
These checks preserve the same state and counts on both adapters.

After runtime shutdown and database reopen, the canonical row remains exact.
The prior action request replays its durable receipt without another event or state revision.
The final adapter counts are `[3, 6, 6, 0]`: three state records, six events, six receipts, and no effects.

Executed on 2026-10-05 with Rust 1.99.0:

```sh
rustfmt --edition 2024 --check tests/persistence/tests/semantic_resources.rs
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p rom-storage-conformance --test semantic_resources
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy -p rom-storage-conformance --test semantic_resources -- -D warnings
```

All commands passed. One integration test executes both real database journeys.
The first fixture run failed because the sort policy was not declared. The corrected explicit generic sort grant passed both adapters.
No storage implementation or semantic controller changed for these stories.

Source baseline remains `6d3b6b24b1447a78abb96c13e5957b3654745f07`, with concurrent release work uncommitted.
The root added the persistence test dependency. This task added only the test file and this evidence section.
Executed lockfile SHA-256: `f2e74a40bcdf6fad44e78abd32b7b689697248102ae6ac8355fe025e0c933095`.
Test source SHA-256: `544a8e67417788fcde2279b967cd39de2f3e4682f7651aaf86505c89b2ae9e5b`.
This focused result does not establish browser acceptance, process-crash durability, packaged-consumer acceptance, or full release verification.

## Enum labels and source compatibility

The 0.0.3 authoring contract adds advisory labels for exact enum wire values.
`Field::enum_labels()` defaults to an empty `BTreeMap<String, String>`.
`Option`, `Presence`, `Vec`, and `BTreeMap` forward the leaf's labels.
`Resource::field_enum_labels()` defaults to an empty list of `FieldEnumLabels` bindings.
The Resource derive generates these bindings. The Input derive includes labels in each member descriptor.

Definition registration validates the bindings and freezes the accepted maps.
A nonempty map requires an enum leaf after optional, nullable, list, and map wrappers.
Keys must match allowed wire values exactly. Maps have at most 1024 entries.
Labels must contain 1 through 256 UTF-8 bytes. Different wire values may have the same display label.
A label cannot change value normalization, allowed values, ordering, authorization, or codec identity.

Authorized discovery carries `enum_labels` on `DiscoveredField`, `InputFieldDescriptor`, and `InputDescriptor::Scalar`.
Empty maps are omitted from serialized discovery. Missing maps decode as empty maps.
The discovery response budget accounts for the labels before it clones their strings.
Denied fields and actions do not disclose their maps.
The persisted `Descriptor`, `Shape`, Resource version, and stored wire values remain unchanged.

Existing manual `Field` and `Resource` implementations keep their default hooks.
For Rust source migration, add `enum_labels: Default::default()` to manual `InputFieldDescriptor` and `InputDescriptor::Scalar` literals.
Manual `DiscoveredField` literals require the same new field.
Use `..` in matches that do not examine this advisory metadata.
This change adds fields to public Rust structures. It requires a source update for complete manual literals.
Studio's strict discovery parser must accept and validate the optional map with the paired 0.0.3 release.
An older strict client is not assumed to accept this new discovery member.

The shared positive fixture is [`enum-labels-discovery-v1.json`](../../crates/rom-fields/tests/fixtures/enum-labels-discovery-v1.json).
The negative fixture is [`enum-labels-invalid-v1.json`](../../crates/rom-fields/tests/fixtures/enum-labels-invalid-v1.json).
Its cases reject a non-enum shape, an unknown key, empty text, non-string text, and oversized UTF-8 text.
The native unit tests also cover excessive entry counts, duplicate bindings, unknown fields, wrappers, and duplicate display labels.

The public consumer test uses a real SQLite runtime.
It verifies exact discovery, authorized projection, wrapped labels, structured action input, and scalar action input.
Both action forms retain exact enum wire values.
Rejected display-label inputs preserve the state, event, and receipt counts.
Changing the label provider after registration does not change existing discovery.
A new runtime accepts renamed labels with the same persisted schema and replays the earlier create receipt.

Executed on 2026-10-05 with Rust 1.99.0:

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p rom --lib
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p rom-consumer --test enum_labels
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p rom-storage-conformance --test studio_discovery --test semantic_resources
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy -p rom -p rom-derive --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy -p rom-consumer --test enum_labels -- -D warnings
```

All commands passed: 95 core tests, two consumer tests, three discovery tests, and one test with both database journeys.
The new consumer fixture first failed compilation because of malformed nested type syntax and an ambiguous trait method.
These test-source errors were corrected before the passing runs.
The last added action assertion initially passed a `String` to an API that requires `&str`; its corrected test passed.
This focused evidence does not establish the full local verifier, package acceptance, or browser matrix.

Positive label fixture SHA-256: `6d6d9a10cf89b01b2c6c4fe510ea2032ec6e0f212f0f541621b16af172cd9f8e`.
Negative label fixture SHA-256: `351026ddc22a9518feafcbf877cfd3a2c1537ea40e1f058f89a6a0efebf94668`.
Consumer test SHA-256: `3cb039da5d0ef0a407255fcbd40c5409f05d9dfc1178b1a5b3d564a4d0a2a658`.

## URL Unicode whitespace conformance

The shared semantic vectors now accept U+FEFF in a URL path and require its percent-encoded form.
They reject U+0085 and U+2003 as Unicode whitespace.
The native URL decoder and Studio normalizer both execute the same fixture.
The earlier JavaScript `\s` check rejected U+FEFF, which Rust does not classify as whitespace.
The coordinated Studio change uses Unicode `White_Space` and explicit control checks.

Executed checks passed: `cargo test -p rom-fields --test shared_vectors` and Studio's semantic unit test file with 32 tests.
The updated semantic fixture SHA-256 is `692714b83b528cf978cb65e4c827380e4a85b7d40b6a95f99e1e443365ce5c32`.
