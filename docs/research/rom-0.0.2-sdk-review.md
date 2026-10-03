# ROM 0.0.2 SDK independent review

Date: 2026-10-03. Verdict: changes required before SDK acceptance.

This review covers `studio/src/lib/client/`, including the current floating-token and canonical query-anchor changes.
The initial review did not change SDK source. It used source inspection and finite executable Node probes.
The coordinator subsequently authorized the scoped anchor correction documented below.
It does not certify the browser host, real provider, or native HTTP integration.

## Findings

### P2 — POST cancellation does not bound completion or reject late replies

Location: `studio/src/lib/client/session.ts:47–60` and `studio/src/lib/client/request.ts:43–45`.

`call` starts a timer that aborts the transport signal. It directly awaits `post` and checks only session generation afterward.
An injected fetch that does not settle on abort leaves the public request pending after its deadline.
When that fetch returns later, the SDK accepts the reply. An already-cancelled caller also reaches fetch and accepts its reply.

The probe configured a 5 ms deadline. After 40 ms, the signal was aborted but the read remained pending.
Releasing the fixture returned a successful read. A second probe sent one request with an already-aborted caller signal.

The body cleanup has the same completion gap. `boundedBody` awaits `reader.cancel()` without a bound.
An oversized response with a pending cancellation promise remained pending after 40 ms. Releasing cancellation produced the expected rejection.

The public transport seam permits injected fetch functions. Native fetch normally honors abort; these observations do not establish a native-browser transport failure.
The SDK still needs its own finite completion and late-reply arbitration at that seam.
For a submitted mutation, cancellation must preserve an unknown outcome and its original operation identity.

### P2 — Successful mutations do not bind revision or deletion outcome

Location: `studio/src/lib/client/session.ts:121–127` and `studio/src/lib/client/validation.ts:23–40`.

`submit` checks kind and ID through `projected`. It then marks the mutation as successful.
It does not compare the returned revision or live/deleted value with the frozen Invocation.

The probe submitted delete with expected revision 7. The fixture returned revision 3 and a non-null value.
The SDK accepted that reply and set the mutation state to `succeeded`.
Such a reply cannot represent the requested delete. It must not confirm success to the user.

Legal no-op behavior must remain supported. `crates/rom/src/execution/command.rs:267` increments the expected revision only when the value changes.
Therefore, checking only `expected + 1` would reject valid no-op replies.
Receipt replay must continue to use the saved result, rather than requiring the latest current row revision.
Malformed success correspondence must retain the original request for recovery because the server might have committed.

### P2 — Integer metadata accepts floating wire tokens

Location: `studio/src/lib/client/validation.ts:12–21`, `studio/src/lib/client/discovery.ts:88`, and `studio/src/lib/client/serialization.ts:43–56`.

The parser retains floating-token information on each original container. Integer validation receives only its primitive value.
As a result, `unsigned` accepts the parsed numeric value from `1.0` as an integer.
Direct protocol-version comparisons accept the same floating token as version 1.

The probe accepted a projected revision encoded as `1.0` and normalized it to `1n`.
It also accepted discovery protocol version `1.0`.
ROM's integer wire contract and existing CLI validation reject floating Number categories for these fields.
Floating-token preservation must not make malformed integer metadata valid.

This finding concerns response metadata. It does not prohibit finite integral-valued f64 Resource values or ordinary numeric editor input.

### P2 — Query replies ignore the submitted page limit

Location: `studio/src/lib/client/session.ts:78–83` and `studio/src/lib/client/stream.ts:101`.

Query and live replies use only the client's `maxRows` bound. They do not apply the submitted query limit.
The probe requested one row and received two distinct rows. With `maxRows: 10`, the SDK accepted both.

This admits a response outside the requested page contract. It can also invalidate assumptions in pagination and bounded rendering.
The effective response count must respect both the client policy and an explicit valid page limit.
The same rule applies to live snapshots for that query.

### P2 — The configurable response byte limit is not passed to the parser

Location: `studio/src/lib/client/request.ts:75–77` and `studio/src/lib/client/stream.ts:68`.

The response reader uses `options.maxBytes`. The subsequent `parseWire` call uses its fixed 1 MiB default.
With a configured 2 MiB policy, a valid 1,100,065-byte response still failed with `wire byte limit`.
Live opening error responses use the same inconsistent parser default.

Use one admitted policy through body reading and parsing. This finding does not require increasing the default limit.

### P2 — Timeout admission accepts durations that the execution environment cannot represent

Location: `studio/src/lib/client/session.ts:18–24` and `studio/src/lib/client/session.ts:49–52`.

`timeoutMs` accepts any positive safe integer. The probe supplied `Number.MAX_SAFE_INTEGER`.
Node 22.16.0 emitted `TimeoutOverflowWarning` and used a 1 ms timer. After 10 ms, the request signal was already aborted.
The admitted timeout therefore did not match the configured duration.

Reject timer values outside the supported timer range before starting a request. Preserve finite defaults and ordinary positive durations.
This is an observed Node result. Browser execution remains a separate acceptance check.

## Floating values and query anchors

### P2 — Query-anchor envelope admission checks only outer arrays

Location: `studio/src/lib/client/query.ts:15–34` and `studio/src/lib/client/session.ts:85–90`.

`acceptedAnchor` checks identity, schema range, array presence, and counts. It does not validate nested envelope members.
The separate probe accepted an unsupported direction, an unknown anchor state, a boolean filter, and a missing value for `state: "value"`.
It also accepted an unknown top-level field. These shapes violate the existing `QueryAnchor` wire contract.
`crates/rom/src/query_spec.rs` denies unknown fields and defines the nested order, predicate, and anchor-state structures.

The probe also requested ascending order and accepted a descending-order response. The SDK resent that anchor with the original ascending query.
Core normalization rejects that mismatch. This is a client admission and correspondence failure, not a reproduced authority bypass.

Validate the declared envelope before saving its canonical string. Check field names, directions, comparison operators, booleans, and anchor-state/value presence.
Check literal query structure against the request, including order and predicate field/operator/absent structure.
Keep predicate operands and anchor values opaque. Resource codecs can normalize operand values, so raw input equality is not a correct general check.
Keep Resource schema, current grants, and codec validation in the server pipeline.

### Weak-map safety observations and recommendations

The original parsed container preserves floating integer tokens during `stringifyWire` replay.
The probe confirmed exact replay of `{"value":1.0,"nested":[2e0]}`.
`structuredClone` loses that container metadata and produces `{"value":1,"nested":[2]}`.
A top-level `parseWire("1.0")` also serializes as `1`.

These limits are observable, but they do not establish a failure in the new canonical anchor path.
That path retains the entire native boundary as a string and parses it when sending the next query.
The current unit case successfully clones the public anchor and preserves `1.0` on replay.
`anchorRequest` also copies only requested order fields and omits unrelated projected fields.

Ordinary projected values are not safe containers for a cloned opaque native boundary.
Callers must use the dedicated canonical anchor contract for that purpose.
The host must still normalize and authorize the anchor through the Resource pipeline. This review did not rerun its Rust tests.

The separate probe also deleted a parsed float property and recreated it with an ordinary numeric `1`.
The serializer reused the old `1.0` token. Array reversal moved a float away from its recorded index and lost its category.
Replacing the float with explicit bigint `1n` correctly emitted an integer token and ignored the old float metadata.

The private WeakMap does not expose arbitrary tokens to callers. Tokens originate from the bounded JSON grammar.
Serialization rejects getters before reading object or array values. No code-execution, prototype, or unbounded token-cache defect was reproduced.
Its metadata is still tied to container identity and member position. It does not describe a freely mutable or movable primitive value.

Keep canonical boundaries immutable and preserve their full string during cloning and replay.
If a future public workflow requires mutable opaque values with category fidelity, specify that contract before extending the representation.
A dedicated lossless clone can preserve parsed containers, but ordinary `structuredClone` does not preserve private WeakMap state.
Do not claim general lossless category preservation from the present mechanism.

## Authority, replay, and lifecycle checks

Source inspection and existing unit tests support these properties:

- Requests carry same-origin credentials and request redirects are rejected.
- No response object creates an Actor or another authority capability.
- Prepared mutations have a private frozen request and session generation in a WeakMap.
- Editing public mutation fields does not change the private request used for replay.
- Explicit retry sends the same body and idempotency key. The client does not retry automatically.
- Receipt replay performs another server request and can be denied by current authority.
- A changed client generation rejects a late reply and prevents old prepared mutation submission.
- Work controls check handle, key, operation, generation, and legal revision/outcome correspondence.
- Journal events check kind, generation, increasing positions, and cursor bounds.
- SSE validates media type, fatal UTF-8, frame bytes, Resource kind, row count, and explicit closure.
- Stream opening and reads use the separate finite deadline helper.
- Duplicate JSON keys, isolated Unicode surrogates, nonfinite numbers, accessors, cycles, and unsupported shapes are rejected.

The executable negative controls rejected malformed UTF-8 and duplicate metadata keys.
No new authority bypass was reproduced. No additional facade or duplicated-implementation finding was found in the reviewed client modules.

The SDK does not retain displayed projections. Applications must clear their own state after detected session invalidation.
`App.svelte` periodically checks the browser session and disconnects on failed checks or a changed session generation.
`createApplication.disconnect` clears projections, descriptors, pending mutation state, and subscriptions.
Offline clients cannot erase previously disclosed data because the server changed authority.
The cancellation and malformed-success findings can still mislead a user about request completion. They need fixes before those lifecycle claims are accepted.

The specification also requires a bound on concurrent observations. The SDK tracks active controllers but exposes no observation admission limit.
The current application uses one subscription. A public SDK admission policy remains an explicit integration requirement, rather than a proven SDK guarantee.

## Executed evidence

Review began at HEAD `ab43c24`; the final identity capture was at `96d4080f1dee28fb9077a564fac448f66bee393e` after a parallel metadata commit.
The review includes uncommitted client serialization/session/type changes and the new `query.ts` module.
Cargo.lock SHA-256: `c278b15a2fc06ea33f253f9f3a2d70888f5de0d46c2e2d796cc63cb3d48e134b`.
Studio package-lock SHA-256: `7830342279addd8dd5dab7ed82d66aa6574b0e70839b9c326b46225394c1f68e`.
The selected source hashes are recorded in [source.sha256](evidence/rom-0.0.2/sdk-review/source.sha256).
The tracked delta is recorded in [changes.patch](evidence/rom-0.0.2/sdk-review/changes.patch).
The hash list includes the untracked query module; Git's tracked diff does not include it.

Commands executed in the existing `rom-dev` container:

```sh
cd /workspace/ROM/.worktrees/release-query-index/studio
npm run test:unit

cd /workspace/ROM/.worktrees/release-query-index
node --experimental-strip-types \
  docs/research/evidence/rom-0.0.2/sdk-review/probe.mjs "$PWD"
```

The [unit log](evidence/rom-0.0.2/sdk-review/unit.log) records 32 tests passed, with no failures or skips.
The [probe source](evidence/rom-0.0.2/sdk-review/probe.mjs) and [probe log](evidence/rom-0.0.2/sdk-review/probe.log) reproduce the observations above.
The separate [anchor probe](evidence/rom-0.0.2/sdk-review/anchor-probe.mjs) and [anchor log](evidence/rom-0.0.2/sdk-review/anchor-probe.log) cover envelope and WeakMap observations.
Each pending fixture has an explicit release. The probes use no network, credentials, provider, or product-source mutation.
Probe assertions establish the observed faulty behavior; they are not new passing acceptance tests for the requested contract.

No Cargo gate, browser suite, real provider experiment, or complete release gate was run during this review.
Parent integration owns those gates while the host source is under development.

## Authorized anchor correction

After the read-only findings, the coordinator assigned only `client/query.ts` and `tests/unit/anchor.test.ts` to this reviewer.
The patch now validates the native envelope and its nested members before retaining the canonical string.
It rejects unknown members, invalid directions/operators/states, malformed absence predicates, missing values, and duplicate order fields.
Missing anchor values remain distinct from present null values.

`acceptedAnchor(value, kind, id, query?: QuerySpec)` accepts an optional submitted query for literal correspondence checks.
The check compares predicate field/operator/absence structure and ordered field/direction structure.
It leaves operands and anchor values opaque. `queryWire` supplies its query when it admits a retained boundary.

The coordinator updated `session.anchor` to pass an exact submitted-query snapshot after awaiting the response.
That session change is outside this patch's file ownership.
The async unit case proves that caller edits during the request do not change its correspondence check.
The existing page-limit helper and integer-token metadata checks in `query.ts` were preserved.

The first [RED log](evidence/rom-0.0.2/sdk-review/anchor-red.log) records three intended behavior failures and three passes.
The stricter [RED log](evidence/rom-0.0.2/sdk-review/anchor-strict-red.log) records two intended failures for coerced operators and malformed absence predicates.
The final [unit log](evidence/rom-0.0.2/sdk-review/anchor-full-green.log) records 51 current SDK cases passed, including nine anchor cases.
The [Svelte check](evidence/rom-0.0.2/sdk-review/anchor-check.log) records zero errors and warnings.
The [format check](evidence/rom-0.0.2/sdk-review/anchor-format.log) passed for the two owned files.
The correction's hashes are recorded in [anchor-source.sha256](evidence/rom-0.0.2/sdk-review/anchor-source.sha256).

These checks used the shared working tree, including the coordinator's separate SDK repairs.
The initial review hashes and probes remain historical evidence; they are not claims about the repaired source.
The anchor correction awaits independent review and parent integration. No browser, provider, Cargo, or release gate was run for this patch.
