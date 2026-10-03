# Prepare a usable framework release

## Why

The integrated alpha demonstrates the Resource premise, but a client application
must also survive definition upgrades, interrupted work, growing data and operator
intervention. Completing separate prototypes does not establish that journey.

## What Changes

Deliver four stages in sequence:

1. A public-API reference application and ergonomics.
2. Enforced relationships and data lifecycle/migrations.
3. Integrated measured query selection and indexes.
4. Operational recovery, identity/configuration, and release contracts.

Each stage carries executable acceptance evidence. Existing demo and
packages are extended instead of introducing a second application framework.

## Impact

Changes affect demo, core, adapters, CLI, documentation and local release tooling.
Studio, WASM, RabbitMQ, shared tenancy, independent multiwriters and analytics are
outside this release goal. No crates.io publication is requested. Persisted-format
changes require explicit migration/compatibility tests and an untouched recovery
source; they must never silently reinterpret existing receipts or pending work.
