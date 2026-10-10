# Independent R3 review

Date: 2026-10-07. Reviewed root-owned notice changes against producer HEAD `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
Read the working diff, ownership implementation, collection, CSS source ownership, inventory admission, and all notice tests.
No source edits, native builds, production access, or broad installation admission were performed.

## Result

No must-fix implementation defect was found in the requested whole-directory shared-installation change.
The implementation retains one configured physical dependency tree and portable logical inventory paths.
It does not establish arbitrary package-link or pnpm support.

## Concrete checks

- `packageOwner` resolves the configured `node_modules` and owner directory before checking containment. An individually linked package outside that tree cannot receive an npm owner through this path.
- `moduleOwner` maps lexical configured-installation paths into that physical tree. Canonical emitted paths within the same tree produce `node_modules/...` inventory IDs.
- The query suffix is detached before path operations and appended to the portable ID unchanged. The new positive test checks `?transformed` and excludes the physical shared path from inventory JSON.
- Existing package-name/version admission remains at the owner boundary. Resolved directory identity must still match the manifest's package name.
- License enumeration and byte retention remain unchanged. Missing licenses, invalid package identity, unknown external modules, virtual modules, and malformed artifact inventories retain failures.
- Tool-runtime ownership resolves Vite/rolldown through the configured tree. Generated Tailwind and imported CSS still use packageOwner. Their logical source IDs remain portable.
- The nested synthetic workspace fixes the earlier fixture ambiguity. A sibling of a temporary Studio root under `/tmp` was previously inside the permitted workspace fallback. The new unrelated shared installation is outside the fixture's actual workspace.

The workspace fallback remains lexical and accepts first-party workspace modules as before. This patch does not turn that fallback into package admission.
A transformed module's suffix is retained as text; it does not influence physical containment or package lookup.

## Assurance gaps worth closing

The new negative cases exercise unrelated physical modules, but not an individual package-directory symlink outside the configured installation.
Source inspection indicates rejection through packageOwner's canonical containment check. Add that exact negative fixture if the release wants executable proof against accidentally broadening symlink admission.
Exercise both lexical and canonical emitted IDs for that rejected package link. Require a classified rejection, not merely any filesystem error.

The shared-install positive case uses one unscoped package. Scoped/nested packages and shared-install imported CSS are currently source-backed rather than specifically combined in this regression.
These are useful focused additions if those layouts are advertised. They do not justify expanding the supported package-manager matrix.

The implementation assumes the configured installation remains unchanged during collection. Concurrent installation replacement is not tested or claimed.
A full build should freeze its dependency tree rather than infer transactional filesystem guarantees from this patch.

## Executed evidence

Executed `node --test scripts/packages/studio-notices.test.mjs` locally with Node `v22.16.0`.
Result: 12 tests passed, 0 failed. This independent run reproduced the narrow notice suite only.
The coordinator's 58-test package/release result was reported, not rerun by this reviewer.
No full source verifier or actual Vite build was performed for this review.

Reviewed hashes:

- `studio/build/notices/ownership.mjs`: `6eb08f4611214b7790bb1712e4fe1610426c6e48fb992789b14325c1fb3836f9`.
- `scripts/packages/studio-notices.test.mjs`: `558da4eac0301dd49176cb6bd0f3d4546a68bdd4b0e682655b0be1aadf7e96ef`.

The review supports integration of this narrow candidate with the stated assurance gaps visible.
It does not establish packaged-consumer, production-deployment, or complete 0.1.0 acceptance.
