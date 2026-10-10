# Source-link and selection independent review

Date: 2026-10-07. Status: bounded source and Node review; reproduced P2 finding corrected and independently verified.

Selection met the reviewed snapshot and replacement contracts. Source-link scheme, credential and origin checks aligned with the [primary-source research](rom-0.1.0-ui-latest-primary-research.md). The original internal decision check accepted a Promise as approval. The corrected source requires explicit `true` and consumes invalid async rejection without approving or waiting for it. No blocking finding remains in this reviewed subset.

## Finding

**P2, resolved — Require explicit synchronous approval from the internal validator.** The original [source-link.ts](/root/ROM/studio/src/lib/ui/source-link.ts:67) tested callback truthiness. A JavaScript consumer could supply `internal: async () => false`. The Promise was truthy, so the function returned `{ href: 'https://studio.example/blocked', external: false }`.

TypeScript rejects that callback against the declared boolean return type. That protection does not apply to JavaScript callers. The public helper already validates several runtime inputs and advertises route validation. It should reject an invalid decision rather than silently approve it. This does not create ROM authorization, but it bypasses the host's intended link selection policy.

Require the decision to equal `true`. Nonboolean decisions should return rejection, or raise the selected configuration error. Preserve the existing rejection behavior for a throwing callback. Do not add asynchronous link validation to this synchronous interface.

Minimal regression:

```js
resolveSourceLink('/blocked', {
  base: 'https://studio.example/',
  externalOrigins: [],
  internal: async () => false,
}); // Must not return an approved link.
```

The researcher reproduced this actual result, then retained an intended assertion failure in `/var/tmp/rom-010-ui-link-selection-review/independent-red.log`. That run had five passes and one failure. The failure was only the async-validator case. The finding was sent to the coordinator; no product correction was made by this reviewer.

The coordinator changed the decision check to `decision !== true` and added rejection consumption for invalid object/function returns. The reviewer read that correction and reran the maintained tests: thirteen passed, including async false and async rejection. The independent six-case file also passed. Both invalid async cases return null synchronously. The corrected path still rejects a throwing callback and does not await an approval decision.

## Reviewed behavior

[selection.ts](/root/ROM/studio/src/lib/ui/selection.ts:1) copies available and initial IDs into sets. It preserves exact string identity, including large decimal strings and leading-zero IDs. Each selected snapshot is a new frozen array. Changing selection cannot mutate an earlier snapshot. Catalog replacement validates before writing state and removes unavailable selections. The independently executed invalid-replacement case preserved the current selection.

[source-link.ts](/root/ROM/studio/src/lib/ui/source-link.ts:1) applies HTTP(S) and nonempty username/password checks before internal route validation. Same-origin blob URLs cannot reach that callback. Complete normalized origins distinguish scheme and port. Lookalike hosts do not match. Internal validation receives a string containing normalized pathname, search and hash, so it cannot mutate the parsed URL.

The resolver parses the base and copies the allowed origins before invoking the internal callback. The independent reentrancy check changed the host options during that callback. The returned link retained its already-parsed original origin. Each accepted result is frozen. Callback exceptions produce rejection.

The fixed 8192-byte bound applies to raw link input, including UTF-8 encoding. Configuration comes from the host. This review does not claim a bound on catalog size, base size or allowed-origin configuration. Runtime shape validation of every host configuration field is also not established by the typed interface.

The reviewed modules keep distinct responsibilities. The [ui.ts facade](/root/ROM/studio/src/ui.ts:1) contains exports. No application routes, authorization, persistence or transport dispatch entered either helper.

## Executed evidence and source fence

Evidence is under `/var/tmp/rom-010-ui-link-selection-review/`. `source-before.sha256` and `source-after.sha256` matched for selection, source-link, facade and both existing test files.

The existing focused command passed eleven tests:

```sh
node --experimental-strip-types --test studio/tests/unit/ui-selection.test.ts studio/tests/unit/ui-source-link.test.ts
```

`focused.log` preserves that output. `async-validator-characterization.log` preserves the direct JavaScript reproduction. The independent test file additionally checks selection order/snapshot isolation, atomic invalid replacement, hostile schemes, normalized port/case policy and option changes inside the callback.

The independent command was:

```sh
node --experimental-strip-types --test /var/tmp/rom-010-ui-link-selection-review/independent.test.mjs
```

No product, repository test or manifest file was changed. No native build, browser check or installed consumer was run. The focused test files import source modules directly. They do not prove package artifact admission or human usability. Current consumer acknowledgement and remaining Task 3A acceptance stay open.

## Corrected source fence

`corrected-source-before.sha256` and `corrected-source-after.sha256` matched across the independent reruns. The corrected source-link hash is `6516b0d6d84d88f1873f8666adedd4c0545486544a752671d7d339ea7a1ddc9c`. The corrected source-link test hash is `5b894ee5ef9c7796b23b3c96572ec864b1ca28e9a77ea2fc8cd2df33c130b385`.

Selection stayed at `e1f40398a4e18c81dce96fad1ac6322e7b736f8e1c28b97671770f823ed30101`. The facade stayed at `b31d9c6e26617360efbadf7fc05b3a50a8157035dd0d5b9fdf233cc123495f80`. `focused-corrected.log` records thirteen passes; `independent-corrected.log` records six passes. The original eleven-pass and intended-failure logs remain preserved. These corrected checks establish the bounded regression result only.
