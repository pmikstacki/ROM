# Browser codec and human provider fixture review

Date: 2026-10-03. Reviewer scope: serialization, normalization, public types, and the human provider fixture.
Status: initial correctness findings repaired by the implementation owner; focused rechecks passed.
This reviewer changed no implementation or tests. Requests and session code under construction were outside this review.

## Findings and resolution

| Initial finding | Reproduction and consequence | Recheck |
| --- | --- | --- |
| Unknown mutation intent became null | `objectInput` with a nullable field and `{mode:'mistyped'}` emitted an explicit null. A malformed draft could become a valid unintended mutation. | It now throws `invalid field intent`. The regression test passed. |
| Unknown shape returned undefined | `normalizeValue({type:'new-shape'}, true)` returned undefined instead of an unsupported-shape error. | Exhaustive rejection now throws `unsupported shape`. The regression test passed. |
| Array and map getters executed | An array element getter ran during serialization. A map getter ran through `Object.entries` during normalization. This differed from object serialization's accessor refusal. | Both operations now reject accessors without invoking them. The regression test and direct counter probe passed. |
| Integer-token negative zero lost its sign | `parseWire('-0')` produced positive zero. The maintained Rust parser treats this token as negative floating zero. | The parser preserves negative zero. Serialization preserves its sign. The regression test passed. |
| Lone Unicode surrogates were accepted | JavaScript accepted an escaped lone high surrogate. Rust String decoding rejects such surrogate escapes. | Parse and serialization now reject unpaired surrogates. The regression test passed. |

These findings concern the initially inspected working source. The owner repaired them while review continued.
They are not remaining defects in the final hashes below.

## Behaviors that passed

Exact u64 and i64 boundary tokens become bigint without Number rounding.
Scalar normalization enforces integer ranges and rejects unsafe Number inputs for integer fields.
F64 remains finite floating-point semantics; it does not promise arbitrary decimal precision.

Duplicate JSON keys are rejected, including keys that become equal after escape decoding.
Reserved names remain data in null-prototype maps. The observed `__proto__` and `constructor` cases did not mutate object prototypes.
Rejecting all reserved names would unnecessarily prevent valid schema members; safe representation is the correct boundary here.

The parser bounds UTF-8 bytes, nesting, and number-token length.
Serialization bounds output bytes and nesting, rejects cycles, and refuses unsupported accessors.
Object and patch helpers distinguish omission, null, value, and removal.
Optional fields alone admit explicit removal. False, zero, and empty strings remain values.

TypeScript types do not validate incoming wire values by themselves.
Discovery and request readers must retain their separate runtime validation and correspondence checks.
This report does not approve those concurrently written modules.

## Human fixture assessment

The fixture uses the pinned upstream `oidc-provider` rather than fabricating a token response.
It performs authorization-code login, consent, required PKCE, actual token exchange, and one-time code consumption.
The provider signs ID tokens with its own generated RSA key.
The observed tests reject missing PKCE, a wrong verifier, undeclared accounts, non-loopback callbacks, and empty credentials.

The fixture deliberately presents declared local accounts without a production password system.
Its HTML identifies this limitation. This is realistic protocol acceptance, not proof of production user credential verification.
The default in-memory provider adapter loses state on restart, as its own warning states.
Provider restart and persistent-session acceptance therefore need explicit later journeys.

The Node fixture test decodes token claims but does not verify the signature.
Its source correctly records this distinction. Rust host acceptance must verify the original signature, issuer, audience, nonce, and current binding.
State, callback replay, wrong-key tokens, unsafe origins, and current identity races remain host acceptance obligations.
Do not cite the Node test as proof of those checks.

The provider is loopback-only and registers an exact numeric-loopback callback.
That is appropriate for browser tests running on the same host.
It does not establish a Mac-accessible VPN issuer or HTTPS deployment profile.
Persistent preview acceptance must use browser-reachable approved provider endpoints separately.

The fixture rejects a supplied incorrect Origin during interaction POST.
Its Node harness also permits requests without Origin. This controlled harness is not a production CSRF acceptance test.
The Studio host still needs mandatory origin/CSRF enforcement for cookie-authenticated unsafe routes.

## Executed checks

Node version: `v22.16.0`. Source checkout: `ba96fe70c06d307390b815c417ee9c22ae5b1913`, with concurrent uncommitted implementation changes.

| Command | Result |
| --- | --- |
| `node --experimental-strip-types --test studio/tests/unit/codec.test.ts` before repairs | Seven existing tests passed; direct independent probes reproduced the gaps above. |
| `node --experimental-strip-types --test studio/tests/unit/codec.test.ts` after repairs | Ten tests passed. |
| Direct codec probes after repairs | Unknown shape/intent and unpaired surrogate throw; negative zero survives; accessor counters remain zero. |
| `node --test demo/provider-fixture/human-provider.test.mjs` | Three tests passed against the actual local upstream provider. |

Rust interoperability findings also used source inspection of pinned `serde_json` 1.0.151.
Its integer parser explicitly converts `-0` into negative F64. Its String reader rejects lone surrogate escapes.
No Cargo verification gate ran during this review.

## Final reviewed source identities

| File | SHA-256 |
| --- | --- |
| `studio/src/lib/client/serialization.ts` | `9d22f5709224122a61786c4edf981f39f9778a1f7e87a9ee91a4a1254d707c9f` |
| `studio/src/lib/client/normalization.ts` | `f5e228f2c36642daf86317eec9e083e7e61e25e52772fd23b71e351474da9a12` |
| `studio/src/lib/client/types.ts` | `2b14b6f643416dac5532d8ce71f820effd99a4a6d10c1a21cf305cff1eb1c525` |
| `demo/provider-fixture/human-provider.mjs` | `2910be3511443362097e9ae6a89a74c31fda4413d04d549dd14719ec2dc88ab7` |
| `demo/provider-fixture/human-interaction.mjs` | `1ef4e4ea5948b3935c6723726353be338d1a2724916df74c3df4a88dfb2e60a4` |
| `demo/provider-fixture/human-provider.test.mjs` | `67cb29f81f1953bf214502e0e934733b5e72fbb56840b7519b92ed95422e25b2` |

No blocking finding remains in the rechecked codec/fixture scope.
Full host, browser, provider-to-maintenance, and packaged distribution acceptance remain separate obligations.
ASD-STE100 is a writing guide. This report does not claim certified compliance.
