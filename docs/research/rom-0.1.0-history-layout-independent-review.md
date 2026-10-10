# History and layout independent review

Date: 2026-10-08. Scope: source and bounded Node review of Task 3B candidates. All findings below are resolved in the corrected source. No blocking finding remains in this reviewed subset.

Reviewed `studio/src/lib/ui/history.ts`, `layout.ts`, `messages.ts`, `components/HistoryList.svelte`, their maintained units and the history browser fixture/spec. The browser slot remained coordinator-owned. This reviewer changed no product or test file and launched no browser or native build.

## Reviewed source and checks

| File | SHA-256 |
| --- | --- |
| history.ts | `0cc4c00bfbd1f449d4e6ded5dc718ca132a2f88a667df8f16bdddb00c6170a19` |
| layout.ts | `772c51aee444a6feb5a91c814098f2d8ac26cb18a06b67c540f5e1c63acd5377` |
| messages.ts | `e8864a45f407b872cfb37013c20eadead790639905ce70e75ce0497a7665f8b7` |
| HistoryList.svelte | `027e3fce2ce34d0cf769adc8d3cf1c705b6070bf0826ec32a31a5be6010644cf` |

The reviewer ran:

```sh
node --experimental-strip-types --test tests/unit/ui-history.test.ts tests/unit/ui-layout.test.ts
```

Nine maintained cases passed. `/var/tmp/rom-010-history-layout-review/maintained.log` preserves the run. Independent Node characterizations are in `getter-probes.log` in that directory. These probes ran against the hashes above. Subsequent corrections require fresh verification.

## Findings

**P2, resolved — Capture history fields and collection length once before validation.** [history.ts](/root/ROM/studio/src/lib/ui/history.ts:24) originally validated repeated reads, then read fields again for the returned object. A title getter returned `safe` for the first three reads and 5000 characters on the fourth. The helper admitted a title beyond its 4096-byte limit. The coordinator corrected the entry snapshot. A remaining collection probe supplied 201 unique valid entries through a proxy. Its length was 1 at the limit check, then 201 during `map`; `captureHistory(entries, 1)` admitted 201 entries. Snapshot the collection length once and iterate only that validated bound. Avoid caller-controlled `map` or iterators. `history-array-length.log` preserves the corrected-source reproduction.

**P2, resolved — Validate the exact captured catalog bounds.** [layout.ts](/root/ROM/studio/src/lib/ui/layout.ts:61) spreads the catalog entry after validating repeated field reads. Inherited min/max properties pass validation but disappear from the spread snapshot. The probe admitted width 10 and height 10 against maximum width 6 and height 4. A separate getter returned maxWidth 6 during validation and 12 during spread; width 10 was accepted. Capture the declared fields once into a named catalog record, then validate and retain that record. Do not infer absent bounds or copy arbitrary properties. Keep configuration failure distinct from stored-input rejection.

**P2, resolved — Isolate array input shape and length failures.** [layout.ts](/root/ROM/studio/src/lib/ui/layout.ts:63) reads stored input before its catch boundary. An array proxy whose length getter throws escaped as a raw exception. The existing entry getter regression does not cover this path. Put stored-input array classification and length reads under the invalid-input boundary. A length getter failure should produce `LayoutShape`, without changing configuration-error behavior.

**P2, resolved — Iterate the validated stored-layout count.** The corrected layout still uses an array iterator after checking length. An ordinary first-entry width getter appended a second valid entry during capture. With `maxItems: 1`, both entries were returned as valid. Capture length once and iterate only that count. `layout-array-growth.log` preserves this independent reproduction. A getter must not widen the admitted collection after its limit check.

**P2, resolved after coordinator browser reproduction — Use consistent identity comparison for the unrestricted authority token.** [HistoryList.svelte](/root/ROM/studio/src/lib/ui/components/HistoryList.svelte:55) uses `owner === authorityToken` after awaiting selection. The public token type is `unknown`, which permits NaN. With a stable NaN token, the comparison is always false, so a resolved callback leaves pending set and all rows disabled. Use `Object.is` consistently, or explicitly narrow and validate the token contract. A maintained browser case should select with NaN authority and verify busy state clears. This reviewer did not execute that browser reproduction.

## Contract assessment and limits

The component uses native buttons. Keyboard activation remains browser-owned. Exact IDs are callback arguments; titles are display labels. Only host `selectedId` sets the current marker. A resolved refusal does not optimistically select a row. Pending navigation disables other row callbacks, and thrown/rejected selection reports a catalog message rather than its raw exception.

Locale and timezone feed Intl formatting. Count messages receive both the numeric count and its locale-formatted text. Invalid locale/timezone or a defective message resolver remains a host configuration failure. The component has no backend classifier, persistence or mutation dispatch.

The authority token fences pending failure/busy publication. It does not cancel arbitrary host navigation or clear the supplied entries. The host must clear or quarantine private disclosures on authority change, and recheck current authority after its own awaits. The fixture's host callback demonstrates an owner recheck; its authority-change button retains entries. That fixture does not prove private history clearing.

The existing layout getter case proves that stored field getters cannot widen captured top-level columns. It does not prove the catalog capture or top-level input isolation identified above. LayoutControls, persistence/reload and current-source installed-consumer acceptance remain separate obligations.

No browser counts, full Studio suite, native verifier, installed package, original-consumer acknowledgement or human usability acceptance were established by this review.

## Correction drift and follow-up

The closing source fence found coordinator edits. The entry and catalog field snapshots now read declared values once. Stored array shape and length exceptions are inside the invalid-input catch. History changed to `687eebf4567e1b664ec808936bc2d3991954cbe0306a2a67a7a8e5e2edc1f38d`; layout changed to `8915511b694c64bd13bbcc12bcbdb7764f36106d707b5f3791078bb128324a13`. Component and message hashes remained unchanged at that fence.

The reviewer reread these corrections and independently ran twelve maintained tests plus four direct capture/boundary assertions. They passed. Logs are `corrected-maintained-first.log` and `corrected-probes.log`. This resolves the original entry getter, catalog getter/inherited bounds and throwing array-length cases at those hashes. The subsequent collection-limit probes above still fail the stated bounds. Further coordinator edits require another source fence and fresh checks.

## Final corrective verification

The coordinator added maintained regressions and corrected all reported paths. History captures each field and the collection length once. Layout captures catalog fields once and iterates the validated input count. Input classification and length failures return an invalid result. History callback fencing uses Object.is, including NaN token identity.

| Final file | SHA-256 |
| --- | --- |
| history.ts | `476be19f48d42e4b1577b3aeae09b077503a00a7e332502debb10e68db873549` |
| layout.ts | `97061c04d9da88de8a45f32c7276a788300c4c34bffa042e9e88d7a15801aaec` |
| HistoryList.svelte | `88a61a1d872cff62c8283a9ccb33990c1be93783210c404942cdedf5059f01e6` |

The reviewer independently reran all fourteen maintained Node tests. They passed. Final independent collection probes confirmed one length read and one admitted item under a limit of one. Logs are corrected-maintained-final.log and final-collection-probes.log. Earlier four capture/boundary assertions passed after the first correction; the final maintained cases cover those paths again.

The reviewer inspected the coordinator's /var/tmp/rom-010-history-layout-review-browser.log: fourteen cases passed across both engines, including the NaN pending-state regression. This is inspected coordinator evidence, not a browser run performed by this reviewer. The historical NaN RED and collection RED remain coordinator-owned evidence.

ReferencePicker was also checked for the unrestricted token identity issue. Its selected-title, selection reentry, scope-change and completion guards already use Object.is. No matching source finding exists there.

Final verification resolves the findings above without changing opaque IDs, host-selected current markers, manual fallback policy or disclosure authority. Installed consumer, original consumer, human usability and release acceptance remain open.

The reviewer also inspected `/var/tmp/rom-010-history-layout-final-evidence.json` and verified all nine listed hashes against current files. Its final logs show 407 unit passes, zero check errors/warnings and fourteen browser passes. Those broad checks were coordinator-executed. This review independently executed the focused Node checks only.
