# Establish ROM

## Why

Resource definitions should carry reliable behavior across every caller. ROM will provide a Rust foundation where adding a resource kind reuses one consistent mutation and reaction contract, instead of rebuilding correctness in each service or transport.

## What Changes

- Establish Resource Oriented Meta Framework as a backend Rust library.
- Define resources, typed fields, actions, events and reactions.
- Specify database persistence guarantees and Rust extension interfaces.
- Reserve HTTP and RabbitMQ for separate integrations; defer WASM.
- Capture behavior as normative requirements with acceptance scenarios.

## Capabilities

### New Capabilities
- `resource-model`: Resource identity, typed fields, revisions and lifecycle.
- `action-runtime`: One mutation path with validation and concurrency guarantees.
- `event-reactivity`: Committed events and recoverable reactions.
- `field-extensions`: Built-in field families and Rust extension contracts.
- `persistence`: Atomic persistence and a database adapter contract.
- `boundary-adapters`: Transport-independent access and separate integrations.

### Modified Capabilities

None. There is no implemented ROM baseline.

## Impact

Future implementation will add Rust library code, adapters and conformance tests. This change currently adds planning artifacts only. No legacy wire compatibility or database migration is implied. Future import adapters must validate and version their mappings; reverting this initial documentation change affects no runtime data.
