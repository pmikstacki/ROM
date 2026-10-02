# Maintained author demo results

Question: can a small author application use ROM's public library APIs for unrelated Resources, durable behavior, configuration, and generic HTTP without bespoke controllers or repositories?

The maintained `demo/` application answers this with Task and InventoryItem declarations, a canonical StockCode Field, typed patch, and a Task → Dashboard action → typed Channel<String> notification chain. Settings uses SourceActivation and config-rs TOML ingestion. The same declarations run on SQLite and redb. Source and identity management use ordinary Resources and explicit host bootstrap. No core or adapter implementation was changed for the application.

## Observed verification

`./demo/verify` passes on native Rust1.99 with two build jobs. Three tests cover the shared TCP scenario on both databases, custom codec positive/negative inputs, and README wire command deserialization. Two additional CLI smoke runs pass. Both databases pass actual server process startup, SIGINT drain, reopen of the same file, and another SIGINT drain. The finite scenario checks two generic Resource kinds, canonical query, invalid input rejection, denied User reads, denied source-owned Settings writes, typed patch/live refresh, complete/action/live removal, three journal facts and cursor resume, one typed notification, and zero owned work after shutdown. Clippy denies warnings; rustdoc denies warnings.

Dependency audit passed with 233 resolved dependencies, 1280 cached advisories, `--deny warnings --no-fetch`. The shared advisory database was already recorded in the repository's MVP dependency audit; this run did not refresh it. Only new external lock entries are Tokio signal support (`errno`, `signal-hook-registry`).

Exact local rerun:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/author-demo && CARGO_TARGET_DIR=/workspace/ROM/target/author-demo CARGO_NET_OFFLINE=true ./demo/verify'
```

The implementation initially compared entire Actor values in app policies. The configuration loader clips expiry, so this denied its otherwise valid Service. The failed smoke exposed it; policy now compares authority + kind + subject, leaving expiry validation to runtime. The README action JSON also needs the generic tagged enum's nested input object; a regression parses all documented mutation bodies into the public Invocation type.

## Author ergonomics and boundaries

Resource declarations, generated selectors and actions remain small. The longer host setup is explicit policy, bootstrap, registry, storage and lifecycle plumbing. Domain code does not need SQL, table names, routes, raw JSON, or transport types. Typed reactions target existing Resource actions, preserving their policies and revisions.

Configuration policies are currently verbose: authors must explicitly distinguish administrative fields from the worker's reload request fields. A future helper could package this policy without granting source-defined schemas or bypassing authorization. Principal matching must ignore incidental expiry/stamp details while preserving kind; a public principal comparison helper could remove the demonstrated footgun.

The local HTTP session uses a public synthetic marker and an Embedded principal, restricted to loopback hosting. A seeded disabled IdentityProvider and explicit IdentityLink illustrate normal Resource management; no actual login, Human authentication or email merge is implied. Production provider activation/binding is covered by the separate rom-identity verifier and must replace this resolver. The in-memory notification sink deduplicates within a process only. The app claims no durable external effect exactly-once semantics, tenant separation, production UI or blob HTTP endpoint. The Studio mock is unchanged.

The folder attachment follow-up registers the maintained Blob definition and exact trusted Service worker in the same registry/gate. Both database smoke paths now reserve, upload, verify and read real folder bytes; drain BlobService before HTTP/runtime shutdown; reopen the same database and folder without reuploading; deny another principal; then detach and reject subsequent reads. Serve creates a synthetic attachment once and drains the BlobService in its shutdown trigger before core shutdown. Both serve/SIGINT/reopen checks pass with the attachment present. No byte-specific HTTP endpoint or blob source code was added. Folder roots must remain exclusively host-owned. Database backup alone does not cover these external bytes. The full demo verifier, Clippy and docs passed again with the maintained blob adapters integrated. A fresh cached-database audit also passed across 268 resolved dependencies and 1280 advisories. README commands and fixtures remain usable without live accounts, credentials, mail services or network providers.
