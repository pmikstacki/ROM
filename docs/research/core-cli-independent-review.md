# Independent core, discovery and CLI review

Reviewed 2026-10-02 against the owner-approved `harden-core-and-add-cli`
design, including the mutation-outcome clarification in `3964f9d`, and
[quality gates](../quality.md). No unresolved P1/P2 finding remains in the
reviewed implementation. This assessment covers deferred research, core
ergonomics/resilience, discovery and CLI. Studio remains excluded.

## Exact source and evidence

Final executable source: review commit
`66864f25b94b10914e80982d6ae06d2d8ea0a31a`, incorporating CLI commit
`1d7a640`. A direct comparison against integrated main `1d1d622` showed
identical `crates/`, `demo/`, `examples/`, `Cargo.toml` and `Cargo.lock`.
The review branch also contains the independent discovery budget regression
`01c2015`, which main incorporated separately.

Execution used native `rom-dev`, Rust `1.99.0 (b940084d7 2026-09-28)`,
`CARGO_TARGET_DIR=/var/tmp/rom-final-review-target` and
`CARGO_NET_OFFLINE=true`. Final lockfile SHA-256:
`35a128d90a9c31bc03f0e073cdba52e819bffb565bfb05cfd34a8261bf112e7d`.
Final independently built CLI binary SHA-256:
`4311b606d8452f3e1ea1148ac03d8ebee3d48ef1ff5128020051af18fba77f1a`.

The final review checkout had no tracked implementation changes. Four standalone
review probe sources were untracked. They are not Cargo inputs. Three were
compiled separately to exercise the actual CLI through TCP and process pipes.
The fourth inspected an earlier SSE concern already corrected before its run.

These commands were independently executed by the reviewer, rather than accepted
from implementation-agent reports:

| Source | Command in review checkout | Result |
|---|---|---|
| `a293e0f` | `cargo test --locked -p rom-consumer --test query_all --test capacity_configuration` | 4 passed |
| `a293e0f` | `cargo test --locked -p rom-http --test capacity_configuration` | 1 passed |
| `7ffacac` (discovery `b05cabf`) | `cargo test --locked -p rom-consumer --test discovery` | 11 passed |
| Discovery plus reviewer probe, preserved in `01c2015` | `cargo test --locked -p rom-consumer --test discovery reviewer_catalog_budget_matches_wire_for_every_smaller_limit` | 1 passed |
| `950ae40` (HTTP/demo `e858d7e`) | `cargo test --locked -p rom-http --test loopback discovery` | 2 passed |
| `950ae40` | `cargo test --locked -p rom-demo --test workshop local_cli_discovers` | 1 passed |
| `66864f2` | `./crates/rom-cli/verify` | Formatting, Clippy with warnings denied, and 26 tests passed |

The earlier core/discovery runs used lockfile SHA-256
`93f51f4bfa2a95d96becf960cb8e00ae32c5767c82c7d5ba8891397637681610`.
The final CLI verification log is retained inside `rom-dev` at
`/var/tmp/rom-independent-cli-review.log`.

## Findings reproduced and corrected

Three P2 issues were independently reproduced against the unfinished CLI and
corrected before the final commit:

1. A valid-looking response for another kind/ID made both a requested read and
   mutation exit successfully. Final independent TCP probes return 4 for the
   read and 5 for the mutation, with empty stdout. Query/live kinds and journal
   identities are also checked by the committed regression fixtures.
2. A journal request after generation `original`, position 100 accepted a
   response at generation `restored`, position 1. The final independent probe
   returns protocol failure 4 with empty stdout. Committed fixtures also check
   continuity across SSE batches, event order and cursor bounds. Explicit
   server-reported history gaps retain exit 6; legitimate filtered position
   advances remain allowed.
3. During a submitted mutation, Ctrl-C blocked while its diagnostic wrote to
   saturated stderr. The final independent probe, with 180224 bytes already
   filling the stderr socket, exits 130. The signal handler produces no output.
   Documentation explains that its exit status conveys uncertainty.

Reproduction sources remain in the isolated review checkout as
`review-cli-identity.rs`, `review-cli-cursor.rs`, and `review-cli-cancel.rs`.
Their final SHA-256 values, after selecting the independently built binary, are:

```text
4dd53503873e5d4e65556b1fb6c26eb1a894b285542e260563d15752005dee70  review-cli-identity.rs
52491244930e197929f5e003c588edfde5fa3a8b2c6699e59d7bd14bbc64822b  review-cli-cursor.rs
0218eb333c3c5ffec5c8a621fe35249c8179b09f2bda7a8d77c5011fc28f5e29  review-cli-cancel.rs
```

The coordinator separately identified that arbitrary ActorGate errors can occur
after commit. The reviewer inspected the final all-submitted-errors-to-5 rule
and independently executed its real post-commit regression. The additional
review comment about misleading `not_committed` wording was corrected: the
diagnostic labels it as a server category after an unresolved-outcome message,
without asserting rollback. This finding is attributed to the coordinator.

## Standards and specification assessment

Core all-query constructors reuse existing query authorization and bounds.
Semaphore maxima are rejected before construction. Discovery uses explicit,
default-denied metadata policy and current-authority checks. Row policies,
codecs, and actions do not run to infer metadata grants. Recursive hidden
references are omitted. Before output allocation, borrowed metadata is charged,
including the envelope and separators. The reviewer added exhaustive smaller-budget
checks for a multi-resource catalog. Callback panic, expiry and revocation tests
passed. HTTP preserves the ordinary authentication/body-limit path.

The CLI uses generic requests for unrelated Resources, strict bounded JSON,
explicit mutation identity, disabled automatic retry/redirect/proxy behavior,
escaped output and incremental bounded streaming. Final tests exercise actual
processes and TCP, both maintained database adapters, partial presence and exact
u64 values, lost acknowledgments, authority changes, malformed responses,
history continuity and process cancellation. No additional P1/P2 standards or
specification finding remains from source review or these focused executions.

All 37 entries in the [deferred roadmap](deferred-capabilities-roadmap.md) were
reviewed against the deferred inventory for options, recommendations,
dependencies, proposed acceptance probes and evidence limits. Primary-source
spot checks covered RabbitMQ confirmations/Direct Reply-To, Lapin/amqprs,
Wasmtime, Moka, OIDC/OAuth, SQLite erasure and PostgreSQL row security. The CLI
research additionally distinguishes parser selection from benchmarking and
correctly requires disabling reqwest's protocol retries. These are inventory
review and sampled source verification, not independent execution of every
proposed experiment or a claim that every external reference was revalidated.

## Completion boundary

The reviewed research and implementation satisfy their scoped acceptance at
the sources above. Final combined workspace verification, dependency audit,
extracted-package execution and publication remain coordinator-owned evidence
and must be recorded separately before claiming the overall goal complete.
The package verifier changes were inspected read-only: they run the CLI's help
from its extracted archive. The reviewer did not execute that packaging gate.

No production identity-provider compatibility, remote service certification,
cross-platform runtime support, representative throughput or human author study
is established by this review. Proposed deferred probes remain unexecuted unless
their own reports explicitly state otherwise. Discovery metadata remains
distinct from operation authority, and no exactly-once external effect or
automatic mutation recovery is claimed.
