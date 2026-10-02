# Core ergonomics and generic CLI Implementation Plan

> **For agentic workers:** use superpowers:subagent-driven-development or superpowers:executing-plans task by task; review independently before integration.

**Goal:** Deliver the owner's revised scope: complete deferred research plus a pleasant, resilient core and a real generic CLI.
**Architecture:** Resource definitions remain authoritative. Core provides fallible limits, typed all-query and opt-in metadata discovery; HTTP maps discovery; CLI consumes the actual generic wire protocol. Research stays evidence, not an implementation claim.
**Tech Stack:** Rust/Cargo 1.99, Tokio/Rayon, existing Axum/reqwest/serde; clap version selected from current primary sources and pinned.
**Spec:** openspec/changes/harden-core-and-add-cli/design.md and specs/*/spec.md.

## Global constraints
- Studio is excluded. No new storage format or bypass of current authority.
- Use isolated worktrees and task-owned persistent targets; main integrates reviewed commits.
- JSON preserves missing/null/false/zero and rejects nested duplicate keys.
- Requests <=64 KiB; finite responses/SSE frames <=2 MiB; no automatic mutation retry.
- CLI finite exits 0/2/3/4/5/6 and mutation interruption 130 follow design.
- Source distribution MIT; no GitHub Actions or external provider credentials.

## Review focus
- Partial metadata leaks through nested references: task 2 deny/omit tests.
- Extreme configuration panics: task 1 red/green boundary tests.
- Post-commit denied/timeout responses interpreted as rollback: task 4 real dropped reply and denial tests.
- Chunked SSE, multiline CRLF, output failure: task 4 byte-fragmented parsing and pipe closure tests.
- Data/credentials/control sequences in stderr or terminal output: task 3 input/output redaction tests.

## Task 1: Core authoring and configuration
Files: crates/rom/src/resource.rs, execution.rs; crates/rom-http/src/lib.rs;
examples/consumer/tests and crates/rom-http/tests.
Interfaces: Query<R>::all() -> Self, Default for Query<R>; existing Limits/build Result and Http::new Result.
- [ ] Reproduce MAX_PERMITS+1/usize::MAX panic with caught unwind on each concurrency field; test zero and MAX boundaries.
- [ ] Reject invalid semaphore counts before construction; keep existing error categories.
- [ ] Add Query::all/Default and verify equality with dynamic QuerySpec::all using actual reads/live/row+field policies/limits/keyset pages.
- [ ] Run focused tests and warning-free Clippy; commit.

## Task 2: Deliberate discovery
Files: crates/rom/src/discovery.rs, resource.rs, lib.rs; consumer discovery tests.
Interfaces: DiscoveryTarget<'a> {Resource,Field(&'a str),Action(&'a str)}, Definition<R>::discovery_policy(F), Runtime::discover(&Actor) async Result<Discovery>.
Types: serializable Discovery { version:u32, resources:Vec<DiscoveredResource> }; DiscoveredResource {kind:String,version:u32,fields:Vec<DiscoveredField>,actions:Vec<String>}; DiscoveredField {name:String,shape:Shape}; tagged snake_case Shape type/value.
- [ ] Write default-deny, selective metadata, nested-hidden-reference, current-authority, size exhaustion and no codec/row-scan/action tests.
- [ ] Implement erased trusted discovery predicate and bounded observe path with deterministic output; no capability=permission inference.
- [ ] Check callback panic, expiry/revocation, no partial success and hidden totals.
- [ ] Run focused tests, public docs/Clippy; commit.

## Task 3: Wire route and finite CLI
Files: crates/rom-http/src/lib.rs/tests, demo/src/lib.rs; crates/rom-cli/{Cargo.toml,LICENSE,README.md,src/{main,args,client,input,output}.rs,tests/commands.rs}; workspace manifest/lock.
Consumes: Discovery types/task2 and existing Invocation/QuerySpec/wire endpoints.
Produces: POST /discover accepts {}; executable rom commands exactly listed in design; public crate helper API only if necessary for testing.
- [ ] Route discovery through normal auth/body limits; authorize demo fields/actions deliberately and test hidden definitions.
- [ ] Freeze clap command/help/exit contracts in argument tests, using --auth-file and explicit mutation keys/revisions.
- [ ] Preserve strict duplicate-rejecting JSON, bounded stdin/files, safe URL/header validation, no redirects/proxies/retries.
- [ ] Implement finite commands and escaped human/JSON outputs; stderr static categories, no secret echo.
- [ ] Test two unrelated kinds through actual binary and TCP with SQLite/redb; commit.

## Task 4: Streams and failures
Files: crates/rom-cli/src/sse.rs/client.rs/output.rs; tests/{loopback,streaming}.rs; docs/cli.md and verifier.
Consumes: existing SSE wire events; outputs one complete JSON data frame per line or escaped human representation.
- [ ] Test split UTF8/CRLF/multiline/event boundaries, oversized/incomplete frames, explicit error events and no unbounded queue.
- [ ] Implement bounded live/journal observation without auto-reconnect; preserve batch/cursor, flush frames, terminate on stdout close.
- [ ] Real HTTP failure fixtures: commit+drop reply, same-key recovery/mismatch, post-commit denial, history gap, redirects, cancellation and credential redaction.
- [ ] Run complete CLI acceptance using actual process/TCP; docs illustrate setup with synthetic demo auth clearly labelled; commit.

## Task 5: Research, review and release
Files: docs/research/deferred-capabilities-roadmap.md plus current results; scripts/package checking if binary selection needs distinction; dependency inventory/notices, README/tasks.
- [ ] Index every deferred topic with primary evidence, alternatives, recommendation, dependencies, local experiment and decision class; explicitly label unrun probes.
- [ ] Independently review each code delta and research completeness; fix proven issues with regression cases.
- [ ] Verify combined local checks, CLI, MSRV, audit/notices and external package consumer/CLI execution.
- [ ] Update scope/status/results, commit and publish; complete revised goal only once all included tasks pass.
