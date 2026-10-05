# Independent external-author correction review

Review date: 2026-10-05.
Scope: `examples/studio-consumer/tests/author.spec.ts`, the example README addition, and the release acceptance status addition.
Result: no material finding in the narrow correction.
Reviewed test SHA-256: `2c4a5ff7b0107494bdae6551cf40f4193f866a555fcf69abf262a6e1ba9ea55c`.

This review inspected source, diffs, and preserved execution logs.
It did not edit repository files or run builds, browsers, or tests.
The corrected full independent copy/extract/type/build/browser workflow completed before the final review was frozen.
The complete release producer and live preview remain pending.

## Test contract is retained

The public author entry point already registers `demo-ticket-code` version one with `layout: "inline"`.
It imports App and registerRenderer from `rom-studio`, not private implementation paths.
AuthorCode imports the public Input and RendererProps contracts.
The correction does not change this composition, the SDK, runtime, accepted codec, authentication, or host fixture.

The obsolete `Edit code` dialog click is replaced with an assertion that the named `Author code editor` is visible.
The same input still receives `ticket-author2`.
The ordinary `Save 1 change` button still submits the form.
The test still requires the exact canonical display `Author code: TICKET-AUTHOR2` after Save.
The previous dialog Escape operation is removed because no dialog exists for this registered inline scalar editor.
This removal does not remove a mutation or canonical-value assertion.

The revision check still requires revision two.
It now finds the exact `maintenance-tickets · Revision 2` text within the selected `Resource details` region.
ResourceSummary renders that combined type/revision text.
This selector is narrower than the old global standalone `Revision 2` lookup and retains the same revision expectation.

The ordinary Task flow remains unchanged.
The test selects tasks, opens task-a, edits the title through the generic control, saves one change, and checks its exact displayed title.
Actual OIDC login and fixture shutdown remain unchanged.
The same test continues to execute SQLite and redb under Chromium and WebKit.
No timeout increase, conditional bypass, catch-and-pass, skip, or fixture-response substitution was added.

The test proves the maintained public custom composition and ordinary mutation presentation path under its existing scope.
It does not add a fresh server-side canonicalization proof from an unnormalized raw invocation or a reopen durability assertion.
Those were not part of the replaced selectors, and their absence is not a weakening introduced by this correction.

## Preserved execution evidence

Logs are retained under `/var/tmp/rom-003-external-author-ergonomics-fix/`.
`red.log` records the expected failure waiting for the obsolete `Edit code` button.
`inline.log` records the later failure waiting for obsolete standalone `Revision 2` after the custom value had already changed.
`summary.log` records the corrected focused Chromium SQLite case passing.
`green.log` records four passes in 6.3 seconds across both databases and both browser engines.
These are inspected execution records from the implementing agent, not independent executions by this reviewer.

The producer at `a7b48ff` passed gates one through seven and its 28 standard actual-host cases before the independent external-author workflow failed.
The author fixture was therefore a real delivery acceptance defect, not justification to omit that workflow.
The corrected complete copy/extract/type/build/browser verifier has now finished successfully, as recorded below.
After that, the fresh complete producer and independent artifact verification remain required.

## Documentation and prior audit disposition

The README addition accurately describes the already-selected inline layout, direct shared form control, ordinary Save path, canonical display, and revision update.
It does not imply a new npm-distributed package or human usability evaluation.
The release acceptance document still marks clean-source distribution and persistent preview pending.
It expressly records the producer's external-author selector failure and requires the corrected source-extraction workflow before another producer run.
It does not declare the release complete.

This finding supersedes the earlier audit's statement that no further implementation or maintained-example delivery gap had been observed.
It does not invalidate the exact-identity bridge for the native/application inputs used in the earlier 28-case host acceptance.
The separate external-author test source changed and requires its own executed acceptance; the bridge cannot carry an obsolete test selector as a passing result.

Task 2.5 remains pending final packaged custom-renderer acceptance until the corrected independent author workflow and final artifact gates pass.
Task 5.6 remains pending complete production, independent manifest/archive verification, and live current-authentication/restart preview acceptance.
The four focused passes do not close those remaining gates or the owner's release goal.

## Frozen final verification evidence

The final result at `/var/tmp/rom-003-external-author-ergonomics-fix/full-verifier/result.json` reports `completed: true`.
All five recorded commands exited zero without timeout or spawn failure: offline consumer install, type check, asset build, offline provider-fixture install, and the browser workflow.
Its author test digest matches the reviewed `2c4a5ff7...` source.
The browser log records four passing cases across Chromium/WebKit and SQLite/redb.
The result identifies the extracted consumer directory, source archive digest, author source hashes, and accepted native binary digest.
This closes the focused source-extraction author workflow defect for those recorded inputs.

The root reports `./scripts/check` exited zero; this reviewer inspected its final log at `/var/tmp/rom-003-external-author-full-check.log`.
This is inspected root execution evidence, not an independently run verifier.
No material finding remains in the narrow correction.
The corrected source still requires a fresh complete producer, independent final artifact verification, and live preview/current-authentication/restart acceptance.
No completed-release declaration follows from the focused extracted workflow.

Review frozen after these final records were inspected. Only this `/var/tmp` report changed.
