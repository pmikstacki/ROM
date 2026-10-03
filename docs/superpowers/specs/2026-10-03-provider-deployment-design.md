# Provider deployment profile design

Status: stage 4.3 design. Operator recovery is integrated in `58f86b2`.
This document specifies the next release slice; it does not claim that its implementation or acceptance tests are complete.

## Intent and scope

Provide one reproducible deployment profile that connects an actual OAuth provider to ROM's existing identity contracts.
A service token must reach an ordinary authorized Resource operation through the actual CLI and HTTP adapter.
Bootstrap, current identity checks, versioned secret references and restart recovery must work on SQLite and redb.

The user's established constraints remain binding: one Resource model, provider-independent core, optional transports and pleasant application APIs.
User, IdentityProvider and IdentityLink remain Resources. Credential bytes and verified identity evidence are host inputs, not new domain entities.
The default demo and its synthetic identity policy remain unchanged.
Studio, interactive human login, automatic account recovery, shared tenancy and universal provider compatibility are outside this slice.

The release goal already authorizes sequential implementation and parallel reviews.
This is an architectural integration task. Product-specific roles and external deployment credentials are not inferred from that authorization.

## Selected boundaries and alternatives

Use a small additive asynchronous authentication seam in `rom-http` and one opt-in host profile in the reference application.
The host composes existing `rom-auth`, `ProviderActivation`, `ActivatedIdentity` and `IdentityGate` APIs.
Core gains no HTTP client, provider SDK, filesystem or secret-store dependency.

| Option | Decision and reason |
| --- | --- |
| Async resolver through the existing decoder | Select. It supports async configuration reads and binding while preserving all generic routes and the synchronous constructor. |
| New authenticator trait hierarchy | Defer. One future and closure boundary provides the needed contract without another author-facing abstraction. |
| Middleware plus private Actor headers or token-to-Actor lookup | Reject. A hidden credential transport and second trust path add unnecessary state and composition rules. |
| Blocking introspection inside the current synchronous resolver | Reject. It blocks request workers and cannot await activation or binding safely. |

Application authors must not write a handler for each Resource or operation.
The opt-in host accepts a Runtime, a provider Resource ID, approved host settings and secret references, and explicit authentication limits.
Provider fixture details remain in test support, separate from that host API.

## Actual provider compatibility gate

Use one disposable instance of the maintained `oidc-provider` OAuth server library, subject to an exact pinned-version compatibility probe.
The initial `9.5.1` candidate failed dependency audit and is not a release dependency selection.
The next candidate is `9.12.2`; the final lockfile, audit, license inventory and unmodified response acceptance must pass before integration.
The [selection research](../../research/real-provider-profile-selection.md) records official sources and alternatives.

The fixture enables real client-credentials issuance and authenticated introspection.
Separate issuing and introspection clients use independently generated private credentials.
Supported issuance configuration supplies `sub=client_id` and `principal_kind=service` for the issuing client.
Resource audience, issuer and token type must satisfy the current strict service profile.
No proxy, response rewrite or weakened verifier may repair incompatible provider output.

The initial fixture uses numeric loopback HTTP with `EndpointPolicy::LoopbackTestOnly`.
The host defaults to `HttpsOnly`; loopback acceptance is an explicit test mode, not a TLS deployment claim.
The provider's memory storage is disposable. Provider restart invalidates its opaque tokens; ROM restart recovery is tested separately while the provider stays running.
No production provider service is installed or changed by this acceptance harness.

## HTTP authentication seam

Add these public aliases and constructor without changing `AuthResolver` or `Http::new`:

```rust
pub type AuthFuture = Pin<Box<dyn Future<Output = rom::Result<Actor>> + Send>>;
pub type AsyncAuthResolver = Arc<dyn Fn(HeaderMap) -> AuthFuture + Send + Sync>;

impl Http {
    pub fn new_async(runtime: Runtime, auth: AsyncAuthResolver, limits: Limits)
        -> rom::Result<Self>;
}
```

An internal resolver representation selects synchronous or asynchronous invocation through one decoder.
Split the request into headers and body. Pass the original owned headers into the async resolver; do not clone credentials unnecessarily.
Keep HTTP body admission before authentication, then read the body only after authentication succeeds.
The existing body limits, JSON rules, error mapping and route semantics remain shared.

The host is responsible for finite authentication deadlines, bounded admission and cancellation supervision.
An async callback alone supplies none of those guarantees.
No request field or arbitrary header can construct an Actor or choose a trusted host stamp.

## Host-owned authentication

The reference profile starts with four admitted authentication jobs and a two-second caller deadline.
These are explicit fixture settings, not universal capacity recommendations.
Validate nonzero capacity, semaphore capacity bounds and representable timer values at construction.

The flow is:

1. Reject closed intake and malformed, duplicate or oversized Authorization values.
2. Try to acquire a host authentication permit. Reject overload without queuing or contacting the provider.
3. Start a host-owned job with that permit. The caller waits for its bounded result.
4. Read `ProviderActivation` using the narrowly authorized configuration-reader actor.
5. In blocking work, validate host-approved settings, resolve the approved credential reference, construct the verifier and obtain activation-bound evidence.
6. Create, use and drop the blocking HTTP verifier inside that worker.
7. Bind the resulting `ActivatedIdentity` through Runtime. Reject changed provider, link or User state.
8. Return only the bound Actor. Release admission after the job and blocking worker finish.

Cancellation or caller timeout cannot release capacity while verification still runs.
The job retains its Runtime ownership until it finishes. Closing admission and draining are separate, repeatable host operations.
Canceling a drain waiter must not discard job tracking; a later waiter can still await completion.
The profile's stop future first closes host authentication, then drains it while Runtime remains open.
Only then does it return to `Http::serve`, which closes HTTP intake and drains Runtime work before store release.
No mutex, native transaction or core gate spans provider I/O or async Runtime calls.
Trusted callbacks must yield and blocking operations must have finite bounds. The host cannot preempt arbitrary blocking Rust code.

Pass the same `Arc<dyn Clock>` to the host and `Builder::clock`. Runtime has no public clock getter.
A caller cannot supply verification time.
Construct a verifier per admitted request for the first profile; no global Actor cache or verifier publication cache is introduced.
This trades connection construction and a provider request for a smaller rotation and ownership contract.
Any later cache must be measured and preserve captured provider revision and finite proof expiry.

Invalid evidence, forbidden endpoints and secret failures deny access with static errors.
Overload and caller deadline return `Overloaded`; closed intake returns `Closed`.
Logs and HTTP errors must not include token bytes, secrets, raw provider bodies or credential paths.

## Secrets and host approval

`IdentityProvider.credential_ref` identifies an entry in a host-approved map of protected local files.
It cannot select an arbitrary path, URL or secret resolver namespace.
The host also approves provider authority, issuer, audience, endpoint and introspection client ID.
An authorized Resource mutation alone does not approve arbitrary external acquisition.

Read a secret through a bounded regular-file operation. Reject missing, empty, oversized, malformed or unsafe file material before verifier construction.
For the supported Linux profile, open with no-follow and nonblocking flags, then validate the opened handle's type and private permissions before reading.
Reject symlinks, directories, FIFOs and other non-regular files without waiting for another process to open them.
Use a trusted local parent directory and immutable versioned files. Concurrent privileged file modification is outside this profile's contract.
The existing locked `libc` package can supply Linux flag constants through an optional demo dependency; no unsafe Rust block is needed.
Do not put credentials in Resource state, receipts, journal, archives, process arguments or evidence logs.
This scope does not claim memory zeroization or disk encryption from ordinary string/file ownership.

Use immutable versioned references. Install approved replacement material, perform the provider's documented credential change, then update the provider Resource reference.
The actual provider fixture must establish a working rotation order; overlapping secret support is not assumed.
The Resource revision invalidates old activation and Actor stamps through the existing identity gate.
An in-flight old activation cannot bind after the revision changes.
If replacement preparation fails, authentication fails closed. Accepted configuration is not proof of a successful external activation.

After restart, rebuild verification from current Resource state and the approved files.
External token revocation remains bounded by the existing proof validity window; it is not promised to be instantaneous.
Replacing file bytes behind an unchanged reference is outside the versioned rotation procedure.

## Explicit provisioning

Provide a host-local provisioning command before public intake.
Its Runtime permits an exact embedded provisioner through explicit application policies and `IdentityGate::allow_host`.
It creates the provider, User and canonical IdentityLink through ordinary Runtime commands with stable exact identities.
The subject comes from the selected trusted provider configuration or verified provider evidence; never infer it from email.

The three creates are separate commits. Interrupted provisioning resumes by repeating the original requests.
Changed input under an accepted identity fails. Do not present the sequence as an atomic multi-Resource transaction.
Serving construction excludes the provisioner allowance. It exposes no first-caller enrollment endpoint or synthetic fallback credential.

Application policy grants only the reference Resource operations required for acceptance.
Authentication and linking do not grant work inspection or operator controls.
The configuration-reader host identity can read approved provider settings but is unavailable to network callers.
A distinct embedded host-local maintainer has explicit authority for the three identity Resource types.
The local control API checks the configured provider, User and canonical link keys before invoking a revision-checked operation.
Row policies alone do not enforce that key restriction; their current callback receives the value, not its ID.
It cannot create arbitrary application records or become a network identity. The token resolver never returns an embedded host actor.
Normal CLI maintenance stops serving and acquires the same native ownership guard before applying an explicit revision-checked Resource operation.
Trusted embedding code can use the same narrow local authority for live rotation or revocation; tests use it to exercise in-flight races.
This is a reference application policy, not a framework-wide administrator role or new authentication backdoor.

## Acceptance and evidence

| Contract | Required acceptance |
| --- | --- |
| Actual provider | Pinned server issues and introspects real credentials; unchanged ROM verifier accepts the configured service profile. |
| Transport compatibility | Existing sync routes remain valid; async auth uses the same body admission and generic route path. |
| Public flow | Real provider credential → activation → binding → CLI/TCP Resource operation works on both native stores. |
| Provisioning | Interrupt after a committed step, reopen and resume exact requests without duplicate state/events; public serving cannot provision. |
| Current identity | Provider, link or User changes deny stale in-flight binding and established access. Re-enable does not revive old stamps. |
| Rotation and failures | Versioned reference changes, missing or unsafe files, failed preparation and restart retain fail-closed behavior. |
| Resilience | Cancellation, bounded overload, deadline, provider outage and shutdown retain admitted job ownership until completion. |
| Disclosure | Marker credentials stay absent from outputs, errors, Resource state, journal, receipts, work views and archives. |
| Operator separation | A valid ordinary linked actor cannot inspect or control work without an explicit operator policy. |
| Packaging | Fixture inputs, lockfile, provider version, commands and sanitized results are retained; final packaged-consumer acceptance includes this profile. |

Existing cryptographic tests remain the source for low-level signature and claim rules.
New tests focus on real-provider interoperability, host composition and operational recovery.
Synthetic response servers remain useful for bounded failure injection but cannot close the actual-provider requirement.

## Completion boundary

Stage 4.3 closes after the profile, its application journey, focused fault tests, independent review and combined local verification pass.
The final release still requires extension conformance, executable author skills, package checks and a completion audit.
This profile establishes one configured service integration. It does not certify human SSO, hosted tenants, provider availability or every OAuth implementation.
