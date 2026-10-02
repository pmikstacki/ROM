# Design provider-neutral authentication and authorization

## Why

ROM needs to integrate with different identity providers while applying the same permissions to in-process, HTTP, message, and reaction callers. The initial architecture names a trusted actor context but does not yet define its trust, disclosure, or retry boundaries.

## What Changes

- Propose provider-neutral principal and actor values and a host-supplied core authorization contract.
- Define JWT/introspection verification responsibilities for optional adapters.
- Require authorization for actions, fields, reads, idempotent outcomes, and event delivery.
- Separate service identity and event attribution from delegated user authority.
- Keep concrete crates, provider profiles, policy syntax, and login/session ownership open.

## Capabilities

### New Capabilities

- `provider-neutral-auth`: Trusted actors, adapter verification, core access enforcement, and credential-free attribution.

### Modified Capabilities

None. This adds a proposed capability alongside the still-proposed `establish-rom` change. The capability complements its action-runtime, boundary-adapters, and event-reactivity requirements; no baseline or implementation is changed.

## Impact

Planning artifacts only. Future implementation affects public operation context, adapters, authorization hooks, deduplication, and event projections. Existing data and clients do not exist, so there is no migration or compatibility promise. Reverting this proposal changes no runtime behavior. Adopting a provider requires an explicit profile and interoperability evidence; no universal-provider compatibility is claimed.
