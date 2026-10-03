# Native conformance implementation plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` for the assigned independent tasks. Preserve the ownership boundaries below.

**Goal:** Deliver the native alpha compatibility profile, reusable conformance assertions, and four executable author skills.

**Architecture:** A development-only crate extracts existing assertions. Skills consume the same public APIs and canonical profile, with portable generated assets.

**Tech stack:** Rust 1.99, existing workspace dependencies, Node.js for local packaging, and Markdown skill sources.

**Spec:** [Native extension conformance design](../specs/2026-10-03-native-conformance-design.md).

## Global constraints

- Finish and integrate the current module cleanup before editing dependent sources.
- Preserve Resource-owned codec versions, native format 8, and archive format 6.
- Keep the broader independent Field identity/registry proposal open and explicitly documented.
- Core and production adapters must not depend on conformance or skill packages.
- Use named modules and facade-only `lib.rs`/`mod.rs`. Preserve existing public paths.
- Use the existing container, shared target, two build jobs, and no debug information.
- Keep publication disabled. These tasks produce local and source-distributed artifacts.
- Coordinator owns root workspace/lock changes, shared verification scripts, package integration, and final evidence.

## Review focus

1. A copied assertion can diverge. Task 1 must replace original bodies with calls to the extracted profile.
2. A factory can reopen while an owner survives. Task 1 must release its handles before reopen and test a broken fixture.
3. A Field helper can assume input equals canonical output. Task 1 uses explicit input, canonical value, and expected typed value.
4. A skill can work only in the original checkout. Tasks 2–3 verify extracted references and explicitly supplied source roots.
5. A package can compile from workspace paths accidentally. Task 3 audits dependency paths in the extracted consumer.

## Task 1: Profile and shared conformance

**Owner:** assigned after cleanup review.
**Files:** `crates/rom-conformance/`; original persistence, blob, and consumer assertion call sites; `docs/native-extensions.md`; `extensions/native-alpha-v1.json`.
**Consumes:** Current public Field, Storage, BlobStore, and version constants.
**Produces:** The exact profile, codec, storage factory, and optional blob interfaces in the design.

- [x] Inventory the assertions to extract and preserve their observable cases.
- [x] Add a failing external public-interface probe before the implementation exists; identify API RED separately from behavioral RED.
- [x] Extract codec, storage baseline, and blob assertions into named modules with static case errors.
- [x] Update native fixture owners and existing callers; keep native fault/process-exit checks local.
- [x] Add negative controls for a broken adapter, incorrect profile, and invalid codec input.
- [x] Publish the profile table with version owners, mismatch behavior, optional features, and remaining exclusions.
- [x] Run focused native, consumer, blob, profile, and feature-isolated checks, strict Clippy, and formatting.
- [x] Freeze source and evidence for independent review.

## Task 2: Author workflow baseline and skills

**Owner:** assigned independently from Task 1.
**Files:** `skills/rom/` with four skill sources and shared executable assets; skill-only verifier and assembler modules under `scripts/skills/`.
**Consumes:** Design/profile contracts and existing public application/operator/release interfaces.
**Produces:** Four named workflows, one canonical asset source, and portable bundle assembly functions for Task 3.

- [x] Define four pressure scenarios: Resource/action authoring, native module/adapter conformance, uncertain operator outcome, and local release verification.
- [x] Run baseline agent attempts without the new skills and record actual gaps. A successful baseline is not a fabricated failure.
- [x] Write the four skills using `writing-for-agents`, `writing-skills`, and the ROM writing rules.
- [x] Give each skill a profile/feature admission step, executable example, negative case, and completion verifier.
- [x] Share canonical references and assets. Include them in the assembled bundle with source hashes.
- [x] Make source-root prerequisites explicit. Reject missing assets, obsolete profiles, and unsupported features before execution.
- [x] Preserve stable operation identity in operator recovery; do not infer compensation from an unknown outcome.
- [x] Run asset tests and packaging negative cases. Freeze the package for held-out evaluation.

## Task 3: Extracted distribution and independent evaluation

**Owner:** coordinator, with independent evaluators.
**Files:** workspace/dependency wiring; `scripts/check-rust`, `scripts/check-packages.mjs`, skill verifier entrypoint; public consumer tests and evidence.
**Consumes:** Reviewed Tasks 1–2.
**Produces:** Repeatable local and extracted-package acceptance with recorded source identity.

- [x] Wire the conformance crate as a development dependency only and retain publication policy.
- [x] Extend the package consumer to include and execute the selected conformance integration tests with their development dependencies.
- [x] Assert that ROM dependencies resolve only to extracted packages, not the source checkout.
- [x] Assemble and extract the skills; verify all referenced assets and source hashes.
- [x] Run one held-out task per workflow with an independent agent and retain its artifacts and results.
- [x] Record behavioral success separately from source review and from human-usability claims.
- [x] Run full local checks, default/provider demo acceptance, and package verification on the final source.
- [x] Obtain final review, update OpenSpec task 4.4 only on evidence, and integrate.

## Completion boundary

Task 4.4 closes only when the public fixtures and extracted workflows pass their stated checks.
The independent Field registry remains an explicit future proposal, not an implemented guarantee.
Stages 4.5–4.6 retain full reference recovery acceptance, release artifacts, support notes, and the final requirements audit.
