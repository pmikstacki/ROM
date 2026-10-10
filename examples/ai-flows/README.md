# Public AI flow consumers

This standalone package is an external consumer of the public ROM and ROM AI interfaces.
Its publication and ticket-triage modules use public Resources, typed tools and FlowHost.
Five declaration/codec tests and twelve durable flow cases passed on actual SQLite/redb adapters.
Each recovery case exercises both publication and triage. All-target standalone Clippy also passed.
The executed cases include lost acknowledgement, reopen, current tool-grant denial, cancellation and generation limits.

Publication keeps three domain Resources: Draft, Edition and Head.
The application captures an authorized draft revision into an Edition identity before AI execution.
Edition preparation and Head publication are separate typed actions.
An Edition retains its original source revision and text, so later head changes do not rewrite historical citations.
Preparation does not claim that the Head was published.
The application supplies prompts, semantic policy, identity credentials and publication intent.
ROM owns run identity, work stages, typed tool commands, receipts and uncertain action recovery.

Ticket triage reads a current authorized Ticket and applies one bounded classification action.
Its domain validator accepts declared classifications only.
The same public flow and tool APIs support both domains.
Neither consumer imports a private AI module or creates a scheduler.

The fixtures run on both SQLite and redb.
They replay exact original command receipts before interpreting target revision changes.
Private source content must be denied after current grants change.
Local deterministic provider fixtures establish these boundary cases; they do not establish production provider or application quality.

AP-UX-007/008 map to durable stages and safe recovery.
AP-UX-013/015 map to current private grants and bounded grounding.
AP-UX-023/024 map to preserved historical sources and explicit fresh run identity versus receipt replay.
AP-UX-027 maps to immutable routing and conservative money accounting.
Original Astral Plane acceptance, packaging and release admission remain separate requirements.

Ticket classification uses an application-owned custom Field implemented through public shape/encode/decode contracts.
Its declared wire values are unclassified, routine and urgent.
Malformed values must fail before a mutation or success event.
The consumer keeps that semantic policy outside the provider and ROM core.

The current core AI candidate passes 131 cases. The feature-enabled adapter checkpoint passes 44 local HTTP/TLS/database cases.
An earlier default-feature adapter command executed only three wire cases; its separate log remains preserved.
Those results do not transfer automatically to this new standalone package.
Its standalone pinned lock, intended missing-API failures and initial domain/tool journey evidence are retained.
The current standalone evidence is `/var/tmp/rom-ai-task5-ai131-adapter44-external17-regressions.log` in the verifier container.
The feature-corrected adapter evidence is `/var/tmp/rom-ai-task5-adapter-feature-corrected-regressions.log`.
Independent review, combined verification, packaged installation and original-consumer acceptance remain separate gates.

Declare the application's Resources and actions first. Build a versioned ToolRegistry through `publication::tools` or `triage::tools`.
Add that registry through `FlowHost::tools`, install the Resource definitions, then call `FlowBuilder::build` with storage and the shared CPU pool.
The trusted FlowAuthority checks current owner, attempt-budget and tool-data grants. Serialized owner identities do not grant permission.
Submit the bounded original request through `FlowClient::submit`. Use `FlowClient::view` for sanitized progress and failure.
The complete public composition is in `tests/flows.rs`; it supplies no private AI implementation or independent scheduler.

If an action acknowledgement is unknown, preserve its run and command identity.
Call `FlowClient::resume` with the current authorized revision and an explicit operation key to recover its original receipt.
Cancellation of an unknown effect retains reconciliation state; it does not declare that the effect stopped or undo its history.
When the generation limit is exhausted, the run reports `BudgetExhausted` before a successor reservation or POST.
For a permitted fresh execution, inspect already committed domain stages and construct a new submission ID and idempotency key.
Reusing the original submission identity replays the original run. It does not start another generation.
The application selects new source captures and target revisions; ROM does not invent replacement domain work.

The owner projection now has an optional `read_progress()` snapshot.
Its bounded status and ordinal contain no tool arguments, names, call IDs or results.
`Queued` identifies a pending read. `Active` means the host observed a physical callback lease.
That lease can remain active after the waiting callback has timed out.
`AwaitingRecovery` means the host found no active lease for a retained unresolved read.
A pure projection reports `ActivityUnknown` when it cannot inspect physical ownership.
Completed and non-read stages have no read progress.

Progress is advisory. It does not authorize execution or certify a successful callback.
For explicit recovery, fetch the current authorized view and pass its revision with a fresh operation key to `FlowClient::resume`.
Same-key replay retains its original expected revision and returns a fresh authorized projection.
The framework checks current grants, physical ownership and frozen limits again.
An explicit bounded wake preserves the original callback ordinal and operation identity.
Applications do not need operator or private service permissions to use this API.

The complete17-case external suite passed with these progress assertions on both public consumer flows.
The library also passed165 AI cases and44 feature-enabled adapter cases.
Independent review, a new combined verifier and original application adoption remain separate gates.
