# Source release support boundary

Status: native source alpha accepted on 2026-10-03.
The [completion report](research/framework-release-completion.md) identifies the accepted source revision, artifacts, and executed evidence.
The current branch prepares 0.0.2 with Studio. It is not yet an accepted release.
The [execution record](research/rom-0.0.2-progress.md) identifies its tested slices and remaining gates.

## Supported profile

| Area | Boundary |
| --- | --- |
| Distribution | Local source archives and native author skill bundles; MIT; no registry publication |
| Version | `0.1.0-alpha.1` plus the exact source revision and Cargo.lock hash |
| Rust | Edition 2024; declared and tested floor Rust/Cargo 1.99.0 |
| Acceptance host | Linux in the persistent `rom-dev` NixOS container; Node.js 22 for fixture and packaging tools |
| Execution | Tokio and Rayon, with ROM-owned work, admission limits, and shutdown |
| Persistence | SQLite and redb adapters; one active native owner per database |
| Transport | Optional HTTP adapter and generic CLI over the Resource contracts |
| Identity | Explicit provisioning and the pinned service-token profile in [provider deployment](provider-deployment.md) |
| Blob storage | Folder and the explicit local MinIO acceptance profile; external bytes remain outside database snapshots |
| Extensions | Trusted Rust source composition and [native conformance revision 1](native-extensions.md) |

The tested Linux host does not establish support for every operating system or filesystem.
Rust source APIs remain experimental. No stable binary ABI or cross-release source compatibility is promised.
Use one matching source package set. Recompile extensions when you change it.
This release does not specify a support response-time commitment.

## Data compatibility

Current native storage format is 8. Current archive format is 6.
Ordinary open rejects unsupported native markers.
[Native upgrade](native-upgrade.md) describes explicit conversion from formats 3 through 7.
[Resource migration](resource-migrations.md) preserves declared historical request codecs and unfinished obligations.
[Retention](retention.md) requires explicit policy and restore fences.

Keep a verified backup before offline maintenance. Preserve external blob bytes separately.
Stop the active owner before maintenance and do not modify a live database with another tool.
An old mutation receipt still requires current authority and its retained historical request codec.
A retry with an unresolved outcome must keep the original operation identity.
[Operator recovery](operator-recovery.md) describes retry, reconciliation, and explicit compensation.

## Limits

An action changes a Resource through the common commit contract.
Reactive chains keep earlier commits and retry a failed step within declared budgets.
Compensation is an explicit Resource action; it does not erase history or infer rollback from a timeout.
External delivery cannot guarantee deduplication without the declared provider or receiver support.

Queries use the common semantic model and checked adapter strategies.
Moving pagination does not promise a stable snapshot across requests.
Local query measurements apply to their recorded workloads, not every database or deployment.

Independent Field identities and a Field registry remain open proposals.
WASM, dynamic loading, RabbitMQ, Studio, shared tenancy, multiwriter deployment, and heavy analytics are outside the accepted native alpha.
The real-provider fixture does not establish universal OAuth compatibility, human login, or production TLS deployment.
Process-exit tests do not certify machine power-loss durability.
Automated author tests do not establish human usability or measured productivity improvement.

## Verification and maintenance

Run the complete local gates before distributing changed source.
Use the release skill for the command sequence and the release report for executed evidence.
Keep failed checks and known limitations visible. Do not infer readiness from a selected example alone.

The [local artifact producer](../scripts/release-artifacts/README.md) runs the fixed acceptance sequence from a clean checkout.
It writes source and skill archives, gate logs, a manifest, and checksums to an exclusive directory.
Artifact publication requires the tested Linux/ext4 host and GNU tools described in that procedure.
This requirement applies to artifact preparation, not to the generic persistence contract.

Report a defect with the source revision, lock hash, safe error category, and a minimal synthetic reproduction.
Exclude credentials, private Resource values, and customer databases from reports.
GitHub hosts source; local scripts perform verification and artifact preparation.

## Prepared 0.0.2 package set

The maintained workspace manifests now use version `0.0.2`. All local ROM dependency constraints use the same exact version.
The frontend manifest and lock identify `0.0.2` separately from the Cargo lock.
Use the complete matching source package set. Do not mix native alpha packages with 0.0.2 packages.

The owner's requested identifier sorts before `0.1.0-alpha.1` under semantic version ordering.
It does not describe a rollback of the accepted native alpha's code or data formats.
Native storage format 8 and archive format 6 remain unchanged.
The historical native alpha artifact and its source revision remain available with their original identities.

The new Studio metadata changes experimental Rust source APIs.
Manual `FieldCodec` and input descriptor literals need the `codec_wrappers` member.
An empty path means that the codec owns the complete declared shape.
Recompile extensions against the complete 0.0.2 source set.

The prepared Studio uses static Svelte and shadcn-svelte assets with an optional Rust host.
The host verifies human OIDC, maintains server-side sessions, and enforces current Resource authority.
The executed provider profile does not establish compatibility with every identity provider.
Actual native/browser tests do not establish production TLS deployment or client VPN reachability.
Final artifact acceptance and protected preview acceptance remain release gates.

## Studio source authoring for prepared 0.0.2

The [public Studio facade](../studio/src/index.ts) exports the application, client, renderer registration, shared types, and input control.
The [external author example](../examples/studio-consumer/README.md) uses these exports from an independently extracted Studio source tree.
Its custom renderer uses codec identity, not Resource-specific view branches.
This is a source-consumer contract. It is not a registry-published npm package.

The [executed author workflow](research/rom-0.0.2-studio-author-workflow.md) uses Linux, Node.js 22, pinned dependencies, and an explicitly selected immutable native binary.
Chromium and WebKit test the actual provider against SQLite and redb.
These automated tests do not establish human usability or support for every provider or operating system.

The [current scenario index](research/rom-0.0.2-scenario-index.md) separates scoped evidence from pending complete release acceptance.
The historical native-alpha scenario report retains its original six-gate scope.
