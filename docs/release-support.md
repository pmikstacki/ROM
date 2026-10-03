# Source release support boundary

Status: native source alpha accepted on 2026-10-03.
The [completion report](research/framework-release-completion.md) identifies the accepted source revision, artifacts, and executed evidence.

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
WASM, dynamic loading, RabbitMQ, Studio, shared tenancy, multiwriter deployment, and heavy analytics are outside this source release.
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
