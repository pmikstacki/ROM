# Mixed observation lease investigation

Investigated on 2026-10-09. Source review, primary-source research, and the actual public Host experiment have separate evidence scopes.

## Contract and test mismatch

The original mixed reader cannot satisfy its zero-SSE-error oracle while the Host correctly terminates an expired observation.
The final scheduled group arrives at 119,990 milliseconds. Streams open before scheduling starts.
An OIDC Actor lasts at most 30 seconds after verification. The raw reader never reacquires its observation.

The Host preserves each stream's captured Actor. At expiry, it emits `identity_expired` and ends the stream.
The reader counts that terminal frame and the subsequent EOF as separate errors.
An otherwise correct expiry therefore fails the original oracle.

| Boundary | Source |
| --- | --- |
| Last scheduled arrival | [Workload plan](../../tests/application-load/workload-plan.mjs), [scheduler](../../tests/application-load/scheduler.mjs) |
| Stream acquisition and verdict | [Mixed runner](../../tests/application-load/mixed-run.mjs), [raw stream reader](../../tests/application-load/streams.mjs) |
| Finite verified identity | [OIDC adapter](../../crates/rom-auth/src/oidc/adapter.rs) |
| Captured authority and terminal frame | [Observation gate](../../crates/rom-studio-host/src/observation_gate.rs), [session observation](../../crates/rom-studio-host/src/session_observation.rs) |

The mismatch does not explain the first HTTP denial or overload. Those failures still require stage attribution.
It does not change any historical trial's failed verdict.

## Primary-source findings

WHATWG EventSource defines reconnection and event parsing. The current reader uses POST `fetch`, rather than EventSource.
It receives no automatic EventSource reconnection. A server-defined `error` event is distinct from the browser's transport-error algorithm.
See [processing rules](https://html.spec.whatwg.org/multipage/server-sent-events.html#processing-model) and [event parsing](https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation).

OIDC token validation does not specify ROM's 30-second proof lifetime. A longer original token does not extend captured stream authority.
New acquisition must retain signature, issuer, audience, nonce, original-expiry, and current identity checks.
See [OIDC ID Token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

## Actual Host evidence

The public Host experiment passed two tests containing eight scenarios across SQLite and redb.
It verified same-session renewal, original-stream expiry, fresh-stream data, and rejection after link/provider revocation or original-token expiry.
The original session expiry remained unchanged. Owned processes closed and the captured sources remained exact.

The [terminal review](/root/ROM/.superpowers/rom-010-observation-expiry-test-execution-20261009/root-terminal-review.json) identifies the commands, hashes, and result scope.
This experiment does not establish automatic recovery in installed Studio, native SQLite 3.53.4 acceptance, or mixed-load acceptance.

## Reviewed acceptance correction

Use a separately versioned lease-recovery workload. Preserve the original reader and every failed result.
Expected expiry must remain visible; ignoring its category alone cannot establish recovery.

Require all of these results:

- Preserve the 12,000 scheduled groups, 13,280 successful workload HTTP requests, and final 12,000 Done records.
- Preserve four normal readers and one actual slow reader.
- Record expected expiry, terminal EOF, reacquisition attempts, fresh snapshots, recovery latency, and observation gaps separately.
- Require bounded successful authorized reacquisition after each expected expiry.
- Reject unexplained EOF, malformed frames, denial, overload, transport failure, exhausted recovery, and post-expiry disclosure.
- Count additional acquisition and revalidation traffic separately from the original workload request budget.
- Preserve original-token expiry, current revocation checks, cancellation, backpressure, and physical shutdown requirements.

Fresh live-query snapshots do not establish lossless journal replay. Do not report cursor continuity without testing that separate contract.
The new reader, negative controls, installed-client journey, and matching-artifact SQLite/redb trials remain to be executed.

The independent [feasibility review](/root/ROM/.superpowers/rom-010-mixed-stream-oracle-feasibility-20261009/report.md) records the inspected call paths and rejected alternatives.
