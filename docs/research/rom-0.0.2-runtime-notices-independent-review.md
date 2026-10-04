# Independent runtime notice review

Date: 2026-10-04.

Scope: `studio/build/**`, the Vite hook, Studio asset admission and the notice tests. This review does not certify legal compliance. It assesses source provenance and artifact correspondence.

## Finding

The initial collector accepted an emitted JavaScript asset with no module or owner metadata. An executable probe supplied an asset with `type: 'asset'`, path `assets/unclassified.js`, and code from an unclassified plugin. The collector emitted an inventory with empty owners and empty modules. Validation accepted it. See [the probe](evidence/rom-0.0.2/runtime-notices-independent/unowned-javascript-probe.log).

This path bypasses the fail-closed handling for unknown virtual chunk modules. The recommended correction is to reject JavaScript assets without explicit reviewed provenance. The current ordinary production JavaScript is emitted as chunks, so this restriction does not require inventing a package owner.

## Other reviewed behavior

The collector uses positive `renderedLength` entries from emitted JavaScript chunks. It does not treat the entire npm build dependency graph as browser runtime code. The Vite module-preload helper and generated Tailwind CSS have explicit owner mappings. Zero-length compiler-only modules are excluded.

The collector copies original LICENSE and NOTICE bytes as buffers. It does not rewrite the legal text or rely on an SPDX field alone. Vendored shadcn files have a separate owner record.

The shared validator binds notice lengths and hashes, output hashes, owner/module bindings, and complete JavaScript/CSS output coverage. It rejects missing, changed and unlisted notice files. The release asset identity covers the complete inventory and copied files. Extraction uses the expected complete asset identity. These checks establish internal correspondence; they do not authenticate an arbitrary third-party inventory as legally complete.

Collection, ownership and inventory validation have separate named modules. The build hook and package gate reuse the same validator. No unnecessary duplicate notice validation was found.

The notice and asset unit tests passed for the reviewed initial candidate. Their existing cases did not cover the unowned JavaScript asset. Exact candidate hashes and raw results remain under [the review evidence](evidence/rom-0.0.2/runtime-notices-independent/).

## Correction check

The owner added a guard that rejects JavaScript or MJS assets without chunk module provenance. A new unit case covers this path. Independent execution confirms that the original bypass is now rejected. The notice and asset tests pass. See [the fixed probe](evidence/rom-0.0.2/runtime-notices-independent/unowned-javascript-fixed.log), [the tests](evidence/rom-0.0.2/runtime-notices-independent/fixed-tests.log), and [the corrected source hashes](evidence/rom-0.0.2/runtime-notices-independent/source-fixed.sha256).

No blocking finding remains in the corrected notice collector within this review scope.
