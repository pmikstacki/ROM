# Native extension conformance and author skills

Status: release stage 4.4 implemented and accepted. The native conformance results record the executed checks and limits.

## Purpose

Give extension authors a versioned compatibility profile, shared executable assertions, and portable instructions with tested examples.
Authors must use the public Resource pipeline. A module must not need its own repository, controller, or worker implementation.
The existing release goal authorizes this work and parallel implementation with independent review.

## Release boundary

This alpha supports trusted Rust extensions compiled with the application.
It makes no stable binary ABI, dynamic loader, WASM, or arbitrary remote-provider compatibility promise.
Core remains independent of transport, database drivers, conformance fixtures, and agent skills.
Native storage format 8 and archive format 6 remain unchanged.

The selected alpha profile versions custom codecs through their enclosing Resource and retained request codecs.
It does not implement independent Field identities, an independent Field registry, or cross-codec identity collision checks.
Those requirements in the still-open `establish-rom` proposal remain open.
Resource versions and independent Field versions are not equivalent.
The release completion audit must preserve this distinction.

Adding a Field registry now would change registration, descriptors, query metadata, and historical upgrade requirements.
That is a separate feature design. It is not needed to distribute and test the currently implemented native extension contracts.
Current collection-ordering rejection remains `Invalid`; unsupported adapter ports retain their documented `Unsupported` results.

## Version ownership

Publish one native profile guide with a machine-readable manifest.
Profile revision 1 identifies this conformance and instruction contract. It does not replace the versions below.

| Boundary | Existing version owner |
| --- | --- |
| Resource and Field codecs | Resource descriptor and explicit historical request codec |
| Action input | Compiled Input codec within the action/Resource contract |
| Reactions and channels | Positive definition versions and persisted consumer bindings |
| Query adapters | Query semantics, profile, and encoding constants |
| Operator transport | Operator protocol constant |
| Native storage and archives | Their existing format markers and explicit upgrade contracts |
| Native Rust source API | Exact alpha package set and tested source/lock identity |
| Author skills | Bundle/profile revision and declared workflow features |

The guide must give each boundary's mismatch behavior, optional features, and upgrade obligations.
An alpha package number alone does not identify every historical source revision.
Verification records include source and lock hashes. A tracked manifest does not contain its own future commit hash.

## Shared conformance library

Add `crates/rom-conformance` as a development-only consumer of public APIs.
Keep the workspace publication policy. Include the crate in source package checks.
Use existing dependencies. Blob assertions use an optional `blob` feature.
Keep `lib.rs` as a facade and place behavior in named modules.

The public signatures are grouped by module below. Implementation bodies are omitted.

```rust
pub const PROFILE_VERSION: u32 = 1;
pub type ConformanceResult = Result<(), ConformanceError>;

// profile module
pub fn require(version: u32) -> ConformanceResult;

pub struct CodecCase<F> {
    pub input: rom::Value,
    pub canonical: rom::Value,
    pub expected: F,
}
// field module
pub fn codec<F: rom::Field + PartialEq>(
    cases: &[CodecCase<F>], invalid: &[rom::Value],
) -> ConformanceResult;

pub trait StorageFixture: Send {
    fn storage(&self) -> std::sync::Arc<dyn rom::Storage>;
    fn facts(&self) -> rom::Result<StorageFacts>;
    fn reopen(&mut self) -> rom::Result<()>;
}
pub struct StorageFacts {
    pub counts: [u64; 4],
    pub events: Vec<rom::Row>,
    pub effects: Vec<(String, rom::Intent)>,
}
// storage module
pub async fn basic(
    factory: impl Fn() -> rom::Result<Box<dyn StorageFixture>>,
) -> ConformanceResult;
pub async fn for_profile(
    version: u32,
    factory: impl Fn() -> rom::Result<Box<dyn StorageFixture>>,
) -> ConformanceResult;
pub fn assert_bundle(
    storage: &dyn rom::Storage,
    facts: &StorageFacts,
    previous: &rom::Receipt,
    bundle: &rom::Bundle,
) -> ConformanceResult;

// blob module, enabled by the blob feature
pub async fn basic(store: &dyn rom_blob::BlobStore) -> ConformanceResult;
```

The module names identify public paths, such as `rom_conformance::storage::basic`.
`ConformanceError` exposes static profile/case information and a safe failure category.
It must not retain arbitrary adapter errors, stored values, or credentials.
A profile mismatch fails before fixture operations.
`basic` delegates to `for_profile` with `PROFILE_VERSION`; an unsupported requested version must not invoke the factory.
`StorageFacts.counts` contains row, event, receipt, and effect counts in that order.
`assert_bundle` shares the fresh-store, two-commit baseline assertion with native fault tests. It does not validate arbitrary application history.

Extract assertions from the maintained consumer, persistence, and blob tests. Replace their original bodies with calls to the shared assertions.
Keep native fault injection, process-exit scenarios, policies, and fixture setup with their owners.
Local fixture-owner types implement `StorageFixture`; external crates must not need orphan trait implementations.

The storage factory creates a fresh, exclusive backing store for each scenario.
The runner releases its clones and runtime owners before fixture reopen.
Assertions cover atomic state/receipt/event/effect results, stale revisions, input fingerprint mismatch, exact replay, and reopen.
The `facts` method is a test inspection seam, not a new production Storage method.
Passing this baseline does not replace native crash and recovery tests.

Field assertions compare declared canonical values and typed results, including invalid input and canonical round trips.
Registration and manual/derive equivalence remain separate public-consumer tests.
Blob assertions use a disposable namespace and a sixteen-byte object bound.
They retain conditional-create races, exact bounds, empty objects, and idempotent deletion.
Default tests use local storage. Real S3 acceptance remains an explicit separate command.

## Author skills

Provide four source skills: Resource/action authoring, native module/adapter authoring, operator diagnosis, and local release verification.
Use precise triggers and one canonical contract guide.
The skill package must include every referenced guide, script, and template asset.
Shared assets remain one maintained source; distribution copies identify that source.

Each workflow must check the requested profile and its features before executing its example.
Examples use public APIs and named modules. They must expose meaningful rejected input or invalid setup.
Operator instructions preserve uncertain outcomes and stable request identity; they must not turn a timeout into an automatic new mutation.
Release instructions prepare and verify local artifacts without publication or external credentials.

A deterministic assembler creates the portable instruction bundle.
The bundle accepts an explicitly supplied ROM source or extracted-package location; it contains no hard-coded developer checkout path.
Its verifier rejects an obsolete profile, missing feature, missing asset, or broken local link.
If a workflow needs the full source checkout, its manifest and instructions must say so.
Do not claim that an instruction bundle contains compiled ROM or every release artifact.

Before writing skills, run pressure scenarios without them and record the actual baseline.
An independent agent then follows one held-out task per workflow using the extracted bundle.
Record artifacts and failures separately from Rust test results.
An agent pass does not establish human usability or a measured productivity improvement.

## Acceptance

- The existing native suites call the extracted public assertions and retain stronger engine-specific tests.
- A separate public consumer invokes codec and module fixtures through the supported API.
- A deliberately broken fixture fails its named assertion; a wrong profile fails visibly.
- The folder adapter and explicitly selected S3 tests share the same blob assertions.
- Existing duplicate registration, unsupported port, descriptor mismatch, and historical replay negatives remain covered.
- Extracted skills contain their references and reject compatibility and packaging errors.
- Core's feature-isolated dependency check excludes conformance and skill machinery.
- Full local checks and package-consumer checks pass on the final source.

This stage does not close release tasks 4.5 or 4.6. They retain application recovery acceptance, final artifacts, support notes, and the completion audit.
