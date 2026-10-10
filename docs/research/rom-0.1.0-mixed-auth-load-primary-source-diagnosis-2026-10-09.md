# ROM 0.1.0 mixed authentication load: primary-source diagnosis

Research date: 2026-10-09. This report combines official specifications, locked-library documentation, current ROM source, and retained failure evidence. This research author performed no runtime experiment, provider launch, or code change. The update below incorporates independently retained root-executed controls.

The [follow-up diagnosis](rom-0.1.0-auth-bind-expiry-follow-up-2026-10-09.md) records the subsequent failed trial and a second reproduced binding-expiry defect.
The initial correction and its results below remain historical evidence, not acceptance of the later trial.

## Finding and acceptance boundary

A stale-clock defect in `Credentials::actor` is now reproduced. The earlier implementation sampled time before an awaited credential mutex, then used that sample to decide whether to refresh a short-lived proof. A blocked request skipped refresh after the proof expired. The subsequent Runtime bind correctly rejected the expired proof. Authentication removes a session on `Denied`, so this defect can produce a persistent denial cascade without an expired provider token. Its contribution to the earlier mixed-load failure remains unproven.

A second candidate is intentional load shedding by the shared eight-permit authentication supervisor. Ordinary requests and stream identity checks share this supervisor. Rejection can terminate a stream. The actual run lacks admission-stage timestamps, so its `overloaded` counts cannot identify this boundary exclusively.

R10 remains incomplete. The required mixed result remains 13,280 successes, zero HTTP failures, zero SSE errors, and final 12,000 Done. A drained process or an unchanged provider configuration does not satisfy that result.

## Reproduced defect and narrow correction

Root control 74766 produced two passing controls and one failing race regression. The regression returned `Err(Denied)` where renewed proof validity `Some(160)` was expected. The [RED review](/root/ROM/.superpowers/rom-010-auth-expiry-red-execution-20261009/root-red-review.json) records physical exit 101, an absent wrapper, an empty current cgroup, and an exact source fence.

The production correction acquires the cached mutex before reading the clock. It changes neither proof TTL nor authentication capacity. Root control 11901 then passed the same three tests in 0.33 seconds. The [GREEN review](/root/ROM/.superpowers/rom-010-auth-expiry-green-execution-20261009/root-green-review.json) records physical exit 0, an absent wrapper, an empty current cgroup, and executed/current source equality. The corrected `oidc.rs` SHA256 is `778a5b0fb9fce83fb4789d5532fd7683aedc99ffb07683c74528ea067193c599`.

Root affected Host control 27726 subsequently passed 49 tests: 36 unit tests and integration suites of 2, 2, 3, 1, 1, and 4 tests. The five expiry regression and negative controls passed. They cover queued proof refresh, queued original-token expiry without key fetch or session extension, expired original credentials and revoked-link denial, transient refresh overload with retained session bounds, and original-lease expiry after a current bind fails. The [affected review](/root/ROM/.superpowers/rom-010-auth-expiry-affected-host-execution-20261009/root-affected-review.json) and [stdout](/root/ROM/.superpowers/rom-010-auth-expiry-affected-host-execution-20261009/affected-host.stdout) record physical exit 0, exact sources, an absent wrapper, and an empty current cgroup.

These controls establish the defect, the narrow correction, and the named failure controls within the affected Host scope. They do not establish the cause of the earlier trial, full-verifier acceptance, or successful mixed-load acceptance. The proposed matrix includes further controls beyond these executed cases.

## Retained observations

The SQLite run's [coordinator result](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-cbc1f723265355425ae8c111/result.json) has SHA256 `2e7b61b6dd5620d26166a85d526fa3662c3c0bbf305d06c0964241554d671eba`. The [independent bounded summary](/root/ROM/.superpowers/rom-010-r10-proxy-failure-diagnosis-20261009/failure-summary.json) preserves client, proxy, storage, and closure evidence.

The client recorded 12,487 traffic requests: 5,045 success, 6,201 denied, 1,240 overloaded, and one timeout. Three operator requests included one denied request. Four normal readers and one slow reader recorded four server-overloaded errors and four early EOF errors. Final Work contained 11,025 records, all Done. The client independently failed before the coordinator labelled the result `proxy-drain-failed`.

The proxy recorded one host upstream `EPIPE`. It retained `failed=true`, with `fatal_shutdown=false`. It recorded zero rejected requests and zero TLS errors. Its observed maximum inflight count was 13, below 48. Ingress and egress remained below their respective 64 MiB and 128 MiB bounds. Servers, sockets, and upstreams closed. Thus the status represents a retained failure verdict, not evidence that physical drain failed.

The client admitted a token window with 298,936 ms remaining against a 225,000 ms requirement. This does not establish that cached identity proofs remained valid. Aggregate storage counters recorded no failed calls, but do not attribute each HTTP failure. Current closure evidence records empty owned cgroups, absent recorded PIDs, vacant ports, unmount success, unchanged original seed, and source fences. The same-run complete provider API configuration was equal before and after; no provider database rollback is claimed.

## Primary contracts and current source

### Token expiry and proof expiry

OIDC requires the current time to precede the ID token's `exp`. Optional small clock leeway does not authorize accepting an expired token indefinitely. The specification does not require ROM's proof cache duration. [OIDC Core, ID Token and validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation)

ROM's OIDC adapter separately limits proof validity to `min(token exp, key-set expiry, now + 30 seconds)`: [adapter.rs](/root/ROM/crates/rom-auth/src/oidc/adapter.rs:104). Refreshing this proof revalidates the existing token and current key material. It is not a new login or OAuth refresh-token exchange. Activated identity binding also resolves current provider, link, and user state. [binding.rs](/root/ROM/crates/rom-identity/src/binding.rs:136)

The historical clock-before-lock sequence occurred in [oidc.rs](/root/ROM/crates/rom-studio-host/src/oidc.rs:131). Tokio 1.53.1 documents asynchronous mutex acquisition and FIFO ordering. FIFO does not bound waiting time or update previously sampled values. [Locked Tokio Mutex documentation](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html)

Runtime validates the actor during bounded I/O and again after awaiting that I/O. Rejection of an expired proof is a security fence to preserve. [authority.rs](/root/ROM/crates/rom/src/execution/authority.rs:95) Session removal on `Denied` or `Panicked`, but not `Overloaded`, makes the classification consequential. [session.rs](/root/ROM/crates/rom-studio-host/src/session.rs:109)

### Admission and streams

The Host uses `try_acquire_owned`, which returns immediately when permits are unavailable. Accepted tasks retain their permits until completion, including when their waiter is dropped. [lifecycle.rs](/root/ROM/crates/rom-studio-host/src/lifecycle.rs:31), [locked Tokio Semaphore source](https://docs.rs/tokio/1.53.1/src/tokio/sync/semaphore.rs.html)

The observation gate uses the same authentication supervisor. Its `current()` bind does not refresh credentials. The gate checks the original actor's expiry and reports `overloaded` as a terminal stream result. [observation_gate.rs](/root/ROM/crates/rom-studio-host/src/observation_gate.rs:15), [oidc.rs](/root/ROM/crates/rom-studio-host/src/oidc.rs:128) These are distinct possible failure paths. Renewing a request proof does not automatically extend an existing stream actor's lifetime.

Browser `EventSource` can reconnect after closure. That behavior does not extend authorization validity or change this workload's explicit zero-error oracle. [WHATWG server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html#server-sent-events) Node stream backpressure uses buffering thresholds; a high-water mark is not a memory quota. Byte, lifetime, and cancellation bounds remain necessary. [Node 22.16 stream buffering](https://nodejs.org/download/release/v22.16.0/docs/api/stream.html#buffering)

### Provider and transport

Authentik documents its per-application JWKS endpoint and warns that proxy challenges can block server-to-server endpoints. That supports inspecting concrete HTTP status and latency, not disabling TLS or security checks. [Authentik OAuth provider](https://docs.goauthentik.io/add-secure-apps/providers/oauth2/)

The current 2026.8 release describes per-IP throttling and JWKS optimizations. This is not proof those behaviors exist in the fixture's pinned image. [Authentik 2026.8 release](https://docs.goauthentik.io/releases/2026.8/) The fixture pins digest `09782fe56675bc616a0324468f1698e2d9d83c978bc5e426686fb7563517a442`. Its upstream release mapping was not established here. No default JWKS rate-limit cause is certified.

Node describes `EPIPE` as writing to a peer that has closed its reading side. It does not identify why the peer closed. [Node 22.16 system errors](https://nodejs.org/download/release/v22.16.0/docs/api/errors.html#common-system-errors) A client deadline, upstream cancellation, or connection lifecycle error remains possible. The one recorded EPIPE cannot explain thousands of denials by itself.

## Ranked, falsifiable experiment matrix

The first row now has root-executed RED, GREEN, and the named expiry negative controls described above. Provider/user revocation and every other proposed variation are not implied by those results. Other rows remain proposals. Each future execution requires separate admission.

| Rank / hypothesis | Exact discriminating control | Required result and negative controls |
| --- | --- | --- |
| 1. Reproduced stale clock skips proof refresh; earlier-trial attribution remains open | Use actual Credentials and the same fake Host/Runtime clock. Hold its cached mutex at time 129, with proof expiry 130 and token expiry 400. Poll `actor()` to Pending before advancing to 130. Release the mutex. | Old behavior must demonstrate expired-proof rejection. A correction must refresh, bind successfully, and retain the same session. Token expiry 130, revoked provider/link/user, and invalid JWKS must still fail closed. Expiry during later I/O must not be bypassed. |
| 2. Eight-permit Host authentication shedding | Hold eight accepted supervisor tasks with explicit barriers. Attempt a ninth request and stream check. Record tagged admit/reject/completion counts. | Confirm exact Host admission rejection and retained session. Release tasks and prove recovery. Sample Runtime status separately; do not label its capacity as the Host's rejection source. |
| 3. Stream actor expires or stream checks contend | Exercise actual observation gate at proof boundary with a normal and bounded slow reader. Separate expiry, cancellation, and admission rejection. | Preserve current revocation/expiry termination. A longer token must not silently extend a stream actor. Record terminal class, bytes, and physical drain without automatic client retries. |
| 4. JWKS acquisition delay or rejection | Supply controlled bounded 403, 429, timeout, valid-key, and invalid-key responses to the actual acquisition path. Bind the actual image release before testing provider-specific throttling. | Attribute acquisition result separately from core binding and Host admission. Known transient failures remain `Overloaded`; malformed or unauthorized identity remains denied. Preserve current authorization and TLS checks. |
| 5. HTTP abort or upstream lifecycle produces EPIPE | Use a seconds-scale isolated real proxy, stalled response, owned client deadline, and controlled peer close. Retain route class and monotonic stage times without secrets. | Preserve failure verdict and unrelated-request isolation. Security/budget violations remain fatal. Identify whether abort precedes upstream close; do not widen deadlines to hide the error. |

Instrument only bounded, payload-free stage records: Host admission, credential-lock wait, proof-refresh decision, JWKS result, Runtime bind result, session-removal reason, stream terminal reason, and HTTP abort/close. Do not log tokens, cookies, subjects, or provider response bodies. Use fixed operation tags and counters. Aggregate HTTP 503 counts are insufficient: the executor also classifies proxy 502 and other 5xx responses as overloaded.

First establish deterministic source controls. Then review the minimal change against revocation, expiry, cancellation, and ownership guarantees. Build a fresh source-bound binary if Rust changes. Only then repeat the unchanged mixed profile under separately admitted provider and workload leases. Do not raise authentication capacity, proof TTL, timeouts, or retry failed client operations before causal attribution.

## Evidence and limitations

Inspected source and retained-input hashes are recorded in [research evidence](/root/ROM/.superpowers/rom-010-auth-load-web-research-20261009/inspected-inputs.json). Official sources were retrieved on 2026-10-09. Tokio documentation matches locked version 1.53.1; Node documentation matches 22.16.0. Authentik documentation is current, not authenticated to the fixture image's release.

The run lacks first-denial timestamps, credential-lock intervals, admission-stage counts, and the exact EPIPE request identity. The stale-clock defect is reproduced independently, but none of these hypotheses is certified as the retained trial's measured cause. No mixed-load acceptance is inferred from source inspection, narrow regression results, or closure evidence. The inspected-input inventory preserves the earlier research source identities; it is not a claim that corrected production source retains those historical hashes.
