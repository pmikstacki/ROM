# ROM 0.1.0-alpha.1: integrated MVP acceptance

Date: 2026-10-02. Implementation revision: `463f78cc121d21632f5e9073ed7c9b5c7aafd42a`.
Subsequent completion/report changes are documentation and notice updates.

## Result and scope

One maintained library now combines the previously separate experiments. A typed
Resource declaration drives codecs, actions, persistence, queries, observations,
HTTP and durable reactions. A second unrelated Resource uses the same machinery.
The executable [author workshop](../../demo/README.md) connects configuration,
identity Resources, notification channels and real folder attachments as well.
No application repository, controller or broadcaster is required for each kind.

The workspace has eleven optional/composable library packages: rom, rom-derive,
rom-sqlite, rom-redb, rom-auth, rom-identity, rom-http, rom-config, rom-blob,
rom-blob-object-store and rom-backup. Database/HTTP/blob SDKs do not enter the
core dependency graph. Resource remains the managed application unit; internal
receipts, causal records and runtime handles implement guarantees around it.

This completes the scoped [MVP acceptance checklist](../../openspec/changes/integrate-resource-mvp/tasks.md).
It does not implement every future capability in the broader design proposals.
The [decision register](mvp-decision-register.md) classifies accepted decisions,
engineering defaults, host policy and deferred profiles. The
[complete index](completion-index.md) connects every research track and prototype
to its result and immutable source. Historical reports retain their original
limitations rather than being rewritten as present implementation claims.

## Independent review corrections

The [final review](maintained-mvp-independent-review.md) covers the combined implementation. Three
new reproduced findings were corrected before acceptance:

1. Blob shutdown could signal zero active work before releasing the task's final
   service owner. Accounting is now separate from service ownership; an original
   forced scheduling regression independently changed from failing to passing.
2. Durable work could fit in Pending state but lack bytes to become Leased.
   Admission now reserves worst-case lifecycle metadata, including retry and
   restore generations. Seven boundary tests cover progression and rejection.
   Incompatible old experimental ledgers explicitly fail; no migration exists.
3. A denied identity could reach Resource lookup and codecs before its current
   authority check. Authority now precedes lookup/normalization under bounded
   execution and the commit gate; later checks still protect commit/disclosure.

There are no unresolved material findings in that review. This is engineering
review and fault testing, not a security certification or a human usability study.

## Executed final gates

Coordinator runs used Rust/Cargo 1.99.0 on x86_64 Linux in persistent NixOS
`rom-dev`, with the retained Cargo.lock. All commands below completed successfully.
The final workspace lock SHA-256 is
`93f51f4bfa2a95d96becf960cb8e00ae32c5767c82c7d5ba8891397637681610`.

| Gate | Executed evidence |
|---|---|
| `./scripts/check` | All four strict OpenSpec changes, formatting, warning-free Clippy/docs, workspace tests/doctests, expected compiler failures, core feature/dependency isolation, consumer, auth and identity verifiers passed. |
| `./demo/verify` | Both SQLite/redb workshops passed actual TCP query/live/journal, typed mutation, custom codec failure, configuration, durable reaction/notification, denial, actual folder attachment/reopen/detach, serve/reopen/SIGINT drain. |
| Blob S3 verifier | Five real folder/MinIO tests passed, including an object committed before the response was lost. The explicit MinIO fixture version is recorded in the Blob report; no arbitrary S3 implementation is certified. |
| Native SQLite patch profile | Official pinned SQLite 3.53.4 archive and amalgamation hashes checked; actual linked version asserted. After the last code change, consumer and persistence suites passed 134 test invocations, including backup and ledger boundaries. Default bundled SQLite remains 3.53.2 through pinned rusqlite; the alternative profile is explicit. |
| Dependency advisory audit | cargo-audit with warnings denied, no ignored advisories, retained RustSec database commit `117edb3bed98e9be112f277b7615eea3252e7c43`: 269 lock entries scanned, no vulnerabilities/warnings reported. This does not certify native engine patch coverage. |
| License/source inventory | 255 external packages in the all-features/target/dev graph, declarations and available upstream notices retained in [inventory](maintained-dependencies.md) and [THIRD_PARTY_NOTICES](../../THIRD_PARTY_NOTICES.md). See [notice review](maintained-license-review.md) for distribution limits. |
| `./scripts/release` | Clean code checkout passed verifier, optimized workspace build, all eleven extracted library archives built with all features, and a consumer outside the repository ran using only extracted ROM sources. Root lockfile stayed unchanged. Source tarball and SHA-256 generated locally; no crates.io publication or GitHub Actions. |

The complete check/demo/S3 run contains repeated package-specific verifier runs;
we do not present their sum as a unique test count. One subprocess test helper is
intentionally ignored by ordinary discovery and invoked explicitly by its parent
fault test. Unused-patch Cargo warnings during standalone archive checks refer to
other optional ROM packages absent from that particular crate's graph, not to
uncompiled archives: each library is checked individually.

Local coordinator logs: `/tmp/rom-final-freeze-check.log`,
`/tmp/rom-final-native-freeze.log`, `/tmp/rom-final-dependencies.log`,
`/tmp/rom-final-release.log`. Extracted packaging evidence is retained inside
rom-dev at `/var/lib/rom-package-checks/rom-packaged-consumer-vN8Zzy`.
Logs are supplemental; committed scripts, tests, lockfile and source pins allow
reproduction without those temporary paths.

## Reproduce and use

```sh
# Within the prepared rom-dev container, from /workspace/ROM:
./scripts/check
./demo/verify
cargo run -p rom-demo -- smoke sqlite
cargo run -p rom-demo -- smoke redb
# See demo/README.md for the persistent local serve command and curl walkthrough.
TMPDIR=/var/lib/rom-package-checks ./scripts/release
```

The release script requires a clean checkout and refuses to overwrite an archive
for the same revision. Create the persistent TMPDIR first; it must be outside
this Cargo workspace. Source archives live in ignored `dist/` with checksums.
Use the [native profile script](../../scripts/check-sqlite-native) and
[Blob verifier](../../crates/rom-blob-object-store/verify-s3) for their extra gates.

## Explicit limits

- One Runtime owns writes/invalidation for a storage deployment; independent
  multiwriters are unsupported and host misuse is not automatically prevented.
- Queries are bounded canonical equality/conjunction and moving keyset pages,
  not indexed arbitrary joins, aggregates or repeatable snapshot pagination.
- Idempotency is durable. No completed-outcome cache/single-flight optimization
  or Salsa mutation memoization is enabled; pure proposals may compute twice.
- Reactions preserve prior commits and retry downstream work within budgets.
  External notifications can duplicate after uncertain acknowledgement/restore
  unless the receiver deduplicates the stable delivery identity.
- Current auth is enforced, but provider discovery/login/session ceremonies,
  first-admin account recovery and tenant product policy belong to host profiles.
  Demo credentials are synthetic loopback fixtures, not a production login.
- Blob bytes are external to database transactions/backups. Detachment is not
  physical deletion. Host must manage backup completeness, original-host fencing,
  trusted directories, private data at rest and any safe physical cleanup.
- Configuration ingestion is a complete, single-target reload with ownership and
  provenance; broad layered partial overlays are not silently enabled.
- Format 3 rejects unsupported formats and insufficient legacy ledger headroom.
  There is no automatic data migration, encryption-at-rest or physical-erasure claim.
- WASM, RabbitMQ adapter, production Studio/discovery, multi-tenant/distributed
  deployment and a human author study remain separate milestones. Their research
  is indexed; a mock is not represented as a completed production interface.
