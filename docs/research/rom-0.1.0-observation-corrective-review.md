# Observation corrective independent review

Date: 2026-10-07. Status: bounded Node and source review; original synchronous findings and additional async callback gap verified corrected.

The correction handles synchronous and async subscriber/hook failures and nested idle rebinding. Reentrant diagnostics did not publish an old scope or dispatch an obsolete source. No blocking finding remains in the corrected subset. Async listener work after its own await remains host-owned.

## Source and executed checks

Reviewed [controller.ts](/root/ROM/studio/src/lib/observe/controller.ts:1), [types.ts](/root/ROM/studio/src/lib/observe/types.ts:1), and [observation-controller.test.ts](/root/ROM/studio/tests/unit/observation-controller.test.ts:1). The reviewer also read the scope implementation and ran the maintained scope/public-entry tests.

The inspected candidate hashes match `/var/tmp/rom-010-observation-corrective-evidence.json`:

| File | SHA-256 |
| --- | --- |
| controller.ts | `995cbc8209497bf4a83aef508c43a4b79ec798eab8c15c860b766af18ea44546` |
| types.ts | `0f99a62b74441ced6099aa4cce99c6660401a4cff7e13d755f9f99ad548e67df` |
| observation-controller.test.ts | `8e7220370b3801128c4ae9ecd2db4c4920b95fbb9b0512f06bbd91a54c0acc7d` |

Evidence is under `/var/tmp/rom-010-observation-independent/`. `source-before.sha256` and `source-after.sha256` matched. The reviewer executed:

```sh
node --experimental-strip-types --test studio/tests/unit/observation-controller.test.ts studio/tests/unit/observation-scope.test.ts studio/tests/unit/observation-public.test.ts
node --experimental-strip-types --test /var/tmp/rom-010-observation-independent/diagnostics.test.mjs
```

All nineteen maintained tests and five additional tests passed. Logs are `focused-all.log` and `diagnostics.log`. An earlier two-file command passed eighteen tests; the public-entry test supplies the nineteenth. The coordinator's historical three-failure RED and reported 389-unit/check results remain separate evidence. This reviewer did not rerun the full Studio suite or Svelte checking.

## Original fixes and reentrant diagnostics

The maintained denied-subscriber case verified that a throwing view cannot suppress healthy subscribers or the authority-loss callback. The connecting-subscriber case verified that the source still starts. The nested idle rebind case verified one surviving source invocation without an obsolete reconnect.

`notify` removes the defective synchronous subscriber before invoking the diagnostic. `publish` checks its original epoch before each remaining listener. `rebind` checks its original stop ticket after publication. These guards preserve a newer owner created during notification.

The additional connecting-diagnostic case rebound from Alice to Bob, then threw. Alice's source never started. Bob's source started once and published fresh. Another case rebound during denied diagnostics; the obsolete authority callback and remaining old-scope notification were suppressed.

Diagnostic hide/dispose cases prevented initial source dispatch and survived a synchronous diagnostic exception. The authority-hook case rebound to Bob, threw, then disposed from its diagnostic. Disposed state retained Bob's scope and contained no rows. Diagnostics exposed only the declared callback codes.

## Additional reproduced finding and correction

**P2, resolved — Consume rejected async host callbacks or enforce a synchronous return contract.** The original [controller.ts](/root/ROM/studio/src/lib/observe/controller.ts:66) ignored the result of `onCallbackError`. Its `try/catch` could not consume a rejected promise. The declared `void` callback shape does not prevent TypeScript callers from supplying an async function.

The direct Node characterization supplied `onCallbackError: async () => { throw Error('diagnostic private failure'); }`. A throwing subscriber invoked it. The observation reached fresh, but the process received an unhandled rejection containing that raw callback message. `async-diagnostic.log` preserves the result. The characterization installed a process listener to record the event; it did not silence it in product code.

The same ignored-return pattern existed for subscriber invocation and the authority hook. Async subscriber rejection also escaped removal/diagnosis; an async authority hook rejection escaped its diagnostic. These are host defects, but that candidate's callback isolation covered only synchronous exceptions.

Handle rejected callback results without awaiting them or changing observation ownership. Preserve sanitized diagnostics and the existing epoch guards. Alternatively, reject async returns explicitly and document the synchronous-only boundary; rejected promises still need consumption. Minimal regressions should cover rejected async diagnostics, subscribers and denied hooks. Additional reentrant tests must confirm that rejection handling cannot restore an obsolete scope.

The coordinator added [callbacks.ts](/root/ROM/studio/src/lib/observe/callbacks.ts:1), a named failure-consumption helper. It catches synchronous failure and consumes promise/thenable rejection. The subscriber failure path removes the subscriber and reports its fixed code. The authority-hook failure path reports its fixed code. Diagnostic failure has a nonthrowing sink. None awaits the callback or assumes its lifecycle ownership.

The coordinator reproduced the async defect in a maintained test: thirteen passes and one unhandled-rejection failure. Its first corrected test run had a separate premature removal assertion. That expectation was corrected to wait for the rejection diagnostic before starting observation. Removing a subscriber when its promise settles is the implemented contract. Removing or coalescing pending async calls was not introduced.

The reviewer reran the corrected maintained suite: twenty passed. Eight independent synchronous/async diagnostic checks passed. Added cases covered a throwing `then` getter, a getter that synchronously disposes before dispatch, and rejecting subscriber/authority/diagnostic thenables with reentrant rebinding. No unhandled rejection appeared in these test runs. Logs are `focused-async-final.log` and `diagnostics-async-first.log`.

The helper/controller/types hashes stayed stable during those checks. The maintained test changed to the corrected removal expectation; the reviewer reread it and reran all twenty cases. `async-corrected-source-final.sha256` records the final source:

| File | Corrected SHA-256 |
| --- | --- |
| controller.ts | `c96ff72b75668fb1ad80998d2e8fe199165e514ae9c18fafc3417ce22ebf7274` |
| types.ts | `0f99a62b74441ced6099aa4cce99c6660401a4cff7e13d755f9f99ad548e67df` |
| callbacks.ts | `97e2e031ace9f47ff9c6331a342cca96e101d4664a3b9064b5ff6cb93aa14f4a` |
| observation-controller.test.ts | `a57aea5980e3b97fd054904c091af66d611fe9dc41ae1683883b7223641bf128` |

This correction does not serialize async subscribers. They can receive later notifications before their first returned promise rejects. Already-delivered snapshots cannot be withdrawn. Host callbacks must recheck current scope/authority after their own awaits before changing UI or disclosing data. Failure consumption does not grant permission, recall data or fence arbitrary host continuations.

No product or repository test file was changed by this reviewer. No native build, browser, installed consumer, release admission or original consumer acknowledgement was established. The observation metadata remains disclosure scope, not a current authorization grant.
