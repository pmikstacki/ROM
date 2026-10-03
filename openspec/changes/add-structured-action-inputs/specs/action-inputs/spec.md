## ADDED Requirements

### Requirement: Structured action payload declarations
ROM SHALL allow authors to derive Input for a concrete named-field struct using existing Field codecs, without implementing a repository, registering another Resource or defining independent serialization behavior.

#### Scenario: Execute a structured action
- **WHEN** an author passes a derived payload to a typed Action and submits it through Runtime
- **THEN** the shared action pipeline decodes and validates the payload and applies the ordinary Resource commit and idempotency rules

### Requirement: Strict object codec semantics
Derived Input SHALL use one wire name per member and SHALL reject unknown, missing required or invalid members. Presence SHALL represent omitted members and Option SHALL represent explicit null. Independent Serde configuration and ambiguous declaration attributes SHALL be rejected.

#### Scenario: Omitted and null input members
- **WHEN** a Presence<Option<String>> member is missing or explicitly null
- **THEN** decoding preserves these as distinct values and encoding restores the same distinction

#### Scenario: Invalid renamed member
- **WHEN** an invalid value is supplied using a renamed field
- **THEN** direct codec decoding identifies its wire name. Runtime invocation reports the Resource kind and action.wire-field from the declared field allowlist. No action mutation commits.

### Requirement: Renamed facade support and declaration diagnostics
The derive SHALL support an explicitly selected facade crate path and SHALL point unsupported field type and attribute diagnostics at the relevant source declaration.

#### Scenario: Cargo dependency alias
- **WHEN** the application names the rom dependency framework and sets input(crate = "::framework")
- **THEN** the derived codec compiles and round trips without a dependency named rom

#### Scenario: Arbitrary custom codec diagnostic
- **WHEN** a manual Input returns an error containing an undeclared field or other text
- **THEN** runtime reports only the Resource kind and action name without forwarding that text
