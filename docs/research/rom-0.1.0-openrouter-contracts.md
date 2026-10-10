# OpenRouter adapter, durable tools and public consumers

Research date: 2026-10-07. Status: source investigation, reviewed ownership and initial test contracts.
The coordinator admitted the new crate's existing dependencies through offline metadata resolution.
The adapter now has bounded transport, global catalog decoding, native completion and tool codecs, and exact decimal conversion.
Thirty-eight adapter tests pass, including demand-aware discovery, native metadata recovery and supervisor cancellation through real flows on SQLite and redb.
Queued discovery and grant failures now have owner-visible candidate coverage. Other admission failures and optional content recovery remain pending.
These tests do not establish Task 4 acceptance.
Two new flow tests cover three observed-outcome variants on both adapters.
The current compatible interfaces and recovery candidate pass 52 flow, 14 reservation, 15 routing and four execution-deadline tests.
Both crates pass targeted Clippy. The coordinator's full verifier remains a separate gate.
No live provider request has occurred. Complete adapter-native acceptance evidence remains pending.

The accepted six-task plan remains authoritative: [implementation plan](../superpowers/plans/2026-10-07-ai-routing-and-flows.md).
Task 4 implements the optional HTTP adapter. Task 5 implements durable typed tools. Task 6 admits two unrelated public consumers.
Task 3 has 30 candidate flow tests, alongside 14 reservation and 15 routing tests. Coordinator review remains open.
These tests do not establish Astral Plane consumer acceptance.

## Existing seams and ownership

The investigation began with `crates/rom-ai/src/{provider,error,request,response,clock}.rs`, `routing/{policy,select}.rs`, and `flow/{host,builder,client,worker,resource,actions,checkpoint,operation}.rs`.
The core contracts inspected are `crates/rom/src/resource/{definition,fields}.rs`. Frozen implementation remains unchanged.

| Boundary | Proposed owner and files | Responsibility |
| --- | --- | --- |
| Provider HTTP | AI agent: `crates/rom-openrouter/src/{lib,config,credentials,transport,catalog,request,response,errors,reconcile}.rs`, crate manifest, `tests/{loopback,wire_contract}.rs` | Wire encoding, bounded transport, exact prices, sanitized responses and conservative reconciliation |
| Typed tools | Later Task 5: `crates/rom-ai/src/tools/{registry,read,action}.rs`, `tests/tools.rs` | Restricted reads, typed Resource commands and original receipt identity |
| Consumers | Later Task 6: `examples/ai-flows/`, isolated consumer admission fixtures | Publication and triage domain Resources, validators and permission policy |
| Integration | Coordinator: workspace manifest, lock, shared facades, full verifier | Exact dependency admission and reviewed integration |

ROM core stays provider and driver independent. Neither provider names nor HTTP behavior enter Resource, action, event or Work contracts.
Backend code owns credentials, grants, frozen attempts, tool execution and recovery. Frontend code observes authorized committed milestones and explicitly requests fresh executions.
Generated token fragments remain ephemeral. They cannot publish success events or replace persisted attempt evidence.

## Primary provider contracts

### Routing and capabilities

Provider `order` is a preference. Use `only` for a whitelist. Set `require_parameters: true` because defaults can ignore unsupported parameters.
Provider slugs can select endpoint variants; a base slug covers related variants. Use strict price filters, rather than soft latency preferences.
`max_price.prompt` and `completion` use USD per million tokens. `request` uses USD per request.
Keep fallback inside the frozen model, provider whitelist, parameter and rate envelope. [Provider selection](https://openrouter.ai/docs/guides/routing/provider-selection)

Send one selected `model`. Do not send a `models` fallback array, router slug, server tool, preset or repair plugin implicitly.
Server model fallback can change the returned model. A returned model outside the prepared identity cannot become a trusted completion.
[Model fallback](https://openrouter.ai/docs/guides/routing/model-fallbacks)

Schema support varies by endpoint. Preserve the exact schema object, including its required fields and additional-properties policy.
`strict` does not certify application correctness. The existing `OutputValidator` remains decisive.
[Structured outputs](https://openrouter.ai/docs/guides/features/structured-outputs)

Native tools require the assistant call message and matching tool-result messages on continuation. Include the tool definitions again.
Preserve call IDs, arguments and result order. Request `parallel_tool_calls: false`; still validate duplicate IDs and array limits.
[Client tools](https://openrouter.ai/docs/guides/features/tool-calling)

The global model catalog supplies string prices, modality and supported parameters.
Provider membership needs endpoint records; do not invent membership from model names or display labels.
Endpoint records expose provider `tag`, context, output-token maximum and parameter support.
[Model catalog](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties), [model endpoints](https://openrouter.ai/docs/api/api-reference/endpoints/list-all-endpoints-for-a-model)

Proposed discovery: bound model catalog at 4 MiB and 4096 models. Bound each endpoint response at 256 KiB and 64 endpoints.
An optional host configuration contains at most 256 explicit endpoint-discovery model IDs. It grants no paid authority.
Fetch configured records under the same deadline, with at most four concurrent refreshes and separate per-record coordination.
Unknown provider membership fails eligibility when a provider whitelist is required. Ordinary catalog use does not fetch all model endpoints.
At generation dispatch, recheck the selected endpoint capabilities and pricing under the prepared policy.
This design needs coordinator review because the frozen `Provider::catalog` has no request or policy parameter.

### Acceptance, errors and retry

OpenRouter sends HTTP 200 once a provider accepts the request, before any token exists.
An HTTP 200 error body, including code 429, cannot prove nonacceptance. Empty output can still incur prompt charges.
Typed `error.metadata.error_type` identifies the failure category; it does not certify nonacceptance.
Retry-After can occur on 429, 503 and selected 402 responses. Do not inspect raw message text for invalid-model proof.
[Error semantics](https://openrouter.ai/docs/api_reference/errors-and-debugging)

Proposed mapping for the existing public API:

| Observation | Public result | Durable consequence |
| --- | --- | --- |
| Known pre-dispatch request failure | `InvalidRequest`, `UnsupportedCapability`, `Denied` or `DeadlineExceeded` | Zero generation requests; no invented provider evidence |
| Non-200 HTTP 429 with bounded, valid retry floor | `RateLimited { retry_after_ms }` | Existing atomic delayed wake, new attempt and reservation only at due |
| Non-200 401/403 | `Denied` | Retain the exact attempt record; no implicit model advance |
| Non-200 402 | `BudgetExhausted` | No automatic paid retry or inferred budget grant |
| Non-200 400 | `InvalidRequest` | No regex-based model rejection or implicit retry |
| Non-200 503 | `ProviderUnavailable` | Conservative hold under current worker; no invented nonacceptance proof |
| HTTP 200 error, malformed body, wrong model, truncated body or unexpected SSE | `UnknownOutcome` or validated-output rejection | Preserve conservative charge; never retransmit automatically |
| Full bounded native completion | `Completion` | Validate output, exact attempt evidence, usage and current authority before publication |

Retry-After parsing accepts integer seconds or HTTP dates. Use checked arithmetic and a maximum 300-second floor.
Never shorten a larger floor to fit expiry: fail boundedly without successor work.
Missing or invalid headers use a host-declared finite floor for a confirmed HTTP 429.
Dates use trusted wall time, while all waiting and network time uses one monotonic deadline.

The initial adapter requests `stream: false`, as the accepted design specifies.
It must handle HTTP transfer chunks without buffering an unbounded body. Unexpected SSE is an uncertain outcome.
If streaming is later added, comments are not progress facts. Usage and final termination must be validated before publication.
Disconnect or cancellation does not prove billing stopped. [Streaming](https://openrouter.ai/docs/api_reference/streaming)

### Usage, lookup and identity

Use `usage.cost` as total charged cost. Do not substitute upstream inference cost.
Token accounting can include reasoning tokens. Missing cost stays unknown; it cannot settle a reservation to zero.
[Usage accounting](https://openrouter.ai/docs/cookbook/administration/usage-accounting)

Generation metadata returns model, usage and total cost, but no completed answer.
An ID match or lookup miss cannot produce `NotAccepted` or a fabricated `Completion`.
[Generation metadata](https://openrouter.ai/docs/api/api-reference/generations/get-request-&-usage-metadata-for-a-generation)

The separate content endpoint can return stored completion content. Its existence does not establish content availability or native tool-call reconstruction.
[Stored content API](https://openrouter.ai/docs/api/api-reference/generations/get-stored-prompt-completion-and-error-content-for-a-generation)

Content recovery requires Input & Output Logging to have been enabled when the generation occurred.
Logging is opt-in, has at least three months of retention, and skips regional endpoints.
The adapter must not enable it automatically. Initial reconciliation remains `Unresolved` when no authoritative completion is available.
[Logging and retention](https://openrouter.ai/docs/guides/features/input-output-logging), [metadata-only default](https://openrouter.ai/docs/guides/features/logs)

No reviewed primary chat contract promises durable deduplication from ROM's attempt ID or a request header.
Response caching explicitly permits two identical concurrent cache misses to be billed independently.
ROM receipt replay is therefore the only local deduplication guarantee. Explicit fresh runs use new identities and disable response caching.
[Response caching](https://openrouter.ai/docs/guides/features/response-caching)

### Neutral contract gaps requiring separate review

`Provider::complete` returns `AiResult<Completion>`. An error cannot carry a known generation ID from response headers.
Do not use an in-memory ID cache or another receipt store to fill this gap.
Implemented candidate extension: an object-safe default `complete_observed` returning a bounded `DispatchOutcome`.
Its `Unknown` case can retain validated `AttemptEvidence`, optional trusted usage and a sanitized cause.
Its `NotAccepted` case must identify trusted nonacceptance and a retry floor. Arbitrary deserialization grants neither property.
Existing providers use the default implementation; only the adapter supplies stronger observed evidence.
The coordinator approved this narrow extension before changes. Integration and complete adapter review remain open.
The unknown observation validates exact prepared identity, finite retry floor, token limits and cost no greater than the original ceiling.
Invalid supplied data follows the existing internally generated unknown HOLD path.
Valid uncertain evidence and usage persist before account settlement; neither publishes an answer nor authorizes generation replay.
`RECORD_ATTEMPT_EVIDENCE` now enriches only previously missing fields for the exact held attempt.
Current authority and exact committed account proof precede each bounded CAS retry.
Known IDs and usage cannot change. State, expiry, counters, checkpoint, cancellation, operations and output remain identical.
Duplicate facts produce no row change. An ordinary committed knowledge change retains ROM's normal journal semantics without a successor intent.
Historical 30-test source remains `/var/tmp/rom-ai-task3-race-frozen-work_flows.rs`, matching its original manifest hash.
Evidence: `/var/tmp/rom-ai-task4-observed-contract-red.log` records the intended missing interface.
`/var/tmp/rom-ai-task4-observed-behavior-red.log` records lost header identity: one failed and one passed test.
`/var/tmp/rom-ai-task4-observed-regression-green.log` records 61 passing tests; `/var/tmp/rom-ai-task4-observed-clippy.log` exits zero.
The independent SQLite and redb late-header races both failed before enrichment in `/var/tmp/rom-ai-task4-late-observed-both-db-red.log`.
The revised candidate records 63 passing tests in `/var/tmp/rom-ai-task4-late-observed-regression-green.log` and zero-exit targeted Clippy.
Initial adapter evidence: `/var/tmp/rom-openrouter-initial-contract-red.log`, `/var/tmp/rom-openrouter-initial-behavior-red.log` and `/var/tmp/rom-openrouter-initial-green.log`.
The expanded behavioral RED executed eleven loopback cases: five passed and six failed for missing native behavior.
Evidence: `/var/tmp/rom-openrouter-expanded-behavior-red.log`. Cargo stopped before the three wire cases.
The revised run passed all fourteen cases and targeted Clippy in `/var/tmp/rom-openrouter-expanded-final-green-clippy.log`.
The earlier Clippy failure remains `/var/tmp/rom-openrouter-expanded-clippy.log`; it reported deprecated atomic naming and a collapsible condition.
The fourteen-test source inventory is `/var/tmp/rom-openrouter-initial14-source-hashes.txt`.
Exact numeric usage cost uses `RawValue`; token prices scale before rounding. Schemas retain their original structure.
Native assistant calls retain IDs and argument strings. Continuations retain tool definitions and paired result IDs.
Metadata lookup performs the bounded original-generation GET. Its result remains unresolved because metadata cannot reconstruct the answer.

`Reconciliation` has only completed acceptance, nonacceptance and unresolved variants.
It cannot settle authoritative metadata-only cost while preserving an unknown answer.
The coordinator approved a compatible `Provider::reconcile_observed` extension. Its neutral coordinator candidate now passes seven recovery tests.
The OpenRouter override now returns matching native usage and exact total cost as held observations.
It rejects foreign models, conflicting IDs, invalid amounts and usage above the frozen bounds.
Metadata-only recovery publishes no output and does not prove provider nonacceptance.
Its bounded `ReconciliationObservation` has no `Deserialize` implementation and renders only redacted Debug text.
It validates exact prepared identity, immutable known IDs, native usage bounds and the frozen reserved ceiling.
Current durable knowledge must also reject a conflicting established count or cost.
The coordinator retains the existing shared permit and two-second lookup budget, including authorization, queue wait and every GET.
Before a knowledge commit, the coordinator rechecks current owner authority, exact prepared attempt and account proof.
Knowledge commits first. Exact known-cost settlement follows through the existing receipt contract.
Metadata-only knowledge keeps the held state, cancellation evidence and hidden output. It permits no POST or successor generation.
Optional stored-content recovery remains disabled by default. ROM never enables vendor input/output logging.
Only matching metadata and documented complete text can reach local output validation; incomplete or tool content remains unresolved.
Unknown-cost metadata preserves the full reservation. Known-cost metadata can settle its original entry while the answer remains unresolved.
The original seven coordinator tests use a metadata adapter fixture.
The later native HTTP cases exercise both actual databases, current budget-grant denial and restart without another generation.

Concurrent manual HOLD can prevent the current rate-limit path from persisting its original Retry-After floor.
The current safe result is held unknown. Proposed `reconciled_not_accepted_at` must freeze that original floor atomically with delayed Work.
It cannot erase evidence, reset counters, weaken current authority or extend expiry.

## Proposed adapter interfaces and transport

Keep the accepted `OpenRouterConfig`, `CredentialSource::resolve`, `SecretToken`, `OpenRouter::new` and `Provider` facade names.
Configuration freezes an approved HTTPS origin, bounded credential reference, catalog TTL and response limit.
Add host-only endpoint-discovery IDs through a validated method, subject to the catalog seam review above.
`SecretToken::new` rejects empty, oversized or invalid header values. Debug reveals no token or reference.
Resolve credentials inside the current deadline. Never store credentials in a Resource, error, body fixture, observer or exported view.

Disable redirects, system proxy, decompression and retry behavior explicitly.
Use `retry(reqwest::retry::never())`, `no_proxy()` and redirect policy `none()`.
These explicit controls remain necessary when workspace feature unification enables additional transport capabilities.
Read body chunks with a checked accumulated limit. Bound response headers, selected metadata and decimal lexemes.
Production configuration cannot select arbitrary URLs. A gated `test-support` constructor admits loopback HTTP and HTTPS.
Do not disable certificate validation. OpenRouter uses Bearer authentication; the reviewed contract declares no request-body HMAC.
[Versioned client behavior](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html)

## Dependency proposal and evidence

The coordinator admitted existing locked versions. Earlier package metadata came from `/var/tmp/rom-010-ai-reservations-resolved-metadata.json`.
The adapter's newly resolved graph is `/var/tmp/rom-010-openrouter-resolved-metadata.json`; final notice and integration review remain open.

| Dependency | Proposed use | Exact source evidence |
| --- | --- | --- |
| reqwest 0.13.5 | Defaults off; rustls only; manual serde bodies, no SDK | MSRV 1.85.0; MIT OR Apache-2.0: [versioned manifest](https://raw.githubusercontent.com/seanmonstar/reqwest/v0.13.5/Cargo.toml) |
| httpdate 1.0.3 | Existing graph; Retry-After HTTP date parsing | MSRV 1.56; MIT OR Apache-2.0: [versioned manifest](https://raw.githubusercontent.com/pyfisch/httpdate/v1.0.3/Cargo.toml) |
| serde_json 1.0.151 | Existing workspace version; add `raw_value` feature only | Local manifest: MSRV 1.71; MIT OR Apache-2.0; local `src/raw.rs` preserves original JSON lexemes; [primary RawValue contract](https://docs.rs/serde_json/latest/serde_json/value/struct.RawValue.html) |
| Tokio and serde | Existing workspace versions and features | No new runtime or serializer version |
| sha2 0.10.9 | Existing graph; only if a stable bounded catalog identity needs a digest | Existing source and manifest require review before direct admission; no request hashing needed for exact neutral equality |

Parse prices and costs with checked decimal arithmetic and upward rounding. Do not use `f64`.
Catalog token prices must be scaled to USD per million tokens before nanodollar rounding.
Deserialize numeric cost through `RawValue`; converting a parsed floating-point `Value` back to text loses the original decimal evidence.
Do not enable `arbitrary_precision`, which can change ordinary shared JSON number behavior.

The reqwest maintainer page showed no published security advisories on the research date.
RustSec package-directory checks returned 404 for reqwest and httpdate; this is limited package evidence, not a complete graph audit.
[Maintainer advisories](https://github.com/seanmonstar/reqwest/security/advisories), [RustSec database](https://github.com/RustSec/advisory-db)
The existing lock's rustls-webpki 0.103.15 exceeds the 0.103.10 fix for the reviewed CRL advisory.
[RUSTSEC-2026-0049](https://rustsec.org/advisories/RUSTSEC-2026-0049.html)
An additional read-only review used RustSec tree `b8a1a33e246a0a9a3b5f377248c41a503defec74`.
The existing metadata closure for reqwest, serde_json and httpdate contains 175 registry packages across all resolved features and targets.
Its 73 matching advisory files were fetched from that exact primary tree. The reviewed locked versions satisfy published patched or unaffected ranges.
The ring maintenance advisory is withdrawn; do not report it as an active vulnerability.
Rustls 0.23.45 meets the latest reviewed TLS handshake fix exactly.
[Versioned RustSec source](https://github.com/RustSec/advisory-db/tree/b8a1a33e246a0a9a3b5f377248c41a503defec74), [rustls advisory](https://rustsec.org/advisories/RUSTSEC-2026-0285.html)
Evidence: `/var/tmp/rom-openrouter-existing-graph-advisory-scope.json` and `/var/tmp/rom-openrouter-existing-graph-primary-advisories.json`.
This is a source/range review, not an executed cargo-audit run. It precedes the new adapter's exact feature resolution.
Record the newly resolved feature graph and license notices before manifest admission.
The normal local verifier alone must not be described as a dependency security audit.

## Task 5 durable tool implementation seam

The accepted `ToolRegistry::new/read/action`, `ReadContext::read/query` and `PreparedToolAction` contracts remain the target.
`rom::Input::descriptor()` can return None. Opaque input cannot become a fabricated JSON schema.
Reject opaque action registration unless the host provides an explicit versioned descriptor and the actual decoder remains decisive.
`rom::Action` hides its name. Freeze a validated registry name/version and Resource kind; never recover action identity by Debug parsing.
Inspect registered metadata before choosing a narrow public accessor or an explicit host binding. Core authority is unchanged.

Proposed host composition: optional `FlowBuilder::tools(registry, authority)` with a separate current trusted `ToolAuthority`.
This preserves existing `FlowHost::new` callers. It replaces the stale proposed mandatory `FlowAuthority::tool_actor` method only after coordinator review.
The tool authority resolves an authenticated actor through bounded `Runtime::establish_actor` with trusted reads.
The model supplies no actor, service, endpoint, registered action name or budget grant.

Persist a bounded pending tool batch before dispatch. Preserve native call IDs and validated generation evidence.
Before each action, freeze call ID, registry version, target, action version, expected revision, typed input, idempotency key and retry epoch.
Read the current authorized retry epoch before freezing it. Replay the original frozen epoch through the existing Runtime receipt path.
Commit the prepared command first. Execute the Resource action second. Commit the tool result and delayed successor third.
These are explicit stages; there is no cross-Resource transaction promise or parallel tool receipt store.
On lost acknowledgement, recover the original command receipt. Never regenerate the model response or substitute current revision/input.

Execute one call per existing Work tick. Bound response calls at eight, run calls at sixteen and ticks at thirty-two.
Keep continuation within the existing 32 KiB request and 256 KiB private record limits.
Refresh current tool and owner grants before execution, receipt replay and result projection.
Cancellation before dispatch causes no action. Cancellation after action uncertainty retains its prepared identity and hidden late result.
ReadContext exposes no mutation handle. Registered read closures remain a trusted host responsibility; arbitrary HTTP and shell effects are unsupported.
Any narrow run-state, checkpoint or worker extension requires source review before changing frozen Task 2 or Task 3 files.

## Finite offline acceptance plan

Task 4 uses real reqwest requests to bounded loopback services. Retain intended RED before implementation.

| Test family | Required negative cases and evidence |
| --- | --- |
| Admission and credentials | Invalid origin, user info, path/query injection, CRLF token, empty/oversized token, expired deadline, revoked credential resolution; zero POSTs |
| Transport | Redirect target receives no token; proxy environment ignored; untrusted TLS certificate rejected; retry-disabled server sees one POST; delayed headers/body/EOF consume one deadline |
| Catalog | >4 MiB, >4096 models, duplicates, malformed/negative/exponent prices, sub-nanodollar rates; independent refreshes; unknown provider membership never passes a required whitelist |
| Schema and routing | Exact original schema, unsupported version, ignored capability, returned wrong model, provider outside whitelist, changed price; no implicit fallback or response-healing plugin |
| Error knowledge | HTTP429 vs HTTP200 body429; 401/403/402/400/503; missing/invalid/date/overflow Retry-After; truncated 200 and unexpected SSE remain unknown; no second POST |
| Output and tools | Empty cold-start and reasoning-only output, malformed JSON, oversized content, duplicate/missing IDs, >8 calls, mismatched tool finish reason, paired continuation and repeated definitions |
| Costs and lookup | Exact decimal lexical cost, missing/negative/overflow cost, metadata-only result, no ID, mismatched ID and 404; no fabricated output/nonacceptance, no free unknown reservation |
| Redaction | Debug, Display and observer cannot reveal planted secret, prompt, raw metadata, URL or completion |

Task 5 runs the same behavioral cases on actual SQLite and redb adapters.
Include forged typed input, denied target, grant revocation, changed receipt identity, crash before command and lost command acknowledgement.
Include wrong retry epoch, duplicate call ID, tool-count exhaustion, expired run, cancelled late action and denied projection after restart.
An external compile fixture proves ReadContext has no mutation method.

Task 6 uses two separately compiled public consumers.
Publication creates an immutable prepared revision, then changes a head through separate authorized actions.
Interrupt before and after each commit and acknowledgement. Old cited revision content remains readable under current authority.
Triage reads an authorized Task, validates classification and invokes a registered action.
Test denied action, restart, cancellation, unknown cost and exhausted budget through the same public facade.
Consumers provide their domain prompts and validators. Neither implements a scheduler or reaches private run codecs.
Production adapter construction must compile in both examples; tests use real loopback HTTP without paid calls.

## Astral Plane traceability and remaining acceptance

| Intake groups | Generic acceptance evidence to add | Consumer-owned requirement |
| --- | --- | --- |
| AP-UX-007/008 | Committed prepared/tool/completed stages, due queue and reopened automatic resume | Stage labels and meaningful progress presentation |
| AP-UX-013/026 | Current grant before provider/tools/projection; bounded authorized reads | Which private context each application may disclose |
| AP-UX-014/015/016 | Bounded tools, deduplicated call identity, authorized source references and real action receipts | Grounding strategy, relevance and interpretation quality |
| AP-UX-018/022/023/024 | Durable history, immutable publication references, new run identity distinct from receipt replay | Historical dashboard source navigation and explicit fresh execution UI |
| AP-UX-027 | Free then explicitly capped paid routing, finite retry, unknown charge retained | Local CPU fallback implementation and content policy |
| AP-UX-029 | Validator seam and unchanged domain ownership | Application prompts, current source selection and output quality |

Withdrawn AP-UX-030/032 stay excluded. No neutral, loopback or fixture success closes original-consumer acceptance.
The shared coordination handoff still requires consumer acknowledgement and concrete reproductions.

Implementation order: resolve reviewed contract gaps; admit exact existing graph; record Task 4 RED; implement adapter; verify focused tests and Clippy.
Then review typed tool state and authority before Task 5 RED. Complete both public consumers before the coordinator's full verifier and integration review.

## Native metadata and advancing-time evidence checkpoint

The current combined candidate passed 74 AI cases and 33 adapter cases, plus targeted Clippy.
Evidence: `/var/tmp/rom-ai-openrouter-task4-observation74-native33-green-clippy.log`.
Frozen source inventory: `/var/tmp/rom-ai-openrouter-task4-observation74-native33-source-hashes.txt`.
The earlier inventories and intended failures remain preserved.

Native metadata returns `ReconciliationObservation::Uncertain`, retaining exact original identity and bounded authoritative usage.
The HTTP tests reject foreign model/ID, negative cost, ceiling amplification and excess native completion counts.
Both database flows settle known cost without publishing an answer. Budget-grant revocation prevents GET while owner inspection remains allowed.
A successful original operation replay performs no second GET or POST after restart.
POST requests explicitly disable OpenRouter response caching; they never enable input/output logging.
[Native metadata contract](https://openrouter.ai/docs/api/api-reference/generations/get-request-&-usage-metadata-for-a-generation), [cache precedence](https://openrouter.ai/docs/guides/features/response-caching).

The actual real-clock flows exposed a milestone timestamp invariant failure hidden by earlier fixed-clock tests.
Two deterministic advancing-clock regressions first failed with the declared Resource contract error.
The knowledge action now derives the latest existing milestone timestamp through `mark()` after its monotonic observation update.
Its clone-equality transition permits exactly that derived change, preserving earlier milestones, sequence, state, counters, attempts and financial account.
These tests do not authorize arbitrary milestone changes or an additional stage.

AP-UX-007/008/027 remain open when a queued discovery/grant denial permanently stops Work without an owner-visible run failure or safe public retry.
The operator contract is not sufficient consumer ergonomics. The queued-step0 seam needs reviewed design and actual tests.
A host test reproduced accepted-header loss when the earlier outer callback deadline cancels the body on both databases.
Both cases failed only the final durable generation ID assertion in `/var/tmp/rom-openrouter-task4-real-host-supervisor-red.log`.
Exact prepared identity, conservative reservation, restart and one POST remained intact.
The worker already caps the prepared deadline at tick-now plus 18 seconds; that cap starts after initial reads and authority.
The coordinator approved an additive monotonic execution budget and persistence grace; the focused implementation and regressions now pass.
The 18-second callback has a 16-second execution end, a 250-millisecond adapter observation margin, and two seconds for persistence.
Manual lookup retains one shared two-second authorization/queue/HTTP budget and a 250-millisecond observation margin.
No frozen attempt, policy, account or core semantics were changed for this design.
Legacy providers that ignore the context and storage stalls beyond the grace can still lose knowledge; unknown charges remain held.
AP-UX-015 research deduplication and AP-UX-023/024 retained context versus fresh execution remain separate tool/consumer acceptance requirements.
AP-UX-033 guest invitations remain application-owned. No invitation domain workflow is copied into these crates.

## Shared execution deadline and valid tool knowledge checkpoint

The missing public deadline API first failed with E0432 and E0599 in `/var/tmp/rom-ai-task4-execution-missing-api-red.log`.
Four neutral contract cases then passed finite bounds, monotonic copies, exhausted-context no-call behavior and compatible legacy delegation.
Real HTTP tests separately reproduced generation exceeding the shared execution end and metadata lookup restarting its own two-second budget.
Those two failures remain in `/var/tmp/rom-openrouter-task4-execution-behavior-red.log`; its zero-POST/GET exhaustion case already passed.
The adapter now uses the same caller context, with its observation margin, while preserving the original prepared UTC expiry and identity.
The original two supervisor failures now pass in 15.94 seconds, retaining the known header ID through restart with one POST.

Two actual-database regressions also reproduced valid requested-tool completions losing known provider identity and cost.
The earlier malformed tool fixture exceeded a free reservation and did not cover this validated branch.
The correction persists exact validated evidence and usage through the existing hold coordinator, then settles known cost.
It executes no tool, publishes no output and repeats no generation. Reopened lookup still requires current budget authority.
Intended failures: `/var/tmp/rom-ai-task4-valid-tool-knowledge-red.log`.

The complete focused run passed 80 AI and 38 adapter cases, plus both crates' all-target Clippy.
Log: `/var/tmp/rom-ai-openrouter-task4-execution80-native38-green-clippy.log`.
Frozen inventory: `/var/tmp/rom-ai-openrouter-task4-execution80-native38-source-hashes.txt`, SHA256 `381f5e12624390fa41f6ee7b0e06011ec72a93b1bfe3115caca863a3cd2cd235`.
The earlier 107-case inventory and all intended failures remain preserved.
Dedicated catalog/permit delay cases, optional content recovery, queued failure/retry, typed tools and both external consumers remain required.
Legacy providers that ignore the context and process/storage stalls beyond the grace can still lose knowledge.
No internal fixture result establishes original-consumer acceptance.

## Queued preparation failure candidate

Four actual-database cases first failed because discovery or current budget denial left the owner-visible run Queued.
Evidence: `/var/tmp/rom-ai-task4-preparation-visible-failure-red.log`.
The separate missing action contract failed only E0432 in `/var/tmp/rom-ai-task4-preparation-action-missing-api-red.log`.
The service-private preparation action now records a sanitized Failed view at checkpoint zero without inventing provider evidence or modifying account money.
The coordinator refreshes current owner authority and exact immutable input before each revision-checked commit and bounded CAS retry.
The new path catches catalog, selection and current attempt-grant failures before ReserveBudget; it does not reinterpret started or unknown generation.

Both database journeys reopen and replay the original submission without generation or history changes.
Only an explicit new identity executes fresh work after eligibility or current grant changes.
A separate shared test preserves an exact account-first Reserved10 entry while failing its still-Queued run.
Owner forgery changes no journal fact; exact action replay adds no event; a changed cause under the same receipt key fails IdentityMismatch.
The initial orphan fixture processed only mapper materialization, so its admission Conflict is retained as fixture timing, not product RED.
An existing grant regression expected the previous stranded Queued state; its reviewed update requires Failed and Denied while retaining every zero-call/account assertion.

The complete candidate passed 85 AI tests and 38 adapter tests, plus both crates' all-target Clippy.
Evidence: `/var/tmp/rom-ai-openrouter-task4-preparation85-native38-corrected-green-clippy.log`.
Inventory: `/var/tmp/rom-ai-openrouter-task4-preparation85-native38-source-hashes.txt`, SHA256 `542c7db0035bbf1c51609b154302250408d82a5e9aae427d4456963fd1e78ab9`.
Capacity, missing-account and post-Prepare denial diagnosis need separate coverage; this checkpoint does not close every pre-dispatch failure.
Typed tools, both external consumers, optional content recovery, full-verifier and original-consumer acceptance remain open.
