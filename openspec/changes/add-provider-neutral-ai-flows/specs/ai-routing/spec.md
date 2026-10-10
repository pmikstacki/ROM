## ADDED Requirements

### Requirement: Optional provider-neutral routing

ROM AI integration SHALL reside in optional `rom-ai` and `rom-openrouter` crates.
ROM core SHALL remain free from AI providers, HTTP clients, routing policy and database drivers.
The neutral Provider contract SHALL expose bounded catalog, completion and reconciliation operations.

#### Scenario: Core-only consumer

- GIVEN an application that depends only on ROM core
- WHEN the workspace adds the optional AI crates
- THEN the application SHALL retain its existing public paths and dependency boundary
- AND its core-only checks SHALL require no provider credentials or HTTP integration

### Requirement: Run-scoped deadline and routing continuation

A run SHALL retain its policy version, deadline, consumed limits and routing continuation across restart.
Discovery, admission, credential resolution, transport, validation and checkpoint persistence SHALL share the tick deadline.
An unrelated run SHALL NOT inherit a model cursor or paid-tier continuation.

#### Scenario: Slow discovery and independent runs

- GIVEN two runs with the same provider endpoint and schema and a delayed catalog response
- WHEN the first run exhausts its deadline or rejects a model
- THEN it SHALL make no dispatch after deadline
- AND the second run SHALL retain its own candidate order and paid authority
- AND restart SHALL preserve the first run's consumed attempts

### Requirement: Conservative durable spend reservations

Potentially billable dispatch SHALL require a durable reservation bound to run, attempt, policy and worst-case exact cost.
Money accumulation SHALL use checked integer units and round estimates up.
Unknown usage SHALL remain reserved until authoritative settlement.
Dynamic policy changes SHALL NOT erase consumed attempts or grant paid authority to an existing run.

#### Scenario: Concurrent claims and lost usage

- GIVEN an account budget with capacity for one attempt and two concurrent runs
- WHEN both attempt reservation
- THEN at most one SHALL obtain dispatch authority
- AND a lost response SHALL retain its reservation
- AND replay with changed reservation input SHALL fail identity matching

### Requirement: Capability and output validation

Routing SHALL enforce schema, tool, text modality and context requirements before dispatch.
The adapter SHALL preserve explicit unsupported-capability errors.
Application validation SHALL remain decisive after provider schema adaptation.

#### Scenario: Schema subset and invalid output

- GIVEN an output schema without a selections property
- WHEN the OpenRouter adapter prepares its wire request
- THEN it SHALL NOT insert a null selections property
- AND unknown or duplicate domain identifiers SHALL remain rejected by the application validator

### Requirement: Typed provider errors and uncertainty

The adapter SHALL inspect response error bodies even after HTTP 200.
It SHALL separate invalid request, denied authority, rate limiting, unavailable provider, invalid output and unknown outcome.
A started request with lost evidence SHALL require trusted reconciliation before another dispatch of that attempt.
A missing generation ID or lookup miss SHALL NOT establish non-acceptance.

#### Scenario: Unknown started request

- GIVEN a committed executing attempt and a lost response without generation ID
- WHEN work recovers or an operator requests reconciliation
- THEN the provider verifier SHALL return unresolved
- AND the existing reservation SHALL remain charged
- AND ordinary retry SHALL NOT authorize another request
