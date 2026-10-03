# Final source release implementation plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` for separately owned implementation and review tasks.

**Goal:** Complete release tasks 4.5 and 4.6 with current application acceptance, local artifacts, support notes, and a requirement audit.

**Architecture:** Reuse the public application and package verifier. Keep host lifecycle and artifact assembly separate from core behavior.

**Tech stack:** Rust 1.99, the existing native container and dependencies, Node.js, shell scripts, and local Git.

**Spec:** [Final release acceptance design](../specs/2026-10-03-release-acceptance-design.md).

## Constraints

- Complete and review native conformance before final acceptance.
- Preserve native format 8, archive format 6, public APIs, and disabled publication.
- Use named modules and facade-only roots. Share existing helpers where their contracts agree.
- Retain active builds and evidence. Reuse the shared Cargo target with two build jobs.
- Assign separate file ownership before parallel implementation.
- Use STE guidance with the documented fallback and preserve technical meaning.

## Review focus

1. Readiness before signal registration can cause abrupt exit. Test immediate SIGINT and SIGTERM with bounded child ownership.
2. A reconstructed Cargo manifest can lose optional or development dependencies. Compile and test the copied application with fresh archives.
3. A package test can resolve checkout paths. Audit resolved ROM manifests after application execution.
4. Failed release commands can leave misleading artifacts. Stage outputs and reject existing identities without overwriting them.
5. Historical reports can imply unsupported guarantees. Audit each current claim against its exact evidence and limits.

## Task 1: Host signal lifecycle

**Files:** `demo/src/main.rs`, named host signal/server modules, `demo/src/provider_profile/serving.rs`, and a bounded demo process regression.
**Consumes:** Existing Runtime, blob shutdown, provider serving, and Tokio Unix signals.
**Produces:** A shared host receiver registered before readiness in both serving modes.

- [x] Add an immediate-readiness SIGINT/SIGTERM regression and preserve the pre-fix result.
- [x] Extract shared signal reception and synthetic serving into named host modules.
- [x] Run focused default/provider process tests and obtain independent review.

## Task 2: Packaged application acceptance

**Files:** `scripts/check-packages.mjs` and cohesive helper modules under `scripts/packages/` where needed.
**Consumes:** Cargo metadata, fresh library archives, and the maintained demo source/tests.
**Produces:** An external application that runs reference, operator, upgrade, process-exit, attachment, and authority tests against extracted ROM libraries.

- [x] Preserve manifest dependency kinds, optional flags, features, and explicit package identity in one shared renderer.
- [x] Copy only the application files and shared test support that its tests need.
- [x] Run application tests and audit all resolved ROM library paths against the extraction directory.
- [x] Preserve the public consumer, CLI, license, and repository lock checks.
- [ ] Obtain independent review and run the combined source/demo/provider/skills/package gate.

## Task 3: Artifacts and support

**Files:** `scripts/release`, named artifact helpers/tests, current support/release documentation, README pointers, and the release completion audit.
**Consumes:** Clean accepted source, source/lock identity, native profile, and skill assembler.
**Produces:** An exclusive local source archive and skill bundle with a checksum/verification manifest.

- [x] Test dirty-source, existing-output, incomplete-output, and archive identity handling with isolated fixtures.
- [ ] Assemble local artifacts after complete verification. Do not publish.
- [ ] Verify source extraction, skill admission, checksums, and the exact committed source identity.
- [ ] Publish the support matrix and requirement-by-requirement evidence audit.
- [ ] Review documentation and artifact logic independently, then mark tasks 4.5 and 4.6 complete on evidence.
- [ ] Integrate the reviewed changes and record the generated artifact paths and remaining scope boundaries.
