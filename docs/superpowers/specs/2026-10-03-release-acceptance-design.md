# Final source release acceptance

Status: design for release tasks 4.5 and 4.6. Execution follows native conformance acceptance.

## Scope

Prepare the existing experimental native Rust framework for local source distribution.
Preserve the one Resource contract, optional transports, and disabled publication policy.
The owner has authorized implementation, parallel review, local artifacts, and source integration.

This work does not add a stable release promise or close every historical OpenSpec proposal.
Studio, WASM, RabbitMQ, shared tenancy, multiwriter operation, and heavy aggregation remain outside this release.
Independent Field identities and registry checks remain a separate open proposal.

## Application acceptance

Run the reference application's upgrade, process-exit recovery, operator, attachment, and current-authority tests against the final source.
Run the same application tests with ROM dependencies resolved exclusively from fresh crate archives.
Keep the separate real-provider acceptance command and its pinned provider profile.
Do not describe separate journeys as one composed provider-to-migration experiment.

The packaged application must use copied source and explicit dependency metadata.
It must not resolve ROM libraries through checkout paths.
Preserve features, optional dependencies, development dependencies, and package identities when constructing the external manifest.
Keep the existing packaged public consumer and CLI checks.
Source and archive identities must appear in the retained evidence.

## Server lifecycle

Both demo server modes must install signal reception before they report readiness.
The local synthetic server must drain accepted work and attachments after SIGINT or SIGTERM.
Share the Unix signal receiver between the synthetic and provider modes.
Keep signal ownership in the host application, outside core.
Use a regression that sends a signal immediately when readiness appears.
Bound test process ownership and preserve failed execution evidence.

## Local artifacts

The release command requires a clean checkout and runs the complete acceptance commands.
It produces a source archive, native skill bundle, checksums, and a manifest in an exclusive output directory.
Record the exact source revision, lock hash, toolchain, profile versions, and verification commands.
Do not overwrite an existing release or publish to a registry.
Stage incomplete outputs separately; expose the completed artifact directory only after all commands succeed.

The source archive includes maintained source and documentation, not build caches or private credentials.
Skill assets preserve their source identities and explicit source-checkout prerequisites.
An extracted source archive must pass skill admission with the matching bundle.
Archive verification is distinct from execution of every workflow.

## Support and completion audit

Publish a current support matrix, compatibility notes, and known limitations.
Record the Rust floor, tested platform, single-owner rule, storage formats, adapter and identity profiles, and source-only distribution policy.
Map each release requirement and scenario to implementation and executed evidence.
Preserve historical reports and their original scope.
Mark release tasks complete only after the final reviewed source passes acceptance and produces verified artifacts.

The final audit must distinguish automated author tests from human usability evidence.
It must distinguish process interruption from power-loss durability and a tested provider profile from universal interoperability.
