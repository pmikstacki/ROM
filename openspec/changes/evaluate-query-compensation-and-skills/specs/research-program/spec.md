## ADDED Requirements

### Requirement: Database planning remains adapter-owned
The selector experiment SHALL compare semantically equivalent core and native paths without forcing a database index or importing vendor planning into the Resource contract. Estimates MUST remain performance hints.

#### Scenario: Statistics become stale
- **GIVEN** an admitted query and statistics from an earlier storage generation
- **WHEN** the selector chooses an execution path
- **THEN** it uses the correct bounded fallback
- **AND** estimates cannot change authorization, query meaning or admission limits

#### Scenario: Planner output is unknown
- **GIVEN** a version-pinned planner adapter encounters an unrecognized diagnostic form
- **WHEN** it cannot supply a validated estimate
- **THEN** the selector uses the reference path without inventing a cheaper plan

### Requirement: Stable Resource semantics across implementations
The Resource/action/committed-change/event contract SHALL have the same meaning across conforming adapters. Optional execution optimizations MUST NOT change results or weaken mandatory persistence guarantees.

#### Scenario: Exact query pushdown is unavailable
- **GIVEN** a conforming adapter without an optimization for an accepted query
- **WHEN** the query executes through ROM
- **THEN** the common evaluator preserves its meaning within declared work bounds
- **AND** the application does not supply a backend-specific query or accept an approximate result

### Requirement: Fair query approach comparison
Generated, hybrid and runtime query authoring SHALL be compared over identical semantics and data. Construction, translation, compilation and execution measurements MUST be distinguished.

#### Scenario: Three physical execution variants
- **GIVEN** an identical normalized query, data and authorization rule
- **WHEN** core evaluation, unindexed exact pushdown and indexed execution are compared
- **THEN** every variant must pass the same semantic assertions before performance ranking
- **AND** measurements distinguish latency from candidate counts and actual database work

### Requirement: Measured performance in every experiment
Experiments SHALL consider performance alongside correctness and ergonomics. Reports MUST name workloads, measurement methods and unmeasured costs rather than claiming universal optimality.

#### Scenario: Optimization trades read cost for maintenance work
- **GIVEN** an index or retained runtime object reduces repeated-read or invocation cost
- **WHEN** the report recommends adoption
- **THEN** it includes measured maintenance or setup costs where feasible
- **AND** it identifies memory, allocations or concurrency costs not yet measured

#### Scenario: Backend translation preserves meaning
- **GIVEN** field codecs, row/field policies and moving-view query semantics
- **WHEN** an adapter translates a filter and order
- **THEN** authorized results match the shared evaluator without numeric coercion or premature limiting
- **AND** unsupported capabilities fail explicitly

### Requirement: Explicit compensation evidence
The compensation harness SHALL preserve prior commits and use explicitly declared actions. It MUST distinguish confirmed failure, transient failure and unknown outcome.

#### Scenario: Uncertain external outcome
- **GIVEN** a synthetic provider may have accepted an operation before its response was lost
- **WHEN** the chain cannot establish its outcome
- **THEN** the harness preserves reconciliation state instead of assuming failure and compensating

#### Scenario: Concurrent changes survive repair
- **GIVEN** another operation changed the same Resource after the original step
- **WHEN** a declared compensation releases its own reservation
- **THEN** unrelated changes and committed history remain intact

### Requirement: Version-aware skills and bounded extension research
Skills and WASM recommendations SHALL cite actual ROM seams, current primary sources and reproducible checks. Standalone probes MUST NOT be reported as integrated features.

#### Scenario: Public extension seam lacks required state
- **GIVEN** an adapter needs state but a public hook accepts only a function pointer
- **WHEN** the research recommends a sandbox integration
- **THEN** it identifies the missing seam and proposes an explicit contract instead of ambient global authority
