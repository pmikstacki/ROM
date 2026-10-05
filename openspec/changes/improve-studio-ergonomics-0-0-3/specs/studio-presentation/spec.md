## ADDED Requirements

### Requirement: Descriptors support ergonomic generic presentation
ROM SHALL expose optional bounded presentation metadata from the same accepted Resource definition used for codecs and operations.
Presentation MUST NOT redefine accepted values, grant authorization, or alter persisted schema identity.

#### Scenario: Hidden presentation fields stay hidden
- **GIVEN** a Resource title selector and field labels referring to a field that discovery withholds
- **WHEN** the actor requests discovery
- **THEN** ROM SHALL omit that selector and field metadata from the response
- **AND** Studio SHALL use a neutral fallback instead of requesting the hidden field

#### Scenario: Backend and frontend agree on presentation
- **GIVEN** an actual encoded Rust discovery fixture with field groups and Settings classification
- **WHEN** the TypeScript client validates that fixture
- **THEN** it SHALL preserve recognized metadata and exact semantic descriptors
- **AND** invalid known metadata SHALL be rejected

### Requirement: Generic Resource views separate titles from identity
Studio SHALL use declared authorized title fields for human presentation and retain exact Resource identity as secondary copyable text.

#### Scenario: A User has a readable heading
- **GIVEN** a User with disclosed display name Alice and a structured string identity
- **WHEN** Studio opens its details
- **THEN** Alice SHALL be the human heading
- **AND** the exact identity SHALL remain available without parsing it to infer a name

### Requirement: Studio has a shared right inspector
Studio SHALL use a right-edge chevron to toggle the shared Filters and Details inspector for Resources and Work.
Quick filters SHALL remain available through a popover.

#### Scenario: Closing and resizing retain a draft
- **GIVEN** a draft inside the inspector
- **WHEN** the user closes, reopens, or changes between wide and narrow layouts
- **THEN** the draft SHALL remain intact
- **AND** keyboard close SHALL return focus to the activating control

### Requirement: Work inspection is readable and preserves recovery
Studio SHALL display readable work definition, category, status, and attempts before technical identity.
It MUST NOT invent a historical timeline from a current work snapshot.

#### Scenario: Unknown recovery is retried safely
- **GIVEN** a recovery request with an unknown outcome
- **WHEN** the user closes and reopens the inspector and retries
- **THEN** Studio SHALL preserve the exact request identity and expected version
- **AND** it SHALL prevent navigation from silently discarding unresolved recovery

### Requirement: Settings and plugin settings use Resources
Studio SHALL group Settings from authorized Resource presentation metadata and use the shared operation and form contracts.
Plugin Settings MUST NOT bypass core policy, configuration ownership, or host-approved secrets.

#### Scenario: A plugin supplies Settings without a custom screen
- **GIVEN** an authorized plugin Settings Resource with group metadata and ordinary fields
- **WHEN** Studio loads discovery
- **THEN** the Settings group SHALL appear without a plugin-specific controller
- **AND** edits SHALL use the ordinary revision-checked mutation path

### Requirement: Semantic controls preserve the generic value contract
Studio SHALL support a declared finite catalog of semantic field editors and displays with shared wrapper behavior.
Unknown semantic codecs SHALL remain protected against unsafe editing.

#### Scenario: A date picker does not change date meaning
- **GIVEN** a calendar-date codec and a distinct instant codec
- **WHEN** Studio renders and submits their controls
- **THEN** each SHALL retain its own representation and timezone rules
- **AND** server validation SHALL remain authoritative

### Requirement: Browser ergonomics has executable acceptance
Studio SHALL test focus, keyboard operation, meaningful labels, layout boundaries, draft retention, and recovery for changed screens.
Mockups and screenshot checks alone SHALL NOT establish acceptance.

#### Scenario: A right inspector is usable with a keyboard
- **GIVEN** a keyboard user on a narrow screen
- **WHEN** the user opens, edits, and closes the inspector
- **THEN** controls SHALL remain reachable and focus SHALL return predictably
- **AND** unsaved edits SHALL remain available when the inspector reopens
