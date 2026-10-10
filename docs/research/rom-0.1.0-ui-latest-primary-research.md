# UI Task 3A primary-source research

Date: 2026-10-07. Status: research and design review; no product implementation reviewed or admitted.

The Task 3A design needs explicit reentrancy, cancellation settlement and URL classification rules. Exact ROM cloning also requires more than bigint preservation. Export identity records the captured context; it does not establish current authority.

## Scope and evidence

The researcher read the [composition design](/root/ROM/docs/research/rom-0.1.0-ui-composition-admission.md:80), [Task 3A plan](/root/ROM/docs/superpowers/plans/2026-10-07-ui-composition-admission.md:75), and actual client codec before external research. The relevant implementation is [serialization.ts](/root/ROM/studio/src/lib/client/serialization.ts:1), exported through [codec.ts](/root/ROM/studio/src/lib/client/codec.ts:1). `studio/src/ui.ts` and `studio/src/lib/ui/` did not exist at the source check. Findings below concern the proposed interfaces, not defects reproduced in those future helpers.

Evidence is under `/var/tmp/rom-010-ui-primary-research/`. `source-before.sha256` and `source-after.sha256` fence the four inspected design/codec files. The hashes matched; no drift was detected in that subset. `characterize.mjs` and `characterization.log` contain the executed Node v22.16.0 characterization. The command was:

```sh
node --experimental-strip-types /var/tmp/rom-010-ui-primary-research/characterize.mjs
```

All characterization assertions passed. No ROM helper tests, browser checks, native builds, packaged consumer journeys or human usability review were executed. The examples use synthetic URLs and contain no credentials from a real account.

## Researched platform facts

Promises have no built-in abort mechanism. `AbortController` changes its signal; the receiving API determines how to respond. The abort algorithm runs registered abort algorithms and fires the abort event within the abort operation. Promise-based web APIs that support cancellation must observe the signal and reject on abort. These rules do not force an arbitrary host task to stop. [WHATWG DOM, section 3](https://dom.spec.whatwg.org/#aborting-ongoing-activities), [abort steps](https://dom.spec.whatwg.org/#abortsignal-abort), [API requirements, section 3.3](https://dom.spec.whatwg.org/#abortcontroller-api-integration).

HTML structured serialization preserves primitive BigInt and Number values. Its memory map preserves cycles and repeated object identity. Those features do not define ROM's wire representation or require preservation of ROM's external lexical metadata. [WHATWG HTML, StructuredSerializeInternal](https://html.spec.whatwg.org/multipage/structured-data.html#structuredserializeinternal).

A parsed URL includes credentials when its username or password is nonempty. An `invalid-credentials` validation error is not necessarily parsing failure. A `blob:` URL can inherit an HTTPS origin. URL origin alone therefore does not identify the scheme of a link. [WHATWG URL, URL representation](https://url.spec.whatwg.org/#url-representation), [validation errors](https://url.spec.whatwg.org/#validation-error), [origin](https://url.spec.whatwg.org/#concept-url-origin).

Svelte calls cleanup returned by a synchronous `onMount` callback during unmount. An async callback returns a promise instead of that cleanup function. `onMount` does not execute during server rendering. [Official Svelte lifecycle documentation](https://svelte.dev/docs/svelte/lifecycle-hooks#onMount).

These are current primary sources read on the research date. The DOM page identified its update date as 2026-10-05. The Svelte page is unversioned. None establishes ROM authorization, recovery knowledge or consumer acceptance.

## ROM source facts and executed characterization

The codec keeps decimal/exponent tokens in a module-private `WeakMap`, keyed by the original parsed container. `stringifyWire` consults that metadata for each member. It also emits bigint as integer tokens, bounds bytes/depth and rejects getters, holes, cycles and unsupported objects.

The characterization parsed this value:

```json
{"id":9007199254740993,"float":1.0,"exponent":1e0,"nested":[1.00]}
```

After `structuredClone`, `stringifyWire` emitted `1` for each floating member. The large integer remained exact. A `parseWire(stringifyWire(value))` copy retained every numeric token in this example. This demonstrates a ROM-specific metadata loss, not a failure of the HTML cloning contract.

This guarantee concerns retained tokens on container members. A primitive Number has no container key for that metadata. The codec cannot recover original spelling or floating category already lost before capture. Exact cloning must not claim broader lexical preservation than the input representation supports.

The abort event order was `before`, `listener`, `after`. A promise that ignored a separately aborted signal still completed when its resolver was released. URL characterization accepted a credential-bearing HTTPS URL syntactically. `blob:https://allowed.example/id` had origin `https://allowed.example` and protocol `blob:`. A tab-containing JavaScript scheme normalized to `javascript:`. These are Node platform observations, not browser compatibility results.

## Design gaps and proposed contract refinements

These recommendations are ROM policy and design inference. They are not additional platform requirements or completed acceptance.

### P1: Fence ownership before cancellation and every host callback

The design requires stale-result suppression but does not specify abort-listener reentrancy. A previous task's abort listener can synchronously call `run`, `clear` or `dispose`.

Advance the ticket and detach the previous owner before calling its `abort()`. After abort returns, confirm that the initiating operation still owns its ticket. Only then publish pending state or invoke its task. Otherwise an outer `run` can overwrite a newer request started inside the abort listener.

Treat `clone`, `classifyError`, `onState` and the task invocation as reentrancy boundaries. Check ownership after each callback, before each subsequent state write or callback. A superseded finalizer must not clear a newer controller. A clear/dispose notification must not overwrite a request created inside that notification.

Define callback exception behavior. A throwing notification must not become an unhandled `run` rejection or recursively enter error publication. A throwing classifier needs a bounded fallback code. Callbacks receive independent snapshots; mutating one must not mutate retained private state. Host clone functions must actually copy their declared shape.

Minimal regressions: abort listener starts request C while B supersedes A; pending notification clears; ready clone starts C; classifier disposes; notification throws. In each case only the current owner can publish or finalize.

### P2: Specify superseded caller settlement

The design says `run` resolves after publishing or suppressing its outcome. It does not define settlement when a superseded task never settles.

Choose an explicit policy: settle the wrapper when ownership is revoked, while continuing to consume later task rejection; or document that settlement waits for the task. The former gives predictable cancellation for callers. Neither policy stops a task that ignores its signal. Describe the guarantee as one current state owner, rather than one physically executing computation. A timeout for current computation remains separate host policy.

Minimal regression: supersede a held task that ignores abort. Verify the documented wrapper settlement without releasing it. Then reject the old task and confirm no unhandled rejection or state publication.

### P1: Apply scheme and credential checks before internal classification

The design explicitly rejects non-HTTP(S) external schemes. Apply that scheme boundary to all returned links. Otherwise a same-origin `blob:` link can reach the internal validator.

Parse once against an explicit validated base. Reject non-HTTP(S) protocols and nonempty username/password before invoking the internal-path callback. Validate the base and allowed-origin configuration separately. Compare normalized complete origins, never prefixes or hostname suffixes. Same origin does not itself approve an internal route.

Give the internal validator an immutable normalized input, or a disposable URL copy. A callback must not mutate a previously validated URL into the returned link. Callback failure should produce rejection. Specify an input length bound as ROM policy; the platform parser does not supply the proposed helper's application bound.

Minimal regressions: same-origin blob; credential-bearing same-origin HTTPS; normalized JavaScript scheme; protocol-relative hostile origin; lookalike host; internal callback that throws or mutates its input.

### P1: Capture export content separately from current disclosure authority

The selected interface copies a value but cannot independently determine current authority. The host recipe must supply that decision. A captured principal or authority ticket is evidence of the original context, not a current grant.

Copy identity fields and value before the renderer starts. Preserve exact ID/revision values without ordinary JSON. For `WireValue` trees, use bounded `stringifyWire`/`parseWire`; ordinary host data needs its own declared clone contract. Do not wire-clone an entire typed wrapper merely to copy its wire subtree.

Retain a private captured snapshot for retries. Each renderer receives a separate copy if it can mutate its argument. Changing selection creates another request; it must not silently change a retry's captured revision/date/locale.

Recheck current owner and authority after rendering, before publishing bytes, and again before a later manual file action. If an authority decision or other callback can await or reenter, check ownership again afterward. Synchronous capture alone cannot cover revocation during rendering. Clearing UI state cannot retract bytes already disclosed.

Define same-principal authority renewal explicitly: either invalidate the captured ticket or authorize rebinding through current host policy. Principal equality and a latest-request identity string are insufficient to make that decision. The helper must not infer permission or R4 mutation settlement.

Minimal regressions: mutate original identity/value after capture; mutate a renderer argument before retry; change principal during a held renderer; revoke authority after bytes are ready but before manual save; reenter during authority check; renew the same principal with a new grant.

### P2: Connect explicit disposal synchronously

A Svelte recipe should return helper disposal from a synchronous `onMount`, or register equivalent destruction cleanup. Async setup needs a separate owned operation. Disposal must invalidate pending publications before aborting or disconnecting resources. This is a recipe requirement; the headless helper must not require an implicit component or global document owner.

## Admission boundary

The proposed refinements preserve Task 3A's original scope: disposable computation, display and export ownership. They add no accepted mutation semantics, document engine or implicit persistence. File-picker cancellation must remain separate from any manual fallback action, as the existing plan requires. This research does not establish that behavior in browsers.

The coordinator retains implementation and test ownership. Current consumer acknowledgement, release-source admission and human acceptance remain unestablished by this report.
