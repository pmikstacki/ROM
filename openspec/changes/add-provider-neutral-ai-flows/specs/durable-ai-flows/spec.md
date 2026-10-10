## ADDED Requirements

### Requirement: Resource-backed flow composition

A flow SHALL use Resources, typed actions, receipts and existing durable channels for state and execution.
Its channel callback SHALL process at most one generation request or one tool call per tick.
The facade SHALL NOT create a competing scheduler or polling task.
Committed checkpoints and their next intentions SHALL share the existing atomic commit contract.

#### Scenario: Restart after checkpoint

- GIVEN a completed checkpoint with a frozen next intent
- WHEN the Runtime restarts and duplicate delivery occurs
- THEN the facade SHALL resolve the stored checkpoint or original action receipt
- AND it SHALL NOT repeat the completed provider request or tool mutation

### Requirement: Typed authorized tools

Read tools SHALL receive restricted read operations and bounded typed input.
Mutating tools SHALL invoke only registered typed Resource actions under current host-supplied authority.
Prepared tool commands SHALL retain target, expected revision, input, retry epoch and original idempotency identity.
Planning SHALL NOT grant tool authority.

#### Scenario: Permission changes after planning

- GIVEN a valid plan and a later revoked tool grant
- WHEN the flow attempts execution or result replay
- THEN current authority SHALL deny the operation or disclosure
- AND earlier commits SHALL remain intact
- AND no model-selected principal SHALL replace host authority

### Requirement: Automatic durable delayed recovery

A confirmed rate limit SHALL commit waiting state and one delayed next intent atomically.
The existing Work scheduler SHALL resume eligible due work without consumer polling or a manual application resume.
The delay SHALL preserve original run identity, current authority, deadline and consumed limits.
Unknown started effects SHALL remain held for trusted reconciliation; routine scheduling SHALL NOT bypass that hold.

#### Scenario: Automatic resume after rate limit

- GIVEN a waiting run with persisted retry time in the future
- WHEN the Runtime restarts and its existing Work worker reaches that time
- THEN no claim or provider request SHALL occur before due
- AND eligible work SHALL resume automatically without an owner or application resume command
- AND exact checkpoint replay SHALL create no duplicate intent
- AND the due attempt SHALL remain subject to the original deadline, current authority and budgets

#### Scenario: Attach before recovered execution

- GIVEN a restored database with an already-due AI tick
- WHEN FlowBuilder constructs Runtime and returns its handles
- THEN Runtime attachment SHALL be complete before handles permit worker startup
- AND Builder construction SHALL NOT dispatch recovered work
- AND explicit start_work SHALL process the tick through its attached host

### Requirement: Explicit cancellation and unknown outcomes

Cancellation before dispatch SHALL prevent dispatch.
Cancellation after execution starts SHALL preserve uncertain external effects, reservations and late evidence.
Cancellation SHALL NOT imply compensation or confirmed provider failure.

#### Scenario: Cancellation race

- GIVEN a prepared attempt and a concurrent cancellation
- WHEN revision-checked dispatch and cancellation compete
- THEN at most one transition SHALL win
- AND a started winner SHALL remain cancel-requested or awaiting reconciliation until its outcome is established
- AND it SHALL retain consumed attempt and cost history

### Requirement: Authorized progress and redacted telemetry

Run views SHALL recheck current authority and expose only declared projected fields.
Private prompts, credentials, service keys, raw tool arguments and provider error bodies SHALL remain excluded.
Ephemeral tokens and generated narration SHALL NOT become committed events automatically.
Observer failure SHALL NOT alter commit or retry semantics.

#### Scenario: Reload and observer failure

- GIVEN an authorized run view, private input and an observer that panics
- WHEN the run commits and the user reloads
- THEN committed progress SHALL remain recoverable under current authority
- AND observer failure SHALL NOT repeat generation
- AND public output SHALL contain no private input or raw diagnostics

### Requirement: Finite public-interface acceptance

The integration SHALL pass finite offline consumers on SQLite and redb through public crate interfaces.
The acceptance cases SHALL cover deadline, reservation, restart, typed tools, partial publication, denied disclosure and unknown outcomes.
Channel timeout SHALL remain shorter than its Work lease.
Nested checkpoint calls SHALL fail boundedly under overload rather than deadlock.

#### Scenario: Two unrelated consumers

- GIVEN a source publication flow and an unrelated task triage flow
- WHEN each runs with simulated providers and real supported adapters
- THEN each SHALL recover original identities after interruption
- AND partial multi-Resource publication SHALL remain explicit
- AND neither consumer SHALL import private modules or add a scheduler

### Requirement: Bounded pure-read recovery and physical ownership

Registered read callbacks SHALL be trusted, bounded and side-effect-free.
Mutations SHALL use typed Resource Actions. Calculation descendants SHALL finish before their owning closure returns.
Each read SHALL retain its original call identity and a finite persisted phase and ordinal.
A waiting caller timeout SHALL NOT authorize concurrent invocation while a physical callback lease remains live.
Current owner, tool, transcript and budget authority SHALL precede explicit retry admission.
Retry admission and its next existing-channel intention SHALL commit atomically.
An explicit wake of a Scheduled read SHALL preserve its ordinal, actor and original operation key.
A wake SHALL append one bounded Finished(Unresolved) admission and one tick; it SHALL NOT classify the original Work outcome.
Same-key replay SHALL preserve original admission inputs and SHALL NOT add another intention or tick.
Exhausted frozen read limits or trusted run expiry SHALL produce an authorized, sanitized failure without new tool I/O or account changes.

#### Scenario: Callback timeout with a live calculation descendant

- GIVEN a persisted read and a physical calculation descendant that retains Runtime admission
- WHEN the waiting callback times out and the owner requests recovery
- THEN recovery SHALL reject concurrent invocation
- AND another Runtime SHALL NOT acquire the same storage while the original owned work remains live
- AND recovery MAY admit one current-authorized retry after the physical lease ends

#### Scenario: Delivery started before the read callback

- GIVEN a Scheduled read whose native channel delivery recorded Unknown before invoking the callback
- WHEN storage reopens and the owner requests an explicit wake at its current revision
- THEN the wake SHALL retain the exact read ordinal, original operation, prepared attempt and account entry
- AND concurrent original and wake deliveries SHALL invoke at most one physical callback
- AND no completed model request or Resource Action SHALL repeat

### Requirement: Authorized advisory read progress

An owner projection MAY expose Queued, Active, AwaitingRecovery or ActivityUnknown with an ordinal between one and thirty-two.
Progress SHALL contain no call ID, alias, argument or result.
A pure durable projection SHALL NOT claim physical Active from persisted Started or Unresolved metadata.
The trusted host SHALL check current owner, registry, transcript and tool grants before progress disclosure.
New transcript and current-call checks SHALL share one bounded execution deadline.
The host MAY report Active only while its real physical lease remains live.
Progress SHALL be advisory; dispatch and recovery SHALL recheck current authority, revision and physical ownership.
Completed, cancelled and non-read stages SHALL expose no read progress.

#### Scenario: Private row grant revoked before progress disclosure

- GIVEN an unresolved read with a valid owner identity and a frozen reference to private Resource data
- WHEN that Resource denies the current read grant
- THEN the host SHALL deny progress disclosure
- AND it SHALL NOT invoke the read or change the run, account or original attempt

#### Scenario: Stale queued progress snapshot

- GIVEN an authorized Queued snapshot and a later explicit wake commit
- WHEN the caller presents the old revision with a new operation identity
- THEN recovery SHALL reject the stale revision
- AND same-key replay of the admitted wake SHALL return a fresh authorized projection without another intention
