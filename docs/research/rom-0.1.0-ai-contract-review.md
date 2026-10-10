# AI neutral contract candidate review

Date: 2026-10-07. Reviewer: ROM release coordinator.
Scope: neutral request, routing, deadline, and prepared-attempt source. This is not complete AI flow acceptance.

The review found that a decoded route could select an external model or reduce its reservation estimate.
The original constructor did not connect the route to the frozen policy and request.
The worker added behavioral regressions before the correction. The RED result retained 13 passes and two failures.

Construction and explicit post-decode validation now require the selected candidate, matching policy/cursor tier and version, and the exact request ceiling.
The paid path requires a budget reference. The deadline must have positive remaining time within expiry and the configured age bound.
The review inspected the corrected provider, selection, cursor, clock, and adversarial test source.
Worker execution logs record 15 passing routing tests and targeted Clippy under Rust 1.99.

Evidence: `/var/tmp/rom-ai-task1-prepared-red.log` and `/var/tmp/rom-ai-task1-prepared-green-clippy.log`.
The worker records frozen Task 1 aggregate source hash `0f5a48a33b9b5c474a3287ac34808fd8b28dfd1af5b6bdee2dec8a79a98bfec7`.
The coordinator inspected these logs but did not independently rerun this expanded native suite during the other worker's build allocation.

The correction permits Task 2 implementation. It does not authorize external dispatch.
Serde decoding alone grants no permission. Current time, catalog capabilities, Resource authority, and actual budget reservations remain dispatch checks.
The actual SQLite/redb reservation tests, supervised flow, provider adapter, tools, consumer acceptance, and full verifier remain open.
Local CPU provider composition also needs separate acceptance. A generic Provider trait alone does not prove that fallback works.

## Conservative reservation candidate

The coordinator inspected the Task 2 run, account, reservation, codec, and transition source.
Each reservation retains the complete validated prepared attempt. Account updates use current revisions and bounded, checked totals.
Unknown evidence retains the worst-case charge, including after run expiry. Confirmed settlement cannot replace previously known usage.
Private Resources require the configured service identity. Serialized owners and reservation references do not grant execution authority.

The complete local verifier independently ran seven reservation tests and 15 routing tests after the historical migration fixture correction.
The command exited zero. Evidence is `/var/tmp/rom-010-integrated-candidate-check-after-legacy-20261007.log`.
Its source comparison and limits are `/var/tmp/rom-010-integrated-candidate-after-legacy.json`.
This dirty candidate is not a clean artifact or production-provider acceptance.

Two contract gaps remain: changed reservation identity reports Conflict, and direct frozen-policy amplification needs an adversarial regression.
The worker now owns those corrections and the remaining pure checkpoint, cancellation, tick, and projection contracts.
Waiting must commit a delayed next intent atomically. It cannot introduce a second scheduler or require routine consumer polling.
No provider dispatch is authorized by this review. Task 3 must verify the exact committed account reservation before executing.
The public facade, tools, provider adapter, external consumers, and production acceptance remain open.

## Expanded Task 2 independent integration review

The gaps identified above now have regressions and corrections in the expanded candidate.
Changed reservation identity returns `IdentityMismatch`. Decoded and patch-based policy amplification is rejected without commit or charge changes.
The coordinator inspected checkpoint, cancellation, delayed scheduling, evidence merging, and authorized projection source.
Confirmed nonacceptance may schedule a bounded delayed retry. An unresolved outcome cannot use that transition.
Known attempt identifiers and usage cannot be silently replaced. Cancelled runs retain accounting evidence and hide late output.

The complete local verifier independently ran all 14 reservation tests and 15 routing tests, plus the workspace checks.
It exited zero in `/var/tmp/rom-010-integrated-auth-checkpoint-check-20261007.log`.
The worker's frozen source manifest is `/var/tmp/rom-ai-task2-checkpoint-source-hashes.txt`.
Its aggregate SHA256 is `6e6f3fdf92b4753fa9abe08b387ec069e7bfdb41df32851ac03fb7c7381ff56f`.
This candidate remains uncommitted source, not clean release admission or provider acceptance.

Task 3 may implement the planned Work composition under separate ownership.
It must check current authority and the exact committed reservation before dispatch.
No started unknown request may be blindly sent again. Provider adapters, tools, consumer acceptance, and production checks remain open.

## Task 3 revised candidate check

The coordinator verified the frozen inventory in `/var/tmp/rom-ai-task3-race-source-hashes.txt` against current files.
Independent execution passed 30 flow cases, 14 reservation cases, and 15 routing cases in the development container.
Evidence: `/var/tmp/rom-010-root-ai-task3-race-review-corrected.log`.
The initial command used a nonexistent `budget_reservations` test target and failed before running tests.
That command failure remains in `/var/tmp/rom-010-root-ai-task3-race-review.log`.

Source inspection covered `committed_attempt`, `hold_unknown`, and `checkpoint_rate_limit` in the frozen worker.
These helpers check the exact prepared attempt and reservation through current Resource reads.
They use at most one CAS retry after a concurrent operation stamp. They do not repeat provider execution.
Malformed, foreign-attempt, and unsupported tool completions enter conservative recovery without accepting supplied usage or evidence.
The executed regression cases include the reported cancellation and reconciliation races on both supported databases.

This is a focused source and test review. It is not acceptance of the complete AI module or release.
The full verifier, OpenRouter adapter, tool execution, external consumers, and original application acceptance remain open.
Provider error classification must still prove nonacceptance before fallback or budget release.
