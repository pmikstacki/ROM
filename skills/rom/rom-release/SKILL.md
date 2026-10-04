---
name: rom-release
description: Use when preparing local ROM source artifacts, verifying release readiness, or reviewing compatibility and package evidence before integration.
---

# Verify local release evidence

Read the [native contract](../../../docs/native-extensions.md) before assessing version and support claims.
Use the [bundle procedure](../README.md) to run `release` preflight and its [selected-check example](../assets/release/run.mjs).
That example executes a finite subset and produces evidence. It does not approve a release.

For a release-readiness request, use the explicit full source checkout supplied as `ROM_ROOT`.
Read `ROM_ROOT/scripts/release-artifacts/README.md`. That file defines the current complete gate sequence and prerequisites.
The instruction bundle does not contain the complete producer or frontend inputs.

1. Record the exact source revision, tree, and Cargo.lock identity.
2. Verify the artifact procedure's toolchain, dependency caches, browser runtimes, and provider prerequisites.
3. Complete independent review and compatibility notes against that source identity.
4. Run `./scripts/release` from the clean supplied checkout, with an exclusive output directory.
5. Run artifact verification against the completed output as specified in the artifact procedure.
6. Compare the manifest's source, gate evidence, extracted assets, and package identities with the review inputs.
7. Record failed, inspected-only, pending, and passed checks separately.
8. Complete the requirement audit before claiming release readiness.

For the Studio profile, completion includes both actual browser runtimes and acceptance of the freshly extracted production assets.
The selected skill example and earlier manual native checks do not establish those results.
If a prerequisite or gate fails, retain its evidence and repair the cause before completing the release.

Keep the original independent Field registry proposal open as the guide specifies.
These steps prepare local artifacts. Follow the owner's existing authorization for source publication and preview deployment.
Completion requires the full applicable gates and final artifact identity, rather than the selected example alone.
