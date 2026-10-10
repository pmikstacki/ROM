## ADDED Requirements

### Requirement: External applications use supported interfaces
ROM SHALL support an external application through public Rust and Studio interfaces without private aliases or unreviewed vendor patches.

#### Scenario: Two distinct application domains
- **WHEN** Astral Plane and a non-AI maintenance portal use extracted release inputs
- **THEN** both SHALL complete selected Resource, mutation, recovery, and presentation workflows through public interfaces
- **AND** their evidence SHALL identify matching source, dependencies, commands, and results

### Requirement: Feedback has explicit acceptance
The release SHALL retain each collected feedback group and an explicit disposition.

#### Scenario: A selected usability candidate
- **WHEN** a candidate is marked verified
- **THEN** its regression and external acceptance SHALL demonstrate the intended behavior
- **AND** source inspection, screenshots, or reported historical tests alone SHALL NOT close that candidate

### Requirement: Optional AI preserves core guarantees
AI routing and agent flows SHALL use the accepted Resource, action, event, authority, and durable work contracts.
The core MUST NOT depend on a concrete model provider.

#### Scenario: Provider response is lost
- **WHEN** a provider may have accepted a request and its response is lost
- **THEN** the flow SHALL retain its identity and explicit uncertainty
- **AND** retry SHALL obey the selected reconciliation, deadline, and aggregate budget policy
- **AND** model output SHALL NOT grant tool authority

### Requirement: Production profile has complete operational evidence
The release SHALL record tested identity, application restoration, populated upgrade, telemetry, bounded load, and failure recovery for its production profile.

#### Scenario: Fresh-host restoration
- **WHEN** an isolated accepted application is restored
- **THEN** evidence SHALL cover its database, blobs, configuration and identity references, receipts, and unfinished work
- **AND** measured recovery limits and unsupported conditions SHALL be documented

### Requirement: Release acceptance precedes publication
The matching 0.1.0 source and artifacts SHALL pass the complete verifier, independent review, external acceptance, and deployment checks before publication.

#### Scenario: Research is complete but implementation is pending
- **WHEN** a required work package lacks implementation or sufficient acceptance evidence
- **THEN** the release goal SHALL remain incomplete
- **AND** targeted successful tests SHALL NOT substitute for that missing evidence
