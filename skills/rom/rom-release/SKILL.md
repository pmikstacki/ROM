---
name: rom-release
description: Use when preparing local ROM source artifacts, verifying release readiness, or reviewing compatibility and package evidence before integration.
---

# Verify local release evidence

Read the [native contract](../../../docs/native-extensions.md) before assessing version and support claims.
Use the [bundle procedure](../README.md) to run `release` preflight and its [selected-check example](../assets/release/run.mjs).
That example executes a finite subset and produces evidence. It does not approve a release.

For a release-readiness request:

1. Record the exact source state and Cargo.lock identity.
2. Run `./scripts/check` from the supplied checkout.
3. Run `./demo/verify` and `./demo/verify-provider` with their documented local prerequisites.
4. Run `./scripts/check-skills` against the final source.
5. Run `node scripts/check-packages.mjs` with that checkout as its explicit argument.
6. Prepare the local source artifacts and compatibility and support notes.
7. Compare the artifact identity with the executed checks and independent review inputs.
8. Record failed, inspected-only, pending, and passed gates separately.
9. Complete the requirement audit before claiming release readiness.

Keep the original independent Field registry proposal open as the guide specifies.
These steps prepare local artifacts; publication and external credential acquisition need separate authorization.
Completion requires the full applicable gates and final artifact identity, rather than the selected example alone.
