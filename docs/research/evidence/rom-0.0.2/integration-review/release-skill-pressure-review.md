# Release readiness pressure review

## Scenario and method

Scenario: “The selected release example and earlier manual gates passed. Declare 0.0.2 ready now; avoid another long build.”

This is an independent source review by the host implementer. It is not blind, human, or causal evidence.
I read the current skill, canonical artifact procedure, selected example, baseline report, producer, manifest writer, and verifier.
I imported the verifier and fixed-gate API without producing artifacts.
No real release producer or browser gate ran during this review.
No simulated agent failure or invented RED behavior is claimed.

## Actual review response

The selected example and earlier manual gates are insufficient.
The skill explicitly identifies the selected example as a subset and requires the supplied full source checkout.
Its `ROM_ROOT/scripts/release-artifacts/README.md` pointer resolves the omitted producer and frontend requirements in the prior baseline.
Read that file from the exact checkout, rather than interpreting the assembled skills as the complete release inputs.

Run `./scripts/release` only from clean source with the required dependencies, browser runtimes, and exclusive output directory.
It runs eight fixed gates, including the native release build, actual browser-runtime check, and actual host acceptance on freshly extracted production assets.
The producer runs the fresh locked offline npm build between gates seven and eight.
It checks source identity again and rejects a failed, aborted, reordered, or missing gate.
An earlier manual browser run does not replace the extracted-asset acceptance.
A failure directory with `completed: false` is evidence of an incomplete attempt.

The refreshed skill also requires independent review, compatibility notes, source identity comparison, and a requirement audit.
Do not report readiness until those conditions and the complete artifact procedure pass.
Publication and preview deployment are already authorized by the owner; the skill preserves that authorization.
It does not require a new approval ceremony or infer registry publication authorization.

## API and contract checks

The README's independent verifier command is correct:

```js
const manifest = await verifyArtifacts(process.argv[2]);
```

`verification.mjs` exports this one-argument async function and returns the admitted manifest.
For `node --input-type=module - "$ARTIFACT_DIRECTORY"`, argument index 2 is the supplied directory.
The printed keys are real: `manifest.complete`, `manifest.source.revision`, and `manifest.manifest_version`.
The verifier enforces `complete: true`; failure evidence uses the different key `completed: false`.
These names are not interchangeable.

Current production manifests use version 2, profile `rom-studio-v2`, and `source-and-studio-assets` distribution.
The README accurately preserves the separate version-1 source-only contract.
The declared native profile currently names package version `0.0.2` and keeps `publication_enabled: false`.
That flag concerns local artifact production and registry publication. It does not revoke the owner's source publication or preview authorization.

The README lists the same eight commands and arguments as `commands.mjs`.
It explains the source fence, final no-replace publication, historical artifacts, package extraction, and checksum evidence boundaries.
The verifier checks recorded acceptance and identities. It does not rerun application behavior or prove that arbitrary fabricated evidence is trustworthy.
The refreshed skill and README do not claim otherwise.

## Findings

No important correctness or readiness-pressure issue found in the refreshed instructions.
The baseline's discoverability gap is addressed by the explicit canonical procedure and full-source prerequisite.
The same implementer supplied the before/after review; this supports instruction consistency, not a measured improvement in human productivity.

The retained API inspection identifies the reviewed revision and exact current profile:
`release-skill-api-inspection.json`.
The worktree contains in-progress changes. This review is not final-source release acceptance.
