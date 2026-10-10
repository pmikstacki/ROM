# Follow-latest independent review

Date: 2026-10-07. Status: bounded source and Node review; reproduced manual-scroll finding corrected in source.

The corrected candidate preserves explicit manual reading, owns one queued animation frame, and invalidates callbacks before disposal. Reentrant following callbacks did not leave active frames after disposal. No blocking finding remains in the reviewed corrected source. The reviewer independently ran Node checks and inspected the coordinator's corrected two-engine browser log.

## Scope and source fence

Reviewed files were [follow-latest.ts](/root/ROM/studio/src/lib/ui/follow-latest.ts:1), [ui.ts](/root/ROM/studio/src/ui.ts:1), [ui-follow-latest.test.ts](/root/ROM/studio/tests/unit/ui-follow-latest.test.ts:1), and [ui-follow-latest.spec.ts](/root/ROM/studio/tests/components/ui-follow-latest.spec.ts:1). The review used Task 3A's original manual-scroll and disposal requirements. No product, repository test or manifest file was changed by this reviewer.

Evidence is under `/var/tmp/rom-010-ui-follow-review/`. `source-before.sha256` records the initial candidate. Source changed during the additional checks. The researcher read the correction and reran both Node commands. `source-after.sha256` and `source-corrected.sha256` matched:

| File | Corrected SHA-256 |
| --- | --- |
| follow-latest.ts | `fe46024b3b9eda7417cb307d6e05cfa468f262025dc28327d8a7f00d98499c2a` |
| ui.ts | `fbe6596c1fdae7f2a061f8231ce4422ddc65c7ab637915d3831030d36acf7181` |
| ui-follow-latest.test.ts | `b3588a5b6e23352e88f5168022a96e8aef11b93007b1ee86a3c10564d9820120` |
| ui-follow-latest.spec.ts | `4d3bc3e85a0de17d3fe39bb2de5cbbba84c3d36822fc92d6d0109ec6fe0f3baa` |

The coordinator then expanded the browser test file to hash `a2a431fc55a9fbaf411f7ee136c4d1e28a972f16f83616b44bfb36e81a7a58f4`. `source-browser-followup.sha256` records that change. The helper, facade and unit test hashes remained unchanged. The reviewer read the two added cases and inspected their result log.

## Reproduced finding and correction

**P2, resolved in source — Small manual upward scrolling must stop following.** The initial implementation used `nearEnd()` after detecting upward movement. A movement from 800px to 790px remained inside the 20px threshold. Following stayed enabled and later content pulled the viewport to 1000px.

`manual-near-end.log` preserves the actual wrong-state/jump characterization against initial source hash `d7d1cdedd1f764aa4c8c168cf59d5c46ab2ff9152903d0482fe4f11d51c7e454`. The finding was sent to the coordinator. It conflicted with the original requirement that manual upward movement disables following until explicit resume.

The correction turns following off for upward movement, independently of the threshold. It also stops automatically restoring following on another near-end scroll event. A viewport enlargement that clamps the old position to the new end is excluded from manual-upward detection. The new maintained small-movement and viewport-clamp cases both passed independently.

The independent test file contains the small-upward assertion. The correction landed before its first run, so `independent-red.log` actually records six passes, despite its historical filename. It must not be cited as an intended failing test run. The earlier direct characterization remains the reviewer's reproduced defect evidence.

## Executed verification

The initial maintained Node run passed six cases. After the correction, the maintained command passed eight cases:

```sh
node --experimental-strip-types --test studio/tests/unit/ui-follow-latest.test.ts
```

The independent command passed six additional checks against the corrected source:

```sh
node --experimental-strip-types --test /var/tmp/rom-010-ui-follow-review/independent.test.mjs
```

`focused-corrected.log` and `independent-corrected.log` preserve the final runs. Additional checks covered disposal from an off notification, explicit reentrant resume, disposal from a resume notification, an already-retained obsolete frame, window ownership, detached snapshots and the small upward movement.

## Lifecycle and callback assessment

Notifications and ResizeObserver callbacks coalesce through one owned frame handle. The frame reads current geometry rather than retaining an old target. It checks `disposed` and `following` before moving the element. A queued frame therefore cannot override an already-observed manual stop.

Disposal sets its flag before cancelling the frame, disconnecting the observer and removing the scroll listener. The independent test invoked a retained frame callback after disposal, even though cancellation had removed its scheduled handle. It did not scroll or queue new work. Post-disposal observer notifications, resume and content notifications are inert.

Following state changes before `onFollowing` executes. If an off notification explicitly resumes, that newer host instruction wins. If either notification disposes, subsequent scheduling checks the disposed flag. These reentrant cases passed without leaked queued frames. The helper does not swallow host callback exceptions; the public recipe should treat them as host defects rather than transport failures.

The helper obtains animation-frame functions and ResizeObserver from `element.ownerDocument.defaultView`. It does not use an implicit global window. A missing owner window fails before observation. Each snapshot is a fresh frozen object. Both motion options use instant scrolling, so the helper introduces no animated movement when reduced motion is selected.

## Browser and admission limits

The reviewer inspected `/var/tmp/rom-010-ui-follow-latest-browser-corrected.log`, which reports three cases per engine, six passes total. That log predates the review's small-upward correction and remains historical.

The later `/var/tmp/rom-010-ui-follow-latest-intent-browser.log` reports five cases per engine, ten passes total, with one worker. The coordinator associates that run with corrected helper hash `fe46024b...`; current source inspection confirmed the hash. Added cases exercise 10px manual upward movement and actual viewport enlargement/clamping in Chromium and WebKit. The reviewer read their test source and result log but did not launch browsers or independently inspect executable identity.

The browser fixture transpiles helper source into a synthetic page. Its broader five-case set also checks manual reading/resume, viewport shrink and queued disposal. It is authoring-source evidence, not an installed package consumer. These ten reported passes establish the logged fixture outcomes, not original consumer acceptance.

Disconnected/adopted nodes, late actual image growth, real touch/wheel input and callback reentrancy in both browsers were not independently executed here. The helper observes the viewport element; hosts must call `notifyContent()` after content updates that do not resize that element. Hosts must dispose it when its lifetime ends. The review does not claim automatic disposal after node removal or document adoption.

No native build, installed public consumer, release-source admission, original consumer acknowledgement or human usability acceptance follows from these Node results. The remaining Task 3A and original feedback acceptance stay open.
