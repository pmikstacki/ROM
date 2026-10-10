# ROM 0.1.0 authentication load: follow-up diagnosis

Research date: 2026-10-09. Official sources were checked online on this date.
This report supplements the [initial diagnosis](rom-0.1.0-mixed-auth-load-primary-source-diagnosis-2026-10-09.md).
The complete release goal remains active. The latest mixed trial failed.

## What the latest execution establishes

The first correction reads time after acquiring the credential mutex.
Its focused controls and affected Host tests passed. The subsequent mixed SQLite trial still failed.

The trial recorded 12,170 HTTP requests: 1,684 successes, 9,181 denials, and 1,305 overloaded responses.
It recorded no request timeout or unknown outcome.
Four normal readers and one slow reader recorded four server-overloaded errors and four early EOF errors.
This trial recorded no `identity_expired` terminal event and no proxy EPIPE.

The retained [client result](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-141f260d66b35ce8d9981e2a/mixed-0a17f432176245afc49ba33a/result.json) contains these observations.
The [root failure summary](/root/ROM/.superpowers/rom-010-mixed-post-auth-fix-preparation-20261009/root-failure-summary.json) separates workload failure from successful process closure.
The [terminal review](/root/ROM/.superpowers/rom-010-mixed-post-auth-fix-preparation-20261009/root-runtime-and-provider-terminal-review.json) records provider closure and unchanged configuration.
No redb mixed trial was admitted after this failure.

These results refute a claim that the first correction was sufficient.
They do not identify the first denial's exact stage.
The absence of EPIPE in this trial prevents using the earlier EPIPE as its explanation.

## Second reproduced defect

`Credentials::actor` can bind a cached proof that expires during awaited authoritative storage access.
Runtime correctly rejects the expired Actor. Host authentication then removes the session on `Denied`.
This can invalidate a session whose original signed token is still valid.

The deterministic control uses actual Credentials, `authentication::resolve`, Runtime, and native SQLite storage.
A one-shot barrier holds the User read. The shared clock advances from 129 to the proof's exclusive expiry, 130.
The original token expires at 400. The expected renewed Actor expires at 160.
Actual execution returned `Err(Denied)` instead.

The [RED review](/root/ROM/.superpowers/rom-010-auth-bind-expiry-red-execution-20261009/root-red-review.json) records physical exit 101: eight controls passed and one failed.
It also records exact source equality, an absent wrapper, and an empty current cgroup.
The unchanged-clock, original-token expiry during binding, and revoked-link controls passed.
This establishes a source defect. Its contribution to the mixed trial remains unproven.

## Primary sources and their implications

| Source | Established contract | Implication for ROM |
| --- | --- | --- |
| [OIDC Core Errata Set 2, ID Token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation) | Authentication must validate signature, issuer, audience, nonce, and original token expiry. | Proof renewal must reverify the signed token. It must not extend acceptance beyond original `exp`. |
| [Tokio 1.53.1 Mutex](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html) | Lock acquisition is asynchronous and FIFO. | Ordering does not bound waiting time. A sampled timestamp can become stale before or during later awaited work. |
| [Tokio 1.53.1 Semaphore source](https://docs.rs/tokio/1.53.1/src/tokio/sync/semaphore.rs.html) | `try_acquire_owned` rejects when permits are unavailable. | Tag Host admission separately from provider acquisition and Runtime binding. Aggregate 5xx counts cannot identify that boundary. |
| [WHATWG server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html#server-sent-events) | EventSource distinguishes reconnecting from terminal failure and carries event IDs when reconnecting. | Transport recovery does not renew authority. ROM must revalidate identity and respect its own cursor contract. |
| [Authentik OAuth provider](https://docs.goauthentik.io/add-secure-apps/providers/oauth2/) | JWKS is a server-to-server endpoint. Proxy challenges can prevent validation. | Inspect endpoint status and latency. Do not infer throttling from an aggregate denial count or disable security globally. |
| [Node.js 22.16 errors](https://nodejs.org/download/release/v22.16.0/docs/api/errors.html#common-system-errors) | EPIPE identifies a write after the peer's reading side has closed. | It does not identify why the peer closed. Investigate HTTP lifecycle separately from authentication. |

Tokio references match the locked library version. Current Authentik documentation does not establish the fixture image's release-specific behavior.
The standard does not specify ROM's 30-second proof cache. That is ROM's implementation policy.

## Correction and falsifiable checks

Use one bounded renewal when a cached-proof bind crosses its known expiry while the original token remains valid.
The renewal must perform actual signature verification and bind current provider, IdentityLink, and User state.
Never return the old expired Actor. Never retry indefinitely.
Genuine revocation, invalid signature, and original-token expiry must remain denied.

Run the same nine controls before broadening verification.
Preserve the failing baseline and negative controls. Then run affected Host tests and the full verifier.
A fresh mixed trial requires a newly built binary with the corrected source identity.
The unchanged acceptance target remains 13,280 successes, zero HTTP failures, zero SSE errors, and 12,000 Done records.

Before another expensive trial, identify where denials and overload originate.
Use bounded counters and a fixed-size first-failure record for admission, renewal, binding, session removal, and stream termination.
Do not record tokens, cookies, subjects, raw credential bodies, or unbounded error payloads.

Test stream authority expiry separately from the request-renewal defect.
The current gate retains its captured Actor expiry; later request renewal does not replace that Actor.
Studio's recovery controller and the raw load reader have different behavior.
This distinction requires explicit tests, not a silent change to the mixed trial's oracle.

## Executed correction controls

The correction performs one fresh verification and current bind when the cached proof expires during binding.
It also renews when a successful bind is followed by an explicit Host sample at the proof's exclusive expiry.
The verified proof and its deadline are published together only after successful binding.
Final fresh-bind denial remains denied. No TTL, authentication capacity, or mixed acceptance threshold changed.

Root executed ten controls after the correction. All ten passed in 1.11 seconds.
The [GREEN review](/root/ROM/.superpowers/rom-010-auth-bind-expiry-green-v4-execution-20261009/root-green-review.json) records physical exit 0 and exact 3,701-entry source equality.
The wrapper was absent and its current cgroup was empty.
The [stdout](/root/ROM/.superpowers/rom-010-auth-bind-expiry-green-v4-execution-20261009/bind-expiry-green.stdout) names each executed control.

An [independent source review](/root/ROM/.superpowers/rom-010-auth-bind-expiry-production-review-20261009/report.md) checked bounded renewal and genuine denial behavior.
It identified the final Host sample case before the additional regression was executed.
The ten controls do not establish all provider, User, cancellation, or overlapping-revocation scenarios.

Two earlier GREEN preflights timed out during a five-second owned-target allocation scan. Neither launched Cargo.
The successful execution used a separately recorded thirty-second ceiling for that measurement only.
Application deadlines and acceptance thresholds remained unchanged.

The subsequent affected Host run passed 54 tests. The full `./scripts/check` also returned physical exit 0.
The [root terminal review](/root/ROM/.superpowers/rom-010-auth-bind-expiry-verifier-execution-20261009/root-terminal-review.json) records exact source equality and empty current cgroups for both stages.
The verified production `oidc.rs` SHA256 is `17b7f048aab3b9108d1ea0b5e990e807c634ec827e73dc1bd4e7bb3c519401f8`.
This report's result update follows that verification snapshot. It does not change the recorded snapshot or establish mixed-load acceptance.

## Limits

The second correction passed focused, affected Host, and full-verifier checks against the recorded source.
Neither a reproduced source defect nor official documentation proves the cause of all 9,181 denials.
Release acceptance still requires successful matching-artifact load, identity lifecycle, restore, upgrade, and consumer evidence.
