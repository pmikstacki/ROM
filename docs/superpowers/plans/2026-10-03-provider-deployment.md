# Provider deployment implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track each step with its checkbox.

**Goal:** Complete release stage 4.3 with one actual-provider service profile, explicit local provisioning, approved secret files and CLI recovery on both native stores.

**Architecture:** An additive async resolver feeds the existing HTTP decoder. An opt-in reference host composes bounded authentication work with existing identity Resources and verification APIs.

**Tech Stack:** Rust 1.99, existing Tokio/HTTP/identity/auth crates, SQLite/redb, and a pinned development-only OAuth provider fixture.

**Spec:** [Provider deployment profile](../specs/2026-10-03-provider-deployment-design.md).

## Global constraints

- Core remains independent of HTTP clients, provider implementations, filesystems and secret stores.
- Preserve `Http::new` and the default demo. New host code is enabled by the `provider-profile` demo feature.
- Keep implementation out of `lib.rs` and `mod.rs`. Use named cohesive modules and stable exports.
- Do not rewrite provider responses or weaken the existing strict verifier profile.
- Host authentication defaults for this example are four admitted jobs and a two-second caller deadline, with no unbounded queue or automatic retry.
- Credential bytes never become Resources, diagnostics, evidence logs or process arguments. Opaque approved references may be configuration.
- Actual-provider compatibility is a prerequisite for the deployment claim. Failure fixtures are not substitutes for that evidence.
- Use the existing `rom-dev` container, two build jobs and debug information disabled. Preserve active worktrees and builds.
- Coordinator owns integration and final combined verification. Agents must not commit or change shared files without coordination.

## Review focus

1. A disconnected caller must not release capacity while blocking verification runs; task 2 tests cancellation, deadline and drain.
2. An authorized provider configuration change must not permit arbitrary file/network acquisition; task 2 tests approved reference and endpoint matching.
3. A provider revision can change between verification and binding; tasks 2 and 4 require stale activation rejection.
4. A provisioning restart can replay only part of a three-Resource sequence; task 3 tests interruption and changed input under the original identities.
5. The disposable provider loses opaque tokens at restart; task 4 distinguishes ROM reopen, provider restart and explicit credential rotation.

## File boundaries

| Scope | Files | Responsibility |
| --- | --- | --- |
| HTTP seam | `crates/rom-http/src/authentication.rs`, `server.rs`, `request.rs`, facade exports | One sync/async resolver path and existing generic routes |
| HTTP tests | `crates/rom-http/tests/loopback/authentication.rs`, parent declaration | Compatibility, await behavior, admission and shutdown |
| Host configuration | `demo/src/provider_profile/{model,secrets,verification}.rs` | Approved inputs, private bounded file reads, strict activation-bound verification |
| Host lifecycle | `demo/src/provider_profile/{authentication,lifecycle}.rs` | Bounded owned jobs, cancellation and drain |
| Application setup | `demo/src/provider_profile/{application,provisioning,serving,command}.rs` | Identity policies, explicit local provisioning and mode dispatch |
| Public surface | `demo/src/provider_profile.rs`, `demo/src/lib.rs`, `demo/Cargo.toml`, minimal `main.rs` dispatch | Opt-in facade and feature selection |
| Integration | `demo/tests/provider_profile.rs` and named children | Both native stores, HTTP/CLI, restart, rotation and redaction |
| Provider fixture | `demo/provider-fixture/`, `demo/verify-provider` | Pinned actual provider, private runtime inputs and finite acceptance command |
| Documentation | `docs/provider-deployment.md`, results and release OpenSpec | Reproducible procedure, evidence and support limits |

Do not create files just to meet this inventory. Combine responsibilities that have one interface and change together; separate unrelated implementation and tests.

## Task 1: Add the async HTTP seam

**Consumes:** Existing synchronous `AuthResolver`, `Limits`, request decoder and all routes.
**Produces:** `AuthFuture`, `AsyncAuthResolver` and `Http::new_async(Runtime, AsyncAuthResolver, Limits) -> rom::Result<Http>` exactly as specified.

- [ ] Add failing TCP tests: delayed async auth permits unrelated requests to progress; denied auth does not consume a streamed body; admission rejects an extra pending request.
- [ ] Add a constructor compatibility case that runs the same ordinary Resource request through sync and async authentication.
- [ ] Test that shutdown closes new admission and a request awaiting auth cannot bypass Runtime closure afterward.
- [ ] Record missing-API or behavioral RED accurately; do not count a compilation failure as a reproduced runtime defect.
- [ ] Implement one private resolver enum in `authentication.rs`; split owned request parts once and await the chosen resolver in the shared decoder.
- [ ] Reuse validation of `Limits` and existing error/body handling. Add no provider-specific route or credential cache.
- [ ] Document host-owned deadlines and cancellation supervision on the public async resolver.
- [ ] Run `cargo test -p rom-http` and strict all-target Clippy; obtain independent source review before dependent acceptance.

## Task 2: Approved secrets and supervised authentication

**Consumes:** Task 1 resolver, `ProviderActivation`, `ActivatedIdentity`, `IntrospectionAdapter` and current `IdentityGate`.
**Produces:** Public opt-in host configuration and lifecycle through these names:

```rust
pub struct AuthLimits { pub jobs: usize, pub response_timeout: Duration }
pub struct ApprovedProvider {
    pub authority: String,
    pub issuer: String,
    pub audience: String,
    pub endpoint: String,
    pub introspection_client: String,
}
pub struct SecretFiles { /* private approved reference map */ }
impl SecretFiles {
    pub fn new(entries: BTreeMap<String, PathBuf>) -> rom::Result<Self>;
}
pub struct HostAuth { /* cloneable owned lifecycle */ }
impl HostAuth {
    pub fn new(runtime: Runtime, reader: Actor, clock: Arc<dyn Clock>,
        provider: ApprovedProvider, secrets: SecretFiles,
        policy: EndpointPolicy, limits: AuthLimits) -> rom::Result<Self>;
    pub fn resolver(&self) -> rom_http::AsyncAuthResolver;
    pub fn close(&self);
    pub async fn drain(&self) -> rom::Result<()>;
}
```

These types are host inputs, not remotely decoded credentials or proof constructors. Record any necessary signature correction before dependent code starts.

- [ ] Add secret file tests for allowed reference, unknown reference, empty/oversized/non-UTF8 material, symlink, non-regular file and non-private Linux permissions.
- [ ] Bound secrets to the existing verifier's 4,096-byte maximum. Require a trusted host directory; check file metadata and reject unsafe acquisition.
- [ ] On Linux, use `OpenOptionsExt::custom_flags` with `libc::O_NOFOLLOW | libc::O_NONBLOCK`, then validate the opened regular-file handle and private permissions. Add a bounded FIFO test with no writer; use the existing locked libc version as an optional dependency, with no unsafe block.
- [ ] Add authentication tests for duplicate/malformed/oversized Authorization, wrong approved endpoint/profile/reference and static redacted failures.
- [ ] Add deterministic paused-verifier tests: caller cancellation and deadline retain capacity, overload contacts no provider, close denies new work, drain waits for the accepted worker.
- [ ] Cancel a drain waiter and start another; it must still wait for the owned worker. Pass the same `Arc<dyn Clock>` into Runtime and host construction.
- [ ] Add a provider-revision race between verification and binding. It must fail without publishing a stale Actor.
- [ ] Record RED, then implement owned admission and completion tracking. Do not build another executor or leave unbounded completed task handles.
- [ ] Read activation before blocking work. Construct, use and drop the blocking verifier inside the worker, then bind asynchronously through Runtime.
- [ ] Match authority/issuer/audience/endpoint and service profile to approved host settings before external acquisition. Resolve only an approved versioned secret reference.
- [ ] Add `provider-profile` feature and the exact existing `rom-auth` package with `introspection`; preserve normal demo dependency behavior.
- [ ] Run feature-enabled tests, strict Clippy and default-feature checks. Review file safety, admission races, permit lifetime and secret redaction independently.

## Task 3: Explicit provisioning and serving

**Consumes:** Task 2 `HostAuth`, existing User/provider/link definitions and the shared reference business Resource.
**Produces:** Opt-in profile construction plus host-local commands, never new public provisioning routes.

- [ ] Define `Provisioning` with the provider Resource ID/value, local User ID and trusted service subject. Use `link_key` for its canonical IdentityLink.
- [ ] Add `provision(&Runtime, &Provisioning) -> rom::Result<()>`, using stable step identities and ordinary Resource creates.
- [ ] Test interruption after each committed step, both-store reopen and exact resume. Assert unchanged accepted revisions/events and changed-input rejection.
- [ ] Register provisioning and serving through one shared declaration helper with an explicit local mode. Serving excludes the provisioner allowance.
- [ ] Give the configuration reader only required provider reads. Grant linked service actors only the reference operations declared by this profile.
- [ ] Add a distinct embedded host-local maintainer for identity Resource types. Its control API validates the three configured keys before a revision-checked invocation. Network authentication never returns that actor.
- [ ] Test missing/forged host stamps, unlinked identities and ordinary authenticated operator denial. Keep default synthetic demo behavior unchanged.
- [ ] Add thin `provider-provision` and `provider-serve` dispatch with backend, database and bounded host configuration file inputs.
- [ ] Add an explicit offline `provider-maintain` mode for revision-checked identity Resource operations under native ownership. Reuse the same narrow local authority in in-flight host tests.
- [ ] Read host configuration without logging its values. Credentials remain in private referenced files, not flags or process arguments.
- [ ] In the profile stop future, close and drain host auth while Runtime remains open, then let `Http::serve` close and drain Runtime. Serving never provisions automatically.
- [ ] Run both-store lifecycle tests and default demo regression checks; obtain independent review.

## Task 4: Actual provider and CLI acceptance

**Consumes:** Reviewed tasks 1–3 and the successful pinned-provider compatibility experiment.
**Produces:** One finite `./demo/verify-provider` command and retained source/version evidence.

- [ ] Promote only the reviewed fixture configuration into `demo/provider-fixture`, with exact package/lockfile and license/audit evidence. Keep secrets and node_modules untracked.
- [ ] Generate private credentials and an isolated loopback provider at runtime; bound readiness, requests, child waits and teardown.
- [ ] Obtain actual client-credentials tokens through the provider's normal endpoint. Do not fabricate or modify introspection responses.
- [ ] Use actual `rom` CLI over TCP for successful declared Resource operations and denial of unlinked, expired, wrong-audience and forbidden operator requests.
- [ ] Run the same host journey on SQLite and redb, including provisioning resume and ROM reopen while the provider remains running.
- [ ] Rotate a versioned credential reference with the provider's supported replacement procedure. Test stale activation, failed preparation, new credentials and reopen.
- [ ] Test provider restart separately: old opaque tokens are unavailable; fresh credentials must be acquired explicitly.
- [ ] Test disablement and relinking against existing Actors, in-flight verification and ordinary receipt replay. Re-enable must not revive an old stamp.
- [ ] Search captured outputs, Resource state, journal, receipts, work views and archives for unique credential markers without printing those markers.
- [ ] Record exact versions, locks, source hashes, commands and sanitized outcomes. Distinguish fixture mechanism tests from actual provider checks.
- [ ] Obtain independent integration review; no claim of hosted-provider, human-login or TLS certification.

## Task 5: Guide, compatibility and final gate

- [ ] Write a guide for local provisioning, external token acquisition, private auth files, start/stop, credential rotation and recovery.
- [ ] State explicit host authority, exact supported provider/profile, proof windows, Linux file assumptions and separate-commit provisioning limits.
- [ ] Keep npm provider dependencies development-only and their audit distinct from Rust dependency checks.
- [ ] Run the full local verifier, default demo verification, feature-enabled profile checks and actual-provider acceptance after all source changes.
- [ ] Check feature-disabled core/HTTP dependencies and compile a public consumer of the added API.
- [ ] Record requirement-by-requirement results and update release task 4.3 only when evidence supports completion.
- [ ] Coordinator integrates the reviewed slice. Stages 4.4–4.6 remain open until their own checks pass.

## Execution status

Design and plan are under review. Provider probe results are being packaged separately.
No product implementation task in this plan is complete.
