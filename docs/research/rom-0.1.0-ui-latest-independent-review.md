# Latest-request independent review

Date: 2026-10-07. Status: bounded source and Node review; no blocking correctness finding in the reviewed candidate.

The candidate implements the ownership ordering identified by the [primary-source research](rom-0.1.0-ui-latest-primary-research.md). Superseded wrappers settle without waiting for cooperative cancellation. Task failures become host-classified codes. Host callback exceptions follow the explicitly documented programming-error policy.

## Reviewed scope and source identity

Reviewed files were [latest-request.ts](/root/ROM/studio/src/lib/ui/latest-request.ts:1), [ui.ts](/root/ROM/studio/src/ui.ts:1), [ui-latest.test.ts](/root/ROM/studio/tests/unit/ui-latest.test.ts:1), [ui-public.test.ts](/root/ROM/studio/tests/unit/ui-public.test.ts:1), and the current [composition design](/root/ROM/docs/research/rom-0.1.0-ui-composition-admission.md:119).

Evidence is under `/var/tmp/rom-010-ui-latest-independent-review/`. Before/after SHA-256 manifests detected no product or test drift during the initial test runs. The implementation hash remained `6aab889370d63807ca6e84e826ce619321b1cb15622e7cd2b783aaaa7f071ae8` throughout review.

The facade and public test changed before finalization. The facade added selection exports; the test changed callback formatting. The reviewer reread both and reran the ten focused tests successfully. `source-final.sha256` records these changes. The final facade hash was `f4d6a52f4693215d1329cb77d8996e1f31507ba01669e7a43b6b98e7af0a7887`. Selection implementation is outside this review.

The composition design changed during review, from `781b554bcafe3bf3c107180b747e4fd4db10d30173bbe4aad3a7e97ce296436a` to `052eb503ca434c4d49db1e6c380786aec3756a4d5e17719b79ddbf2e973f78d2`. The reviewer read the added lifecycle refinement. It explicitly distinguishes task errors from host callback exceptions and describes cooperative execution limits. This report evaluates that updated contract.

No product, test or manifest file was changed by the reviewer. Additional characterization files are outside the repository.

## Executed verification

The existing focused command passed ten tests:

```sh
node --experimental-strip-types --test studio/tests/unit/ui-latest.test.ts studio/tests/unit/ui-public.test.ts
```

An independent bounded Node file passed twelve additional checks:

```sh
node --experimental-strip-types --test /var/tmp/rom-010-ui-latest-independent-review/reentrancy.test.mjs
```

`focused.log`, `focused-final.log` and `reentrancy.log` preserve output. The independent checks cover new requests started during pending/ready/error notifications, cloning, classification and synchronous task invocation. They also cover exceptions from clone/classifier/ready/error callbacks, clear during an abort listener, and twenty held tasks that ignore cancellation. No unexpected rejection appeared in these Node test runs.

## Correctness assessment

`retire` advances the epoch and installs the next owner before resolving or aborting the previous attempt. A reentrant abort listener can supersede that next owner. The outer `run` checks ownership before notification or dispatch. The independent clear/abort case also confirmed that clear does not publish idle over the listener's newer request.

Pending notification is followed by an ownership check before task invocation. Successful completion checks ownership before cloning and again before publication. Failure completion checks before classification and again before error publication. The final settlement clears `active` only if the same attempt still owns it. Ready/error callbacks therefore cannot erase a newer attempt through an obsolete finalizer.

A task can synchronously create a newer request and return a rejected promise. The original invocation still attaches a rejection handler. The obsolete failure does not classify or publish. The independent task-reentrancy case passed this scenario.

Superseding, clear and disposal resolve the detached wrapper immediately. A never-settling underlying task does not keep that wrapper pending. Disposal sets its terminal flag before aborting, so abort listeners cannot dispatch another task through that controller.

Current host callback exceptions reject the wrapper and invalidate ownership before abort. They do not pass through the server-error classifier. This differs from the research's proposed nonthrowing fallback option, but the updated design and source explicitly select rejection for programming errors. Callers must handle that promise policy. `clear()` has no promise wrapper; an exception from its idle notification propagates synchronously. This follows ordinary callback behavior, but a public usage recipe should state it when describing exception handling.

The helper does not retain a result cache or expose a snapshot accessor. It clones the task value before the single ready notification. Consequently a consumer mutation of that notified copy cannot corrupt an internal retained result. Actual copying and exact ROM representation remain obligations of the host's declared clone function.

## Execution bounds and remaining limits

The twenty-task characterization confirmed the stated cooperative limit: all twenty underlying tasks started, but only the last published ready. The controller provides one current state owner. It cannot bound physical task growth when the host ignores cancellation. Host deadlines, cancellation-aware work and admission limits remain necessary. This is an explicit contract limit, not a reproduced candidate violation.

The facade test imports `studio/src/ui.ts` directly. It establishes this source entry's export behavior, not an installed `rom-studio/ui` consumer or package artifact identity. The reviewer did not rerun Svelte checking or inspect package admission. Coordinator-reported checks remain separate evidence.

No browser, native adapter, actual export renderer, authority-revocation journey or original consumer acceptance was exercised. No human usability acceptance follows from the twenty-two bounded Node checks. Other Task 3A helpers and all later task requirements remain outside this review.
