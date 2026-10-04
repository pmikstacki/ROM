# Final Studio slice review

Date: 2026-10-04 UTC. This is an independent source and evidence review of the named slices.
No native build, provider journey or browser gate was run by this reviewer.
Two finite Node mechanism probes ran without changing maintained source or another worker's extraction.

The review applies [the quality gates](../quality.md), [Studio design](../superpowers/specs/2026-10-03-rom-0.0.2-studio-design.md)
and [implementation plan](../superpowers/plans/2026-10-03-rom-0.0.2-studio.md).
The full producer and persistent preview remain separate acceptance obligations.

## Scope and verdict

| Slice | Reviewed boundary | Verdict |
| --- | --- | --- |
| Actual provider through maintenance | Commit `f88964f`, `crates/rom-studio-host/tests/support/maintenance.rs`, its human helpers, manifest edge and recorded run | No behavioral or standards blocker found in this scoped journey. |
| Inventory restock | Commit `29f9b68`, `demo/src/{studio_model,studio_application,studio_model_tests}.rs`, facade declaration and native/browser records | No behavioral blocker found. The unused test import observed in the first run has been removed. Final strict combined checks remain required. |
| External Studio author | `studio/src/index.ts`, `examples/studio-consumer/`, `scripts/research/verify-studio-consumer.mjs`, `demo/verify-studio` | Accepted in this scope. All three verifier findings are repaired and bound to the final source and evidence. |

## Provider maintenance

The [new journey](../../crates/rom-studio-host/tests/support/maintenance.rs:282) uses actual provider-generated authorization codes and ID tokens.
It reuses the approved code exchange, sealed proof, captured Provider activation and current identity gate.
The HTTP helper follows provider login and consent. It does not supply an Actor or fabricate an authenticated session.

The same test executes SQLite and redb within explicit journey deadlines.
It creates an ordinary Resource through generic HTTP, rejects migration while the native source is owned, then stops serving.
Backup, restore, schema migration and a new disk open preserve the recorded Resource/event/receipt/effect counts.
The migrated event contains the converted boolean false value.
The source database bytes remain unchanged.

The restarted host rejects the old cookie and CSRF request before receipt disclosure.
A fresh actual provider login establishes the same managed User with a different session generation.
Replaying the original request returns its converted historical value without another event, receipt or effect.
Disabling the current identity link then denies that same replay without further writes.
These checks exercise current authority rather than treating receipt possession as authorization.

The [author result](rom-0.0.2-provider-maintenance-journey.md) and [passing log](evidence/rom-0.0.2/task4/maintenance-provider-final.log)
match the source assertions.
The earlier expected-403 failure was a fixture-contract error: current identity rejection occurs before generic invocation and returns 401.
The report attributes that correction accurately.

This is an actual provider and HTTP-session maintenance journey. It is not rendered-browser, provider-durability or power-loss evidence.
The independent seeded operator migration suite retains its own scope; this test does not replace it.

## Restock action and ordinary authority

[RESTOCK](../../demo/src/studio_model.rs:81) is an ordinary typed `Action<InventoryItem, u64>`.
Its checked addition completes before assignment. Overflow returns an invalid-field error without a proposal value or external effect.
The [registration](../../demo/src/studio_application.rs) adds the action to the existing Resource definition and unchanged domain policy.
It adds no per-kind route, alternate mutation handler, trusted browser Actor or duplicate receipt implementation.

The existing Runtime invocation path checks current identity before receipt interpretation and again around disclosure and commit.
The Studio transport still resolves current managed sessions before reaching that path.
This review found no authority shortcut in the new action or registration.

The [native test](../../demo/src/studio_model_tests.rs:6) discovers the action, creates quantity `u64::MAX - 1`, and restocks once.
The returned row reaches `u64::MAX` at revision two.
A new overflowing request leaves Resource, event, receipt and effect counts unchanged.
Exact replay of the earlier request returns the same outcome without another count change.
Its host actor is explicitly trusted test configuration; that case alone does not establish human authentication.

The [RED log](evidence/rom-0.0.2/inventory-action/red.log) fails because the action was absent from Resource metadata.
The [GREEN/build log](evidence/rom-0.0.2/inventory-action/green-build.log) records the passing native case and executable build.
That log also reports an unused `Field` import at test line two.
The live corrected test removes that import. The correction requires the final warning-denied gate; no such rerun was performed by this reviewer.

The later [actual workflow evidence](evidence/rom-0.0.2/full-workflows/observer-only-results.log)
uses the immutable action-enabled binary on both stores and browser engines.
It is separate evidence from the trusted native unit test.

## Public author boundary

The [public Studio entry](../../studio/src/index.ts) contains exports only.
It exposes the generic App/client/types, renderer registration and shared Input without implementing another protocol.
The author's `main.ts` and `AuthorCode.svelte` import only that entry and the public stylesheet.
The renderer selects by exact codec identity, not Resource kind.
It uses the shared callback and native-authoritative codec path.

The author config resolves the public entry inside an independent extraction.
Its dependency symlink points to the copied consumer dependency graph within that extraction.
The trusted test fixture imports the copied host/provider harness separately from the application's public imports.
The example therefore does not claim an npm registry package or a newly minimized dependency set.

The verifier reuses the existing complete Studio inventory, archive extraction and exact offline lock contracts.
It checks source and extracted Studio identities before and after acceptance.
It requires an explicit immutable native executable and checks its hash after the browser run.
The fresh-build integration additionally copies the accepted executable into the gate directory with mode 0555.
The production gate command and manifest gate count remain unchanged.
The combined fresh-native-build integration has not been executed after this script change.

The [author report](rom-0.0.2-studio-author-workflow.md) and retained result distinguish the earlier trial from the corrected supervised run.
The corrected run records four passing real OIDC/browser/store cases, zero type diagnostics and successful offline installs/build.
These are agent-executed source-consumer checks, not human-usability evidence.

This reviewer independently compared the recorded inventories with the current Studio source, original author inputs and retained extracted copies.
All five checks passed, including the immutable binary hash.
The [identity record](evidence/rom-0.0.2/final-slices-review/final-identity-check.json) preserves those results.
The [reviewed input manifest](evidence/rom-0.0.2/final-slices-review/source.json) records the exact source and evidence scope.

## Findings and probes

| Priority | Finding | Status |
| --- | --- | --- |
| P2 | The original `spawnSync` timeout killed the direct command but left descendants alive. | Repaired: npm/type/build/browser commands await the existing Linux process-group supervisor. The author records four passing cases with that correction and two passing supervisor regressions. |
| P2 | The original verifier checked the original author inventory without checking the copied inputs before/after execution. | Repaired: both copied-input checks use the same recorded inventory. Generated manifest/lock files remain outside the selected author-input set. |
| P2 | The original standalone WebKit fallback used direct `execFileSync` supervision for a helper that can spawn child commands. | Repaired: the fallback awaits the same bounded process runner. The final result records its successful execution. |
| P3 | The restock test retained an unused import in the passing author log. | Corrected in source. Final strict native verification remains a coordinator obligation. |

The [process probe](evidence/rom-0.0.2/final-slices-review/direct-child-ownership.log)
uses the same `spawnSync` mechanism with a 150-ms parent timeout and a finite 1,200-ms descendant.
At the timeout return, the parent is terminated while the descendant remains sleeping.
After its natural deadline the descendant is absent.
No product process, native executable, credentials or unrelated process was involved.
The probe establishes missing descendant ownership; it does not claim that the parent return itself blocked.

The [copied-input probe](evidence/rom-0.0.2/final-slices-review/copied-author-binding.log)
uses the actual shared inventory helper on an independently copied author fixture.
After a copied file changes, the original-author fence still passes while copied identity differs.
It changes neither maintained source nor the author's retained extraction.
This is a binding-mechanism probe, not an executed full-verifier false-success claim.

The author subsequently ran a complete [copied-source drift control](evidence/rom-0.0.2/studio-consumer/copied-source-drift-control.log).
It changed only the retained extracted `AuthorCode.svelte` after asset build.
All four browser cases passed, but final admission rejected the changed copied source with exit 1.
The final positive run used the repaired verifier and passed all commands.
This review inspected those records; it did not repeat either browser run.

## Parser timing deviation

The original Task 2 requirement called for a wire-fixture benchmark before parser adoption.
The [measurement report](rom-0.0.2-wire-parser-results.md) explicitly records that the benchmark occurred after adoption.
That chronology cannot be fulfilled retrospectively.

The report retains fixture, binary and parser identities, finite timing methodology, measured overhead and the comparison with built-in JSON operations.
It preserves separate correctness obligations and makes no maximum-payload, browser, heap or universal performance claim.
An explicit plan refinement can accept this recorded ordering deviation without declaring the original ordering fulfilled.
Preserve the original requirement and evidence. Require a comparative benchmark before any parser replacement.
Final source acceptance remains required.

## Completion boundary

Native maintenance, restock and the external consumer have no open findings in this inspected scope.
All three external verifier findings are repaired and supported by current source, successful acceptance and the negative control.
Final combined warning-denied checks, clean producer execution and persistent preview acceptance remain pending.
