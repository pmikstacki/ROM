# Shared installation notice regression

Date: 2026-10-07. Status: implemented candidate; targeted verification only.

## Source and decision

The consumer investigation found Vite-emitted physical module paths outside Studio's lexical installation path.
The collector previously rejected those paths, although Studio intentionally shared the complete installed dependency directory.
The reviewed consumer fix resolves the configured installation before package ownership checks.

Adopt that narrow boundary in `studio/build/notices/ownership.mjs`.
Keep package-name/version checks, full license bytes, transformed query suffixes, inventory admission, and portable module IDs.
Do not add general package-link or pnpm support without separate acceptance.
This change adds no dependency and does not disable a license gate.

## Executed evidence

The producer began at `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
The fixture uses a distinct workspace and a shared installation outside that workspace.
Its first pre-change run failed with `unclassified runtime notice external module`.
After the implementation, all 12 notice tests passed.
The regression checks the exact license bytes and portable transformed module ID.
Negative cases retain rejection of missing licenses, changed package identity, and unrelated external modules.

A broader host run reported 42 passes and 16 failures because host PATH contained no Rust compiler.
That result is retained at `/var/tmp/rom-010-r3-evidence/package-tests.log`; it is not a passing verifier.
The same package/release-artifact tests ran in the existing rom-dev container with the declared Rust toolchain.
All 58 passed; evidence is `/var/tmp/rom-010-r3-evidence/package-tests-declared-toolchain.log`.
Strict OpenSpec validation passed at `/var/tmp/rom-010-r3-evidence/openspec.log`.

## Remaining acceptance

An independent implementation review, actual external control-package builds, and complete release verification remain required.
Synthetic package tests do not establish a supported production installation or a completed 0.1.0 release.
The final accepted source identity must be recorded after all concurrent implementation changes are reconciled.

## Review follow-up

The [independent review](rom-0.1.0-shared-install-review.md) found no required implementation correction.
Its suggested package-link boundary case is now executable for lexical and physical module IDs.
All 13 notice tests passed at `/var/tmp/rom-010-r3-evidence/notice-tests-external-link.log`.
This pins rejection of an individual package link outside the configured installation.
It does not add support for arbitrary package symlinks or pnpm layouts.
