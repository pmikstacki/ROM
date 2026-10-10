## ADDED Requirements

### Requirement: Public control composition

ROM Studio SHALL expose the existing supported controls through its public root and controls-only entry.
The supported catalog SHALL include Input, Button, Textarea, NativeSelect, NativeSelectOption, NativeSelectOptGroup, Checkbox, Slider and Label.

#### Scenario: Independent Svelte consumer

- **WHEN** an independently copied consumer imports the catalog from `rom-studio/controls`
- **THEN** its type check and production build resolve only public package entries
- **AND** the consumer requires no private `$lib` or Studio source aliases

#### Scenario: Existing root callers

- **WHEN** an application imports Input or existing client and renderer APIs from the root entry
- **THEN** those public names remain available
- **AND** the additional supported controls are available from the same facade

### Requirement: Existing control semantics

Public exports SHALL preserve each component's existing class host, ref target and bindable value representation.
The export change SHALL NOT add browser auth APIs or new UI dependencies.

#### Scenario: Consumer edits and refs

- **WHEN** the consumer edits text, selects an option, toggles a checkbox or adjusts a slider
- **THEN** the bound application values update
- **AND** bound refs identify their actual native or bits-ui host elements

#### Scenario: Compact styled composer

- **WHEN** the consumer imports `rom-studio/styles` and applies bounded textarea styling
- **THEN** the textarea remains usable at desktop and narrow viewport sizes
- **AND** NativeSelect wrapper classes and inner data-slot selectors target their documented elements

#### Scenario: Failure and keyboard states

- **WHEN** controls are disabled or marked invalid
- **THEN** their existing native and accessible states remain exposed
- **AND** supported keyboard operations still update values when enabled

### Requirement: Independent public source admission

Public control admission SHALL install the supplied source with a frozen consumer graph through `npm ci`.
The consumer SHALL declare only ROM and its selected Svelte, build, type-check, and browser toolchain.
The exported stylesheet SHALL declare its imported stylesheet packages as runtime dependencies.
ROM SHALL reject dependency graph drift, source mismatch, reused evidence paths, and copied dependency shortcuts.

#### Scenario: Frozen independent installation

- GIVEN a supplied source package and its matching consumer lock
- WHEN public control verification installs the package
- THEN the installed ROM package SHALL be a physical consumer-local directory
- AND installed source and dependency versions SHALL match the supplied source and frozen lock
- AND a changed dependency resolution SHALL reject admission

#### Scenario: Preserve historical evidence

- GIVEN an existing evidence directory with an earlier result or archive
- WHEN the verifier receives that output path
- THEN the verifier SHALL fail before modifying any existing evidence

#### Scenario: Complete release browser admission

- GIVEN a release artifact source and a locked independent installation
- WHEN release admission is requested
- THEN actual Chromium and WebKit tests SHALL pass
- AND authoring-only, bootstrap, skipped-browser, missing-engine, failed-test, or unmatched-source evidence SHALL reject admission

### Requirement: Bounded public source archive admission

Before extraction, ROM SHALL admit only regular files and directories under the fixed `rom-studio-source/` candidate prefix.
Admission SHALL bound compressed bytes to 32 MiB, expanded archive bytes and logical member bytes to 64 MiB, and members to 20,000.
Admission SHALL reject traversal, duplicate members, links, and undeclared top-level members.
Candidate source archives SHALL retain ROM's LICENSE and THIRD_PARTY_NOTICES.md.

#### Scenario: Reject an unsafe candidate source archive

- GIVEN a candidate listing with traversal, links, duplicate entries, undeclared members, or exceeded byte/member bounds
- WHEN public source archive admission runs
- THEN admission SHALL fail before extraction writes any archive member
