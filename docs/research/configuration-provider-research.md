# Configuration providers for ROM Resources

Research snapshot: 2026-10-02. The review covers documentation, published metadata and source only. No dependencies were installed. No prototypes were built or tests run. These are candidates, not an adoption decision. Beskid-specific findings belong to the separate investigation.

**Recommendation:** probe `config-rs` first as a source-loading adapter, compare Figment where provenance matters, and defer confique as the universal importer. No reviewed crate supplies ROM's configuration control plane: ownership, reconciliation, authorization, Resource validation, activation and rollback remain ROM responsibilities.

`AppSettings`, `IdentityProviderConfig` and `User` remain Resources. Built-in configuration Resources use the same accepted descriptors and runtime handling as manually or procedurally authored Resources. External files and providers supply candidate values or desired instances; they do not introduce a competing definition system. Studio consumes these Resources and their accepted metadata. This follows the existing [implementation boundary](core-implementation-readiness.md) and [auth architecture](auth-architecture.md).

## Candidate matrix

| Candidate | Reusable capability | Main mismatch / decision |
|---|---|---|
| **config-rs (`config`) — first probe** | Generic value tree, files/environment, ordered sources, Serde extraction; custom synchronous `Source` and asynchronous `AsyncSource` | ROM must add stronger provenance, explicit import operations and activation. Recent release requires Rust 1.88. |
| **Figment — comparison candidate** | Generic providers, profiles, several merge modes, per-value metadata and contextual extraction errors | Provider API is synchronous; profile/global precedence can surprise; older release and no declared MSRV need checking. |
| **confique — defer universal use** | Strong ergonomics for a closed Rust configuration struct, generated layers, defaults, environment names and validation | Requires another typed configuration model; ordinary optional leaves cannot express every missing/null/clear distinction. Could suit private bootstrap configuration. |
| **Serde + selected parsers — supporting candidates** | Decode external representations into an intermediate import envelope and the existing Resource codecs | Serialization is neither Resource metadata nor authorization. Avoid requiring a parallel confique-style derive on every Resource. |
| **notify — optional later** | Filesystem change hints and polling alternative | A watch event is neither a coherent snapshot nor a successful configuration transaction. |

The extensibility claims come from the actual [config Source](https://docs.rs/config/0.15.27/config/trait.Source.html), [AsyncSource](https://docs.rs/config/0.15.27/config/trait.AsyncSource.html), [Figment Provider](https://docs.rs/figment/0.10.19/figment/trait.Provider.html) and [confique Layer](https://docs.rs/confique/0.4.0/confique/trait.Layer.html) contracts.

## Provider contracts and merge behavior

**config-rs:** a custom `Source` collects a `Map<String, Value>`; boxed sources also need its cloning contract. `AsyncSource` supports asynchronous collection but ships without built-in asynchronous providers. Registering sources does not read them until `build`; defaults apply first, sources in order, explicit overrides last. The inspected builder awaits asynchronous sources sequentially and runs synchronous collection inline: an async builder does not automatically parallelize remote calls or move blocking file work off Tokio. ROM should acquire remote snapshots with bounded concurrency, then apply deterministic precedence. [Builder documentation/source](https://docs.rs/config/0.15.27/src/config/builder.rs.html), [source contracts](https://docs.rs/config/0.15.27/src/config/source.rs.html).

**Figment:** providers return profile-indexed dictionaries plus metadata and an optional profile. Composition reads provider data eagerly. Ordinary `merge` recursively combines dictionaries and replaces existing scalar/array values; `join` keeps existing scalar/array values. `admerge`/`adjoin` concatenate arrays. Default profiles provide fallback; global-profile values override profile values, so global is more than “the next layer.” Keep this behavior out of ROM's public semantics unless deliberately selected. [Provider](https://docs.rs/figment/0.10.19/figment/trait.Provider.html), [Figment composition](https://docs.rs/figment/0.10.19/figment/struct.Figment.html), [profiles](https://docs.rs/figment/0.10.19/figment/).

**Confique:** layers are typed intermediate structs; earlier layers have priority, opposite the typical config-rs append order. External providers can deserialize or construct a layer and combine it with `with_fallback`; they need not be built-in file providers. Nested configuration recurses; ordinary leaf collections are chosen as whole values. Leaf validation occurs during deserialization, even if that layer's value would later lose; whole-struct validation occurs after combination. [Crate documentation](https://docs.rs/confique/0.4.0/confique/), [Config derive](https://docs.rs/confique/0.4.0/confique/derive.Config.html).

| Case | Observed loader behavior | Proposed ROM import rule |
|---|---|---|
| Missing field | Generally lets a lower layer survive | Means “no contribution from this layer.” |
| Explicit null | config has `Nil`; Figment has `Empty`; confique's ordinary optional leaf uses the same `Option<T>` slot for absent and null | Preserve null distinctly until Resource validation; null is a value, never implicit deletion. |
| Object/table | config-rs and Figment recursively merge maps | Permit only where the Resource/import contract declares layered fields. |
| Array | config-rs whole-array overlay replaces; Figment can replace or concatenate; confique leaf chooses one layer | Default to replacement; never silently concatenate trusted issuers or audiences. Merge-by-ID needs an explicit contract. |
| Delete instance/key | General overlay does not supply Resource lifecycle semantics | Explicit tombstone or reconciliation of a provider-owned complete set; a failed fetch cannot mean “delete everything.” |

The config-rs implementation recursively merges tables and replaces other values. Indexed path edits are a separate mechanism. Its source keys can be parsed as paths. For Resource identifiers that contain dots/brackets, use an explicit encoding to prevent accidental path interpretation. [Path implementation](https://github.com/rust-cli/config-rs/blob/main/src/path/mod.rs), [Source implementation](https://docs.rs/config/0.15.27/src/config/source.rs.html). Figment's empty variants represent `None` and unit, not lifecycle commands. [Empty](https://docs.rs/figment/0.10.19/figment/value/enum.Empty.html). Confique generates `Option<inner_type>` and uses `self.field.or(fallback.field)` for ordinary leaves: explicit null cannot clear a lower populated optional value through that representation. Custom codecs could change this, but the default is insufficient. [Reviewed generation source](https://github.com/LukasKalbertodt/confique/blob/63afcdcf341471acd82a33b302f582c4ab1820ff/macro/src/gen/mod.rs).

A hierarchy such as defaults → deployment file → environment-specific file → host override is possible. Its exact order remains a ROM policy decision. Profiles, tenant scopes and Resource references are different concepts. A `User` being a Resource does not imply that every user's identity should be assembled by arbitrary global overlays.

## Diagnostics and environment handling

Figment is the strongest provenance candidate: tagged values retain provider/profile metadata; errors expose kind, path, metadata and profile. It warns that Serde flattening can interfere with error attribution. This is winning-value provenance, not automatically the complete history of overridden inputs. [Metadata/extraction](https://docs.rs/figment/0.10.19/figment/struct.Figment.html), [errors](https://docs.rs/figment/0.10.19/figment/struct.Error.html).

config-rs values carry an optional origin string and type errors can include origin and key. That is useful but weaker than a structured ROM diagnostic containing source ID, generation, Resource identity, field path and safe error code. Confique provides structured internal causes and error chaining, without an equivalent public per-value origin tree in the reviewed API. ROM should retain a provenance sidecar and redact sensitive values before diagnostics reach logs or Studio. [config Value](https://docs.rs/config/0.15.27/config/struct.Value.html), [ConfigError](https://docs.rs/config/0.15.27/config/enum.ConfigError.html), [confique error source](https://github.com/LukasKalbertodt/confique/blob/63afcdcf341471acd82a33b302f582c4ab1820ff/src/error.rs).

For environment input, config-rs offers prefix/separator handling, optional scalar parsing and list parsing. Enabling a list separator can turn all remaining strings into lists unless list keys are restricted. Figment can filter/map keys but normalizes case and handles non-Unicode input lossily. Confique binds explicit environment names and supports custom parsers; an empty value that fails parsing/validation can be treated as unset. These are materially different contracts. [config Environment](https://docs.rs/config/0.15.27/config/struct.Environment.html), [Figment Env](https://docs.rs/figment/0.10.19/figment/providers/struct.Env.html), [confique configuration attributes](https://docs.rs/confique/0.4.0/confique/).

ROM should snapshot an allowlisted namespace, reject normalization collisions, and define empty-string, boolean, numeric and list syntax explicitly. Prefer descriptor-aware parsing over broad inference. Environment access must not expose unrelated process secrets or let names select arbitrary Resource fields.

## What ROM must own

The following is a proposed adapter boundary, not an adopted API:

1. **Acquire:** each provider returns a bounded candidate snapshot with source identity, revision/fingerprint, capture time, owned scope and provenance. Authentication, retries and remote timeouts belong here. Keep blocking I/O off runtime workers and reject stale asynchronous completions.
2. **Normalize and validate:** preserve missing/null/value/delete distinctions; merge only declared fields; invoke the same Resource codecs, descriptor checks, policy and cross-reference validation as other writers. A loader's successful Serde extraction does not confer permission to create trusted identity providers.
3. **Plan reconciliation:** distinguish initial seeding from continuous external management. Decide whether Studio edits are allowed, rejected or become a separate explicit overlay. A source must not silently overwrite another owner's changes. Record expected revisions and a reviewable change set.
4. **Activate:** publish a complete accepted generation only after required validation succeeds. Keep the last accepted generation on parsing/fetch failure; report pending/error status separately. Startup without a required valid configuration must have explicit failure behavior. Atomic multi-Resource activation is a requirement to investigate, not a capability established by this research.

For authentication settings, “last good” also needs a freshness policy: indefinitely retaining withdrawn issuer trust is unsafe. The auth layer must define which expiry/revocation failures deny new operations. Secret retrieval failure and missing configuration are separate states.

Filesystem notifications can trigger a debounced reread. Editors can replace files, events can be missed, and some filesystems require polling. Multiple files require a generation/manifest convention if their collective snapshot must be coherent. `notify` supplies observation, not this transaction protocol. [notify documented caveats](https://docs.rs/notify/8.2.0/notify/).

Store external secret **references** in configuration Resources; resolve them through host-authorized provider clients. Do not place fetched secret material into generic snapshots, audits or Studio responses. 1Password documents reference identifiers, and Vault KV v2 exposes versioned secret reads; neither requires ROM to invent secret storage. Resolver freshness, access rights, caching and redaction remain explicit adapter concerns. [1Password references](https://www.1password.dev/cli/secret-references), [Vault KV v2 API](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2).

## Maintenance and format choices

Published metadata and default-branch activity were checked on the snapshot date. Commit activity is evidence of maintenance, not a support guarantee; the older projects were not marked abandoned.

| Crate | Latest stable / published | Declared MSRV; license | Observed default-branch activity |
|---|---|---|---|
| config | 0.15.27 / 2026-09-30 | 1.88; MIT OR Apache-2.0 | 2026-10-01 |
| figment | 0.10.19 / 2024-05-17 | Not declared in published metadata; MIT OR Apache-2.0 | 2024-09-13 |
| confique | 0.4.0 / 2025-10-27 | 1.68.2; MIT OR Apache-2.0 | 2025-10-27 |
| notify | 8.2.0 / 2025-08-03 | 1.77; CC0-1.0 | 2026-09-21 |

Version/license/MSRV sources: crates.io metadata for [config](https://crates.io/api/v1/crates/config), [figment](https://crates.io/api/v1/crates/figment), [confique](https://crates.io/api/v1/crates/confique), [notify](https://crates.io/api/v1/crates/notify). Activity sources: official histories for [config-rs](https://github.com/rust-cli/config-rs/commits/main/), [Figment](https://github.com/SergioBenitez/Figment/commits/master/), [confique](https://github.com/LukasKalbertodt/confique/commits/main/), [notify](https://github.com/notify-rs/notify/commits/main/). Verify the selected feature/dependency graph's actual MSRV in a later executable trial.

Prefer a minimal TOML/JSON surface initially. TOML has no null value, so clearing/deletion requires an explicit import encoding, not an empty string convention. JSON can carry null. Current standalone parser metadata: `toml` 1.1.6+spec-1.1.0 declares Rust 1.85; `serde_json` 1.0.151 declares 1.71; both MIT OR Apache-2.0. These standalone versions are not a claim about each loader's transitive selection. [TOML specification](https://toml.io/en/v1.1.0), [toml metadata](https://crates.io/api/v1/crates/toml), [serde_json metadata](https://crates.io/api/v1/crates/serde_json).

Treat YAML as optional. The original `serde_yaml` is explicitly deprecated/unmaintained; Figment and confique's published YAML features use it. config-rs uses `yaml-rust2`. Inspect the chosen parser and feature graph before enabling YAML rather than adopting every loader default. `config`'s broad defaults include several formats and async support; select only necessary features. [serde_yaml status](https://docs.rs/serde_yaml/latest/serde_yaml/), [config features](https://docs.rs/crate/config/0.15.27/features), [Figment features](https://docs.rs/crate/figment/0.10.19/features), [confique features](https://docs.rs/crate/confique/0.4.0/features).

## Smallest useful disposable probe

Compare config-rs and Figment behind one ROM-owned snapshot envelope using `AppSettings`, `IdentityProviderConfig` and an ordinary `User` through the same Resource contract. Use defaults, two files, an allowlisted environment map and a fake asynchronous remote provider. Measure adapter code and diagnostic quality, not speculative throughput.

Exercise nested overlays, whole-array replacement, missing versus null, explicit deletion, dotted identifiers, invalid overridden values, environment collisions, redaction, failed reload, out-of-order remote completion, unchanged snapshots and conflict with a Studio edit. Demonstrate that failed candidates never become active and that source removal cannot erase unmanaged Resources. Stop short of live secrets or infrastructure. Adoption should depend on this semantic probe; a “universal configuration library” claim is justified only for loading, never the complete Resource control plane.
