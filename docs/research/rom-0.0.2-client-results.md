# Studio client and human-provider trial

## Result

The first client slice passes 21 Node tests and the Studio type check.
The human-provider fixture passes three code-flow tests.
These checks do not establish browser-to-Rust integration or release readiness.

The client has one Resource API. It has no per-kind endpoint or storage driver.
It validates discovery, Resource identity, revisions, query rows, and live snapshots.
Mutation tracking retains an independent copy of the request and its idempotency key.
A lost reply has an unknown outcome. An explicit retry sends the same request.
A receipt replay calls the server again so current authority still applies.
Session changes abort active operations and reject old replies and retries.

## Exact wire values

JavaScript Number cannot represent every i64 or u64 value.
The client parser retains large integer tokens as bigint.
The encoder emits integer tokens instead of quoted strings.
The shared field encoder distinguishes omission, null, set, and remove.
It preserves false, zero, empty strings, signed zero, and reserved map keys.
It rejects duplicate keys, malformed Unicode, nonfinite numbers, getters, cycles, and oversized input.

A source inspection of lossless-json 4.3.1 found an unsuitable object-parser contract for this boundary.
Its object assignment did not preserve the `__proto__` member in the required manner.
Its duplicate handling did not reject every repeated member with an equal value.
This trial therefore uses a small bounded parser and an executable malformed-input corpus.
This is a local source finding, not a claim that the whole upstream library is unsafe.
The release must retain further independent review and real Rust interoperability checks.

The independent review found three gaps in the first implementation:
unknown intent modes were coerced to null; unsupported shapes returned undefined;
array and map accessors could execute before encoding.
The corrected implementation rejects these cases.
The review also identified signed-zero and lone-surrogate differences from Rust JSON.
Regression tests now preserve negative zero and reject lone surrogates.

## Live queries

The client uses authenticated POST fetch streams.
It reads split LF or CRLF SSE frames and ignores keepalive comments.
It validates each snapshot's Resource kind, row identities, revisions, and row limit.
Terminal error events stop the stream. Oversized and truncated frames are errors.
The client uses its session generation. It does not invent a server descriptor generation.

Journal and operator responses still need their dedicated semantic validators.
The first slice does not claim complete cursor recovery, idle timeout handling, or browser state clearing.
Those checks must pass before Task 2 is accepted.

## Real human-provider fixture

The fixture uses oidc-provider 9.12.2 with a signed RS256 ID token.
It binds numeric loopback addresses and has a fixed, declared list of test accounts.
It requires authorization code flow, PKCE, state, and nonce.
The tests exercise login, consent, exchange, code replay, wrong verifier, and undeclared accounts.
Node tests decode the returned claims. They do not replace Rust signature verification.
The fixture is local test infrastructure, not a production identity provider.
A VPN preview still needs a correctly configured public issuer and callback.

## Evidence

- `evidence/rom-0.0.2/client/unit.log`: 21 client tests passed.
- `evidence/rom-0.0.2/client/typecheck.log`: zero errors and warnings.
- `evidence/rom-0.0.2/client/provider.log`: three provider tests passed.
- `evidence/rom-0.0.2/baseline/check.log`: unchanged native baseline passed before these additions.

The tests use injected HTTP responses for client failure cases.
The provider tests use the actual local provider endpoints.
Actual Studio authentication and both-store browser acceptance remain release gates.

## Journal validation extension

A further failing test demonstrated that the first journal method returned unchecked cursor data.
The corrected method binds cursor kind and history generation to the submitted checkpoint.
It requires strictly advancing disclosed event positions within the returned cursor bound.
Each event view receives the same identity and revision validation as a Resource reply.
Positions are normalized to bigint. Gaps are valid because authorization can hide events.

The extended suite passes 22 tests, recorded in `evidence/rom-0.0.2/client/unit-journal.log`.
Operator response validation and stream inactivity bounds remain unfinished.

## Bounded live lifecycle and operator replies

The client now bounds stream opening and idle reads.
Cancellation closes the upstream reader. Silent EOF marks the live query as stopped.
The UTF-8 decoder flushes at EOF, so an incomplete character is rejected.
An invalid media type cancels the response body. HTTP errors retain their structured category.

Operator replies are validated against protocol version 1.
Work handles, state enums, delivery outcomes, counters, and row bounds are checked before display.
Control replies must match the submitted handle, key, operation, generation, and expected revision transition.
An unresolved reconciliation must retain its expected revision and cannot claim replay.
The client snapshots submitted requests so later caller edits cannot change reply correspondence.

The extended suite passes 29 tests with zero type diagnostics.
Evidence: `client/unit-stream-work.log` and `client/typecheck-stream-work.log`.
These remain client-side failure fixtures. Actual host/browser acceptance is still required.
