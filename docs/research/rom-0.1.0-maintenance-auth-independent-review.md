# Independent maintenance HTTP and legacy denial review

Date: 2026-10-08. Scope: repository-source maintenance fixture and legacy auth denial classification. No remaining actionable implementation defect was reproduced.

This review does not establish release admission or original Astral Plane adoption. Production identity and human acceptance remain open.

## Maintenance HTTP candidate

Reviewed the four public Resource declarations, shared owner policy, HTTP journey and fixture transport. Reviewed the [coordinator report](rom-0.1.0-maintenance-http-2026-10-08.md) and `/var/tmp/rom-010-maintenance-http-evidence.json`.

All twelve source/manifest hashes match that evidence file. Both test and Clippy log hashes also match. The corrected HTTP test hash is `10611bbd670c679dd170ff8de4faff49355a078782a69c814a9cd20af57ddcfe`. Transport hash is `1a2d5d85c1d1ff242bf90b1fd67e904427ec8938f4f0f0c43fbee929d7b5b023`.

The original query variable shadowed the database factory. The corrected source uses `open_query`. It now checks authenticated Bob denial after Alice's receipt exists. It separately disables Alice's fixture credential before replay.

The bounded weak-reference wait occurs after server completion and the test's storage drop, before reopening the database. This explicitly checks storage handle release. It addresses the possible retained shutdown clone identified in the first review.

The test repeats an identical invocation before and after reopening. Projected JSON remains equal and the WorkOrder journal remains at two events. Reopened Settings retain revision two, columns three and hidden history. Alice's permitted read also checks that the denied transfer did not change ownership.

Invalid date rejection checks HTTP 400 and unchanged inspection event count. The report correctly avoids claiming separate absence of an invalid row or receipt. It also distinguishes post-receipt ownership denial from a same-principal policy-change experiment.

The response reader has a three-second deadline and a 131072-byte bound. It assumes this fixture's unchunked, close-delimited JSON. Fixed bearer credentials and an ignored forged subject header are fixture authentication, not production credential verification.

The coordinator's logs record six application test passes and clean Clippy with warnings denied. The two HTTP journeys use SQLite and redb. These native commands were not independently rerun by this reviewer because another native build lease was active.

## Legacy denial classification

Reviewed `legacy-adapter.ts`, its application facade, public `/auth` facade, shared transport/parser and seventeen focused tests. `SessionDeniedError` extends `Error` directly. It does not extend `SessionExpiredError` and does not fabricate anonymous generation.

Only validated 401/403 responses with `error: "denied"` produce the denied protocol outcome. The shared transport clears local credentials before the adapter throws the new typed error. A temporary 503 remains a generic unavailable error and retains credentials until their original expiry. Actual expiry still throws the existing `SessionExpiredError` class.

Both facades export the same class identity. Public paths remain stable. The change adds a named export and changes confirmed denial from a generic unavailable error into an explicit authority-loss error.

Independently executed from `/root/ROM/studio` with Node `v22.16.0`:

```sh
node --experimental-strip-types --test tests/unit/auth-legacy-adapter.test.ts
```

All seventeen tests passed, with zero failures or skips. Additional inline assertions passed for shared export identity, distinct denial/expiry classes, and malformed 401/403 bodies remaining transient. The initial inline probe omitted JSON Content-Type and failed protocol validation; correcting the probe header made it pass. No product change was needed.

The coordinator's preserved RED log records two denial failures and one passing 503 control. Its public GREEN log records seventeen passes. Full Studio units record 417 passes, and Svelte checking records zero errors and warnings. Those broader checks were inspected, not rerun here.

Logs use `/var/tmp/rom-010-legacy-authority-loss-` with suffixes `red.log`, `public-green.log`, `studio-units.log` and `studio-check.log`.

| Current reviewed file | SHA-256 |
| --- | --- |
| studio/src/lib/auth/legacy-adapter.ts | `3801adc41cb3c0b9db8df8c7d7141263c740ed8d91785e52efd64a1a1bf8ab6a` |
| studio/src/lib/application/auth.ts | `cb52f39cd5e4500ec142174f8153e42181112aa2267e16222ac2f033a234cb6f` |
| studio/src/auth.ts | `9a5520c2dd8cdb97c9a5e566dfc13f5969595258c5825b8d0e4ad4af17238640` |
| studio/tests/unit/auth-legacy-adapter.test.ts | `23d2442249a0c90c30fb3ac7332ea44551ab2df7727793612b2326c51c469a79` |
| studio/src/lib/auth/browser-transport.ts | `03a035a540b21a05c16c2f6991f7c6814437e96c763720b6c7524a088ea0f201` |
| studio/src/lib/auth/browser-response.ts | `0df55773b4348c9fa339a861b548dd0134f8a1c90288551c80101afc627ac1b6` |
| studio/package-lock.json | `11e7630aa0293c7360838d41bb07bb40a52940f80b4938063a8d0eb9465ce5dc` |

## Compatibility and acceptance limits

Consumers that treated every generic refresh failure as temporary must handle `SessionDeniedError` explicitly and clear private disclosure. Consumers must not label denial as expiry. Code that matches the old error message can change behavior.

The repository's legacy App catch disconnects on every error, including denial. Only `SessionExpiredError` selects its expiry/provider branch. This helper change does not itself add denial-specific login UI or establish another application's handling.

Original Astral Plane adoption remains open. An installed consumer must use the same admitted package class identity for `instanceof` checks. Multiple package copies can invalidate that assumption. No installed-source, browser, real authentication or held-out human journey was executed in this review.

Only this new report was written. No product edits, dependency changes, native execution or commits were made.
