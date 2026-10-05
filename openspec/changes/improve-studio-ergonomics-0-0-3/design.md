# Studio ergonomics and descriptor design

## Context

The owner requires Studio to set the ergonomic target and the backend to meet it through generic ROM functionality.
Research and alternatives are in [the release research](../../../docs/research/rom-0.0.3-studio-release-research.md).
The approved inspector mock is [here](../../../docs/research/mockups/rom-0.0.3/right-inspector-proposal.png).
The mock contains illustrative values; it does not define a cross-kind query or a historical Work protocol.

## Decisions

Use one semantic Resource definition with optional typed presentation metadata.
Keep storage shape, codec meaning, and authorization separate from labels, title selectors, settings classification, and widget preferences.
Freeze and validate metadata at registration. Project it through authorized discovery before Studio consumes it.
Use shared wire fixtures to verify Rust encoding against the TypeScript client.

Select human titles only from declared, disclosed fields. Keep exact identity available as secondary copyable text.
Do not parse string IDs to infer names. Unknown custom codecs remain read-only unless their editable contract is known.

Use a right-edge chevron to toggle the inspector. Keep the quick Filters popover.
Reuse the shell across Resources and Work. Keep recovery controls explicit and preserve uncertain request identity.
Work uses its current snapshot contract. A historical timeline requires a new authorized history contract and is not inferred from attempts.

Settings are ordinary Resources classified by optional presentation metadata.
Plugin groups use the same descriptors, forms, action path, and core policies.
The screen must not override configuration source ownership or host-approved network/secret settings.

Reuse installed shadcn-svelte controls before adding dependencies.
Evaluate Svelte-native sortable lists and exact-value JSON editing with interaction and bundle-size probes.
Codecs define temporal and color semantics; widgets cannot infer those meanings from field names.

## Compatibility

The initial discovery extension is optional and additive at protocol version 1; existing clients ignore unknown top-level metadata.
New clients validate recognized presentation metadata and preserve legacy descriptors without it.
Persisted Descriptor and FieldDescriptor layouts remain unchanged.
Public Rust DiscoveredResource struct literals need the new optional presentation member; document this source compatibility change.

## Verification

Test disclosure filtering, exact metadata byte limits, invalid registration, and the shared Rust/TypeScript fixture.
Test right-edge geometry, keyboard focus, mobile drawers, viewport changes, and draft retention.
Test plugin Settings discovery and ordinary mutation behavior through the shared client.
Test Work recovery controls under unknown acknowledgement without replacing the idempotency identity.
Before integration after refactors, run affected checks and the full local verifier.
The final release additionally needs real backend journeys, extracted consumer acceptance, and source-bound artifacts.
