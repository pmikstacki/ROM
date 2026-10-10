# Feedback coverage audit, 2026-10-08

All 47 references returned by the fresh read have explicit dispositions: 41 map to active intake groups, four to withdrawn groups, and two to non-usability exclusions. No reference is unmatched. This is reference accounting, not acceptance of every usability requirement.

## Scope and method

The fresh record is [feedback-sync-2026-10-08.json](rom-0.1.0-feedback-sync-2026-10-08.json), checked at 2026-10-08T06:44:59.873Z. Its thread updated_at is 1791396483. The provider returned eight turns and hasMore=true. The intake contains 33 groups.

The audit joins exact turn_id and item_id pairs to intake feedback_refs, including the same thread_id. For exclusions, it joins the explicit non_usability_messages records in the prior sync. Presence in a sync list or a text search alone was not treated as disposition. All 47 pairs are unique. Raw messages and credentials are not republished.

The older paginated intake records 22 turns. That does not remove provider-truncation or uninspected-image limits. This audit cannot certify requirements omitted from available text. It does verify every reference in the fresh returned page.

| Audited record | SHA-256 |
| --- | --- |
| docs/research/rom-0.1.0-feedback-sync-2026-10-08.json | `140fcdfa3ec78010a9bbe9c9ca5f318f6852abe79894752f28608017d212beb3` |
| docs/research/rom-0.1.0-feedback-sync-2026-10-07.json | `535535337829f4ce747f2b7bb778191c2af075d84af37185480e780b58118e21` |
| docs/research/rom-0.1.0-usability-intake.json | `6fb3cebe2ec00f4ebc07e8756dcd69db99bce77105409cbf4a42d87dace3c5b8` |

## Explicit disposition of every fresh reference

| Turn ID | Item ID | Intake group or exclusion |
| --- | --- | --- |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115c2-99d1-74d0-a928-40460585c476 | AP-UX-030 — withdrawn |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115c2-d03e-7862-81d9-fb8bd07e31eb | Non-usability: Instruction to resume work; no additional usability requirement. |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115ce-a388-7962-b7d4-fb57537d1cd8 | AP-UX-007, AP-UX-013 |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115d4-52de-7940-84e1-9f10c22816e2 | AP-UX-001 |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115d6-8250-7ab0-83fb-9d74c8688fae | AP-UX-014 |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115d8-8e0d-7602-b6fa-904b1db8144f | AP-UX-016 |
| 01a115c2-999b-77b2-a90f-449043de2e9a | 01a115e5-a77a-7e23-84e3-d10aae3817b0 | AP-UX-015 |
| 01a115c2-766b-7200-b992-27a17497e05a | 01a115c2-76a6-74d3-baa7-728a4e3cfb01 | AP-UX-030 — withdrawn |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11544-afce-73e3-bd8c-8eb974868ce2 | AP-UX-004, AP-UX-006, AP-UX-007, AP-UX-024 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11545-3d0d-7c00-8495-f9da0cae4305 | AP-UX-008 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11546-17cf-79e2-a544-46c43f247674 | AP-UX-008 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11558-1568-7a12-a225-6fcd8c7164ac | AP-UX-009 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11558-e61d-7752-be93-c871d4753ea0 | AP-UX-004, AP-UX-005, AP-UX-026 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11560-ac1c-7830-a70f-5773e9ee35fa | AP-UX-017, AP-UX-028 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11561-963d-7f23-844c-9c93423fde06 | AP-UX-017 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11562-2a35-7061-a5b4-956451d93595 | AP-UX-025 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a1156a-d91a-7212-9e9e-9076bad3e617 | AP-UX-005, AP-UX-009, AP-UX-022 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11570-b070-7dd1-a2bf-8800e1d32bb5 | AP-UX-027 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11573-c0cd-7e41-85d9-8fd12aa08f88 | AP-UX-027 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11577-2d9a-7c30-a7fb-e9765a34b9cc | AP-UX-012 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11580-7143-7523-b431-d6555c66d146 | AP-UX-011 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11583-fb95-7f63-ba69-8698ced76e0b | AP-UX-015 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a11583-fb9a-77a0-9694-f062bc7e8a02 | AP-UX-029 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a1159b-7f31-75a3-9a95-91c46e37b530 | AP-UX-032 — withdrawn |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a115a2-9a05-7320-8c34-9cf11d8312b6 | AP-UX-031 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a115a8-38ad-7133-bd62-8e66580ad484 | AP-UX-032 — withdrawn |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a115a9-3946-7d91-a1a9-e83d6dfce33d | AP-UX-015, AP-UX-023 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a115ac-592a-7603-9b64-a7f4bdc7352d | AP-UX-005, AP-UX-029 |
| 01a11544-af8a-78d1-af2e-2c3df58a89f3 | 01a115ae-c75d-7e03-9884-37c48b10f49b | AP-UX-024 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a1151b-5132-72c1-92ad-fcb366809351 | AP-UX-018 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a11520-3037-72c0-848c-c9a2d1a520d4 | AP-UX-029 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a11523-29e7-73f2-b77a-28725a7ea45c | AP-UX-029 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a11523-e310-7f92-9dd4-2d7e0900e143 | AP-UX-018 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a1152c-989d-7f92-bfb5-50478f0d1d12 | Non-usability: Credential and provider setup request. No credential value is retained; provider policy is tracked in AP-UX-027. |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a1152c-98a2-7611-b4f8-c331f2248544 | AP-UX-018 |
| 01a1151b-50ef-7543-b1ba-fe182955be8b | 01a1152d-81bd-7f10-8ab1-5d72e8b89b69 | AP-UX-027 |
| 01a11513-66a4-7c30-a318-bf4611d830f6 | 01a11513-66f2-7e51-a457-c55ac930e799 | AP-UX-011 |
| 01a11513-66a4-7c30-a318-bf4611d830f6 | 01a11515-cdc7-7813-bb13-795536c30ce2 | AP-UX-008 |
| 01a11513-66a4-7c30-a318-bf4611d830f6 | 01a11518-dea5-7442-a140-7750e2886bd9 | AP-UX-018 |
| 01a114f7-6fad-7971-9d7d-f16db46896c4 | 01a114f7-7014-7830-8172-cc21add51a8e | AP-UX-019 |
| 01a114f7-6fad-7971-9d7d-f16db46896c4 | 01a114fa-0461-7270-a9a5-6c5875448a10 | AP-UX-011, AP-UX-023 |
| 01a114d4-c6a9-7030-a34a-e651c8f505e6 | 01a114d4-c6eb-71d0-8403-c8df6e36f309 | AP-UX-019 |
| 01a114d4-c6a9-7030-a34a-e651c8f505e6 | 01a114d6-4ae9-79a1-a78b-eb0040c85281 | AP-UX-020 |
| 01a114d4-c6a9-7030-a34a-e651c8f505e6 | 01a114d6-b525-71a0-8751-bd6c799845c2 | AP-UX-019 |
| 01a114d4-c6a9-7030-a34a-e651c8f505e6 | 01a114d8-6a57-7c21-a042-aeb586efa397 | AP-UX-020 |
| 01a114b0-49a3-7e90-b0bd-9ca83a0a7cc2 | 01a114b0-4a4f-7c62-b716-c2b3b97d4c7a | AP-UX-021 |
| 01a114b0-49a3-7e90-b0bd-9ca83a0a7cc2 | 01a114b1-e8c7-7603-866d-0a710e616903 | AP-UX-033 |

The two non-usability exclusions are explicit records, not unexplained gaps. Credential setup does not remove AP-UX-027 provider-policy acceptance. The four withdrawn references preserve AP-UX-030 and AP-UX-032 withdrawal strength. They require no new feature.

AP-UX-002, AP-UX-003 and AP-UX-010 have empty feedback_refs. The prior [intake review](rom-0.1.0-usability-intake-review.md) explicitly identifies them as source-derived candidates. They remain tracked requirements without invented direct user attribution. They do not cause any of the 47 fresh references to be unmatched.

## Acceptance remaining for all 33 groups

The current public component facade exports ResponsiveDetails, ReferencePicker and HistoryList. Headless UI helpers, messages and layout validation exist. No SelectionCard or LayoutControls export was present at the inspected cutoff. New work in progress does not establish acceptance. Application ownership keeps the requirement in the usability handoff; it does not discard it.

| Group | Current contribution and actionable acceptance remaining |
| --- | --- |
| AP-UX-001 Public shared controls | Public control/style installed admission and original consumer deployment; do not substitute a local source build. |
| AP-UX-002 Control styling ownership | Styled-node width, disabled, invalid and focus contract in an external composition. This group is source-derived, not directly attributed to a message. |
| AP-UX-003 Compact composer and form bindings | Compact composer focus, exact attributes and bounded form recipe in the actual consumer. Source-derived candidate. |
| AP-UX-004 Autosave and uncertain mutation recovery | Actual consumer autosave, same accepted command across lost acknowledgement, rejected/conflict/unknown presentation and owner privacy; latest-request is not mutation recovery. |
| AP-UX-005 Safe navigation and drafts | Actual new/switch/delete confirmation barriers and private state quarantine; HistoryList delegates navigation and does not authorize it. |
| AP-UX-006 Session refresh and view sequencing | Actual same-owner refresh, expiry clearing and transient single-flight recovery with retained view. |
| AP-UX-007 Actual work stages and durable progress | Owner-visible real work facts, retry timing, same-job refresh and raw-job denial; HistoryList and observation supply no durable stage presenter. |
| AP-UX-008 Immediate pending queue and resume | Immediate input plus durable queue/resume identity and explicit bounds; UI pending is not persisted work. |
| AP-UX-009 Long results and composer geometry | Actual unobstructed composer/history/answer geometry at 1280×900, 390×844 and 390×500. Details/history authoring tests cover only part. |
| AP-UX-010 Respect manual scroll | Current installed follow-latest and actual late-content/reading interactions; source-derived candidate. |
| AP-UX-011 Responsive panels, focus and reduced motion | Current installed details, actual consumer focus integration and manual screen-reader assessment; source-authoring details evidence exists. |
| AP-UX-012 Localization and structured error messages | History date/count/error locale and ReferencePicker host labels now have candidates. Active work/progress/error localization, persisted preference and complete stable catalogs remain required. |
| AP-UX-013 Stale cache and recoverable observation | Corrected observation handles reentrant and rejected host callbacks. Current installed transport, hidden/resume/private clearing and post-await host authority fences remain required. |
| AP-UX-014 Useful conversations and grounding | Application-owned useful ordinary conversation during research, with computed/cited/generated distinction; no UI helper acceptance substitutes for answer usefulness. |
| AP-UX-015 Lazy topic research and deduplication | Application-owned lazy, source-backed deduplicated research while conversation remains usable; disposable cancellation is not durable deduplication. |
| AP-UX-016 Domain tool transparency | Application/AI-owned actual executed-tool provenance and filtered private inputs; full tool work must not become unsupported-tool closure. |
| AP-UX-017 Selectable cards and reactive computation | Public accessible SelectionCard and held reactive calculation composition remain absent from inspected facade; exact selection/latest-result helpers alone are insufficient. |
| AP-UX-018 Dashboard widget layout and settings drawer | Corrected catalog-bound validator exists. LayoutControls, keyboard/mobile commands, principal-bound persistence, corrupt reload and actual consumer catalog acceptance remain open. |
| AP-UX-019 Chart terminology and dynamic label spacing | Application-owned translated chart spacing/terminology and SVG identity tests; generic component tests do not validate the chart. |
| AP-UX-020 Autocomplete with explicit manual fallback | Public ReferencePicker authoring proves exact ID, keyboard, manual fallback, bounds, locale labels and stale authority fencing. Current-source installed flow, original consumer autocomplete and coordinate adapter remain open. |
| AP-UX-021 Admin-only navigation and guest workflows | Actual guest compute/export without private persistence, admin absence backed by denial, and expired identity clearing. |
| AP-UX-022 Readable saved-history list | Corrected HistoryList authoring proves readable names, Intl metadata, host current marker, disabled uncertain writes and callback fences. Current-source installed navigation barrier and original saved-history consumer remain open. |
| AP-UX-023 Source links retain application context | Actual immutable citation identity, inline context/deep-link recipe, close focus and unsafe URL rejection in installed/original consumer. |
| AP-UX-024 Fresh execution versus retained result | Actual new-run identity versus stored reopen, deliberate rerun and visible provenance; request identity is not execution/receipt knowledge. |
| AP-UX-025 PDF/export from actual selected snapshot | Actual renderer output from displayed revision/date, busy/failure/retry and long translated content; recheck current authority before bytes/manual save. |
| AP-UX-026 Private observations/profile | Application-owned owner-private observation/profile edit/delete and separation from computed facts. |
| AP-UX-027 Model provider failover policy | Application policy plus generic AI routing: exact pending identity, bounded free/paid/local policy, durable attempts/deadline and actual failure recovery. |
| AP-UX-028 Theme and domain brand | Application-owned theme/brand and human contrast assessment; shared controls must preserve reduced motion and usable navigation. |
| AP-UX-029 Daily questions and relationship-aware readings | Application-owned daily freshness and relationship-aware computation identity; do not infer chart policy into ROM. |
| AP-UX-030 Video frame-rate request withdrawn | Explicit withdrawal retained: no 60 FPS video requirement added to ROM. |
| AP-UX-031 Date timeline and selected temporal context | Application-owned keyboard/touch temporal selection and export of that exact selected snapshot. |
| AP-UX-032 Music and sound request withdrawn | Explicit withdrawal retained: no music or sound requirement added. |
| AP-UX-033 Guest invitation and optional account save | Application-owned guest invitation without account, optional account save retaining draft/retry identity and one related Resource; no chart-specific ROM subsystem. |

## Current corrective evidence

[History/layout review](rom-0.1.0-history-layout-independent-review.md) records all getter, catalog, collection-bound and NaN callback findings resolved. The reviewer independently executed fourteen focused Node tests and corrective probes. The coordinator final evidence has nine matching source hashes, 407 unit passes, clean checking/build and fourteen browser passes. These are source-authoring tests. They do not establish original saved-history acceptance.

[Reference evidence](rom-0.1.0-reference-composition-evidence.md) records the public source form, stable renderer compatibility wrapper and shared bounded lookup contract. It passed eleven focused Node cases and fourteen public browser executions within an affected run of 117 passes and one existing WebKit touch skip. The form uses the public entry but is not an installed package consumer.

[Observation corrective review](rom-0.1.0-observation-corrective-review.md) independently records twenty maintained Node passes and eight reentry/thenable checks. Async failures are consumed without granting authority or fencing arbitrary host continuations. Current installed and actual transport integration remains a separate gate.

These specific improvements supersede absence statements in the older [UI gap report](rom-0.1.0-ui-feedback-gap-review.md) for history, reference and message contracts. That historical report remains intact. Its unresolved actual-consumer, geometry, export, persistence and human-acceptance requirements still apply.

## Coordination and acceptance boundary

The fresh sync explicitly records consumer_acknowledgement=false. A shared repository handoff and messages to ROM workers demonstrate producer coordination. They are not acknowledgement from the original Astral Plane agent or owner acceptance of deployed behavior.

Complete the current-source installed public composition, actual guest/private authority/navigation/export journeys, and original consumer retest. Obtain original-agent feedback and the held-out human/screen-reader assessment. Keep application-owned groups and the two explicit withdrawals in that handoff. Do not replace the full R14 tool and public-flow contract with unsupported-tool handling.

This audit ran no browser, native build, deployment or acceptance test. It establishes zero unmatched references in the fresh page and preserves every group's remaining acceptance. It does not declare release readiness.
