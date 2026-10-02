# Design provider-independent capability contracts

## Why

ROM must define resources once while the host chooses databases, blob stores and notification providers. Selecting SQLite for an experiment must not make SQLite transactions, SQL or driver types part of the core. The same rule applies to folder/S3 storage and ordinary Rust functions used as notification channels.

## What Changes

- Specify distinct semantic contracts for persistence, blob storage and notification channels.
- Keep technology-specific protocols, clients, physical layouts and credentials in adapters.
- Define required guarantees, capability negotiation and explicit uncertain outcomes.
- Separate atomic resource commits from recoverable external effects.
- Require shared conformance and resilience tests across interchangeable implementations.

## Capabilities

### New Capabilities

- `capability-adapters`: Provider-independent contracts, registration, atomic persistence, blob operations, channel delivery and conformance boundaries.

### Modified Capabilities

None. This proposal complements the still-proposed persistence, field-extension, action-runtime and provider-neutral-auth capabilities. It does not claim those changes are implemented or archived.

## Impact

Design and disposable experiments only. The owner has confirmed generic adapter composition; exact Rust signatures and production dependency versions remain provisional. No deployed ROM data or clients exist. Reverting this design changes no released behavior. Future adapter replacement must preserve schema, identity, receipts and cursor-generation rules through an explicit migration; changing host configuration alone does not migrate stored data.
