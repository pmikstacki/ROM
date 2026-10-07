# ROM 0.1.0: optional AI routing and durable flows

Date: 2026-10-07. Status: source investigation and proposals only.

## Evidence boundary and source identities

This investigation read source and retained evidence. It did not run tests, builds, installs, provider requests, or deployments. No secrets were read. Only this scratch report was written.

ROM HEAD: `d7ef529040eec60dc869034c2d33130219db85fe`.
ROM lock SHA-256: `a080b892d06dd30e65917a9a13715918cbd2b02fe64827f0334d7bde7005b441`.
Its working tree initially contained three untracked research documents: the consumer feedback, research plan, and source review.
Astral Plane HEAD: `86dca6fa514895f94fd09ea788ebf07fa21cf648`. Its working tree was clean.
Astral lock SHA-256: `4dd0bb900581925cd82ada55aa60b98a9f19481ace5e1186055a4c5b70c28c82`.

Read ROM quality/writing rules, AI workflow map, 0.1.0 consumer feedback, research plan, and consumer source review. The research skill's source-first requirements inform this report. The parent assigned investigation as a separate background task and fixed ownership to this file.

Selected Astral source hashes:

| File under `/root/astral-plane/` | SHA-256 |
| --- | --- |
| `src/interpretation/openrouter.rs` | `a342ce0083a36bab5c09add3960ee18e25a97a6fc446a25019011a58f986b09c` |
| `src/interpretation/openrouter_catalog.rs` | `685cadb229d5b112ef1b325da337a44d28b5822fbdbf60d08d305bbcc2541fe3` |
| `src/interpretation/model.rs` | `fece39dd7198c43907aeba0318f54243ccde444e50d6cc08274ea30abfbe49d6` |
| `src/interpretation/research_budget.rs` | `77116727c22a3b3541688acc9ed500913c466c92b614cc37d712d0f59537c331` |
| `src/interpretation/research_search.rs` | `7383865b2d8405bc50ff1f9b340793a683010d5cb8efebdc315a2a83549871a8` |
| `src/interpretation/service.rs` | `d7356d581cccc242f2bfb6f0752d3d7b3b02eb5ab2161883a8009a3300f331af` |
| `src/interpretation/tools.rs` | `364e058ef9583c2a96bd3eae4d20fe8355bab562f078e75d59f1fbc3941dd2e1` |
| `src/enrichment/pipeline.rs` | `8b5dd710c7ff8e49db2fc08a2a57b9f68c41712021bf4f5f4085db052de1cd8b` |
| `src/enrichment/worker.rs` | `91d464d75738683fef553d48fcd49b919171315d096e1a912f7250a2cb6440cb` |
| `src/knowledge/publishing.rs` | `b5bc46ce8616d4e181825ca2f5c63f07a3c18da3625961510d3a1cb49488b7e5` |

Other projects were not scanned. No bounded named comparison implementation was supplied. ROM and Astral already supply concrete code for this decision.

## Findings from consumer code

Paths in this section are under `/root/astral-plane/`.

| Concern | Inspected behavior and existing correction | Remaining gap or proposed experiment |
| --- | --- | --- |
| Deadline splitting | `src/interpretation/openrouter.rs::select_tier` gives a batch the remaining duration, capped at ten seconds and the configured timeout. It no longer divides time by all catalog candidates. Native fallback batches contain at most three IDs; two named models are isolated for reasoning controls. | Discovery runs before the free deadline calculation. `openrouter_catalog.rs::discover` uses an independent five-second timeout, and mutex acquisition is outside that timeout. Test delayed discovery and concurrent endpoints within a total job deadline. |
| Retry continuation | A bounded request stores the next model before dispatch. `model.rs::complete_with_fallback` returns `remote_budget_exhausted` to the queue without starting CPU inference. Tests include `exhausted_free_deadline_continues_candidates_before_paid` and `continued_free_round_exhausts_before_reserved_paid_attempt`. | Cursor identity is only endpoint/schema/tier. It is shared by every run in the process, persists after successful intermediate batches, clears at 64 entries, and disappears on restart. Another run can inherit the cursor or skip free selection because a paid cursor exists. Persist continuation per run and policy version; keep shared provider health distinct. |
| Invalid JSON schema | The adapter now calls `get_mut` before removing selection `uniqueItems`. It no longer inserts a null property into conversation/tool/intent schemas. `non_selection_schemas_are_preserved_without_inserting_null_properties` covers this defect. | A provider schema subset is not the application contract. Local validation still rejects duplicate or unknown source IDs. Version schema adaptation and reject unsupported requirements explicitly. Do not silently weaken domain validation. |
| Catalog and prices | `openrouter_catalog.rs` caches success or failure for 300 seconds, caps models at 4096 and response bytes at 4 MiB, requires text modalities and schema support, rejects batch IDs and request fees, and orders discovered paid models by price. Catalog prices are divided against per-million configured caps. Request-time `provider.max_price` supplies the same ceilings. | One global async mutex is held across network discovery, so one endpoint delays others. Five-minute negative caching delays recovery. Missing discovery accepts explicitly configured paid IDs, relying on request enforcement. Model catalog eligibility does not prove endpoint capability or availability. |
| Paid authority | `types.rs::OpenRouterConfig::validate` bounds finite prices to USD 0.10/M input and 0.40/M output, plus model-list sizes. Paid fallback is optional. | These are model rate limits, not account/day/run spend limits. The response parser retains model provenance but drops generation ID, usage and cost. Persist worst-case reservations before each potentially billable attempt, including invalid output and lost replies. |
| Cold loading | Retained `docs/evidence/2026-10-07-ai-retention.md` reports a cancelled 90-second Ollama cold load. Its correction retained the model and aligned then-current context sizes. It reports real requests and tests, which this investigation did not reproduce. | Current `model.rs` selection uses 16384 context tokens, while generic local completions use 4096. The old shared-context claim does not cover all current paths. Semaphore waiting precedes a 45-second transport timeout in `local_model.rs`. Test startup, queue wait, context switches, and restart under one outer deadline. Preloading belongs to the local adapter/host. |
| Tool plan | `tools.rs` generates a structured plan, then validates at most five distinct known tools and dates/topics. `execution.rs` prepares once within the in-memory run and returns durable research requests separately. | This is application plan generation, not an implemented general OpenRouter native tool-call loop. Introduce a trusted tool registry with schemas, bounds, current authority, and effect classification. Planning does not grant permission. |
| Progress | `workflow.rs::Step` contains id/status/detail. `service.rs` stores progress and attempt count in memory, renews a five-minute observation lease, and retries while observed. Completed readings commit as Resources with run identity. | Restart loses intermediate model state and consumed attempts. A continuously observed run can retry without a total durable limit. Generated narration is presentation, not an authoritative event. Separate durable milestones from transient token/narration output. |
| Search reservation | `research_budget.rs` creates immutable daily slot Resources before search. Concurrent claimants use distinct receipt identities. `research_tests.rs` contains restart/concurrency and denied-update tests. | It is a useful conservative reservation pattern, not a generic money ledger. Uncertain writes stop the search; do not reclaim a reservation solely because the caller timed out. Clarify day boundaries, retention, limit changes, and aggregate multi-instance assumptions. |
| Search security | `research_search.rs` disables redirects, limits time and response bytes, normalizes queries, filters public HTTPS source URLs, preserves partial results, and reserves failed attempts. | Query normalization does not itself prove absence of private text; callers provide canonical public topics. URL checks are for source references, not a complete SSRF defense for future page fetches. A fetch adapter needs DNS/address/redirect enforcement. |
| Enrichment scheduling | `enrichment/worker.rs` scans bounded pages of 64, advances past terminal/delayed rows, wraps, polls, and increments persisted attempts. `worker_tests.rs` covers scanning past terminal pages. | The worker has no ROM Work claim lease and no terminal attempt cap. Multiple coordinators can still duplicate external search before a revision conflict. Replace duplicated coordination only after a ROM channel/work composition fixture demonstrates equivalence. |
| Prepared publication | `enrichment/pipeline.rs` persists `PreparedPublication` before `catalog.publish`. `knowledge/publishing.rs` freezes expected revision, payload and idempotency. Publication first commits immutable revision, then head; those are two commits. | Restart replay is useful, but it is not atomic two-Resource publication. Intermediate revisions can remain after head failure. Retained `persisted_prepared_publication_resumes_after_restart_without_search` opens SQLite again and asserts publication without search; it was not run here. Inject interruption around each command and acknowledgement. |

The exact provider corrections are documented in `docs/ai-reflection-and-knowledge.md`. Existing tests are source evidence of intended coverage, not fresh passing evidence.

## Existing ROM seams

Paths are under `/root/ROM/`.

`crates/rom/src/channels/model.rs` supplies typed versioned payloads, stable delivery IDs and explicit outcomes: accepted, retryable, permanent, unknown, timed out and panicked. `channels/registration.rs` accepts async host callbacks and validates canonical payloads. Default delivery timeout is five seconds. An AI host must configure a measured deadline; a ten-second adapter call cannot assume the default channel supervisor permits it.

`channels/execution.rs` checks current service authority and source disclosure, records delivery start, runs outside the core gate, then records its result. Cooperative cancellation aborts and joins the callback. It does not preempt blocking Rust.

`reactions/declarations.rs` maps committed source snapshots into typed target actions. `reactions/receipt.rs` resolves the original frozen action through durable receipt identity and current replay authority. It already prevents treating lost acknowledgement as permission to create a new action identity.

`reaction_work/model.rs` persists attempts, causal root, start time, definition/version, service identity, due time, delivery profile and fencing generation. Default bounds are depth 16, root work 256, attempts 3, age 3600 seconds, fan-out 16, records 1024 and bytes 1 MiB. These are existing defaults, not proposed AI support limits.

`reaction_work/control.rs` keeps original budgets during retry. `reaction_work/delivery_profile.rs` holds unknown/timed-out/panicked deliveries for reconciliation when selected. `/root/ROM/docs/operator-recovery.md` explains authorization, exact retry identity, provider verification and independent compensation actions.

`crates/rom-auth/src/profile.rs` defines identity credential profiles. Those establish principal kind; they are not AI routing policy. Provider policy, tool grants, and secret references should use application/configuration Resources plus host adapters. Reuse existing authority and configuration contracts instead of calling an OpenRouter model profile an authentication profile.

ROM supplies the scheduler and commit semantics. The investigation found a composition/discoverability opportunity and adapter gaps; it did not establish that core lacks durable work.

## Primary OpenRouter documentation

Read official documentation after consumer source inspection. These live docs were accessed on 2026-10-07; no provider capability was tested here.

- `models` requests specify ordered model fallbacks. Provider failure handling does not validate domain citations or application semantics. Preserve actual returned model. [Model Fallbacks](https://openrouter.ai/docs/guides/routing/model-fallbacks).
- Model fallback and provider selection are separate controls. Use provider allowlists, `require_parameters`, disclosure policy and request price caps where applicable. Price caps use dollars per million tokens; latency preferences are not hard deadlines. [Provider Routing](https://openrouter.ai/docs/guides/routing/provider-selection).
- Models expose pricing, supported parameters, modalities and context metadata. Treat that response as discovery input, with bounded stale handling. [Models API](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties).
- Structured output support varies by endpoint and can change. Strict schema support has provider-dependent restrictions. Local validation remains necessary. [Structured Outputs](https://openrouter.ai/docs/guides/features/structured-outputs).
- Native tool calls require local execution and linked `tool_call_id` results. Include tools again on the continuation request. The host remains responsible for execution. [Tool Calling](https://openrouter.ai/docs/guides/features/tool-calling).
- Inspect error bodies even after HTTP 200. Preserve typed errors and bounded retry guidance instead of collapsing every failure to model unavailability. Authentication, invalid input, rate limits, unavailable routes and failed generation need different handling. Streaming can fail after partial output. [Errors and Debugging](https://openrouter.ai/docs/api_reference/errors-and-debugging).
- Generation metadata can identify provider, token usage, cancellation and total cost when a generation ID is available. It does not establish a deduplication contract for lost chat-completion responses. [Generation Metadata](https://openrouter.ai/docs/api/api-reference/generations/get-request-&-usage-metadata-for-a-generation).

No inspected source or these docs establishes exactly-once OpenRouter chat generation. Do not select `ProviderDeduplicated` merely because ROM passes a stable ID. A metadata lookup by known generation ID cannot resolve every request whose response and ID were lost.

## Proposed optional composition

Names below are candidates, not adopted crates or implemented capabilities.

| Boundary | Proposed ownership |
| --- | --- |
| `rom-ai` | Provider-neutral bounded request/result types, capability requirements, validation hooks, route decisions and typed failure categories. No HTTP or database driver. Small explicit contracts before a broad agent framework. |
| `rom-openrouter` | HTTP adapter, catalog/endpoint capability mapping, credential reference resolution, provider policy, schema subset translation, usage IDs and bounded error parsing. Own dependencies here. |
| `rom-agent` or a narrow flow module | Resource-backed run/step/checkpoint and attempt reservation composition over existing actions, channels, reactions and operator recovery. Evaluate as a public example before extracting a helper. No second scheduler or private ledger access. |
| Host registration | Trusted tool implementations, current grants, provider endpoints, secrets, allowed model policy and transport bounds. Keep blocking computation outside async workers and transitions. |
| Studio/client helper | Authorized run projection, submit/retry/reload identity handling, milestones, optional ephemeral output stream, and clear unknown outcome. No credentials, route selection or tool authority in browser. |
| Application | Domain prompts, fact/citation validation, publication licensing, approved topics, business decisions, compensation definitions, and public/redacted projection fields. |

A useful author surface is: declare run Resource and tool registry; submit a typed action; attach a channel adapter; observe an authorized run projection; recover through existing receipts and Work controls. The implementation must show which contracts remain user-authored. Avoid hiding idempotency, authorization, or unknown outcome behind automatic retries.

## Durable and transient output

Persist prepared input and policy/schema versions before external effects. Persist attempt reservation before dispatch. Persist provider identity, generation ID when available, validated result, tool command identity, and confirmed checkpoints through ordinary Resource actions.

Only committed mutations produce ROM events. Model tokens are provisional bytes, not Resource facts. A bounded stream can show transient tokens; reconnect should return the last durable checkpoint and a documented stream reset. If audit requires retained text, commit bounded chunks or a completed artifact under an explicit retention/disclosure contract. Do not write an event per token by default.

A channel callback that receives model output must persist the result through a documented host composition. `DeliveryOutcome::Accepted` alone carries no typed output and does not constitute a completed run result. Test nested Runtime calls and admission interaction before promising this ergonomic path.

## Budgets, authorization and effect safety

Use durable fixed-unit or exact-decimal money amounts, not float accumulation. Freeze each reservation's policy version, ceilings, input/output bounds and attempt identity. Native provider fallback remains within one reserved request envelope; application retries require additional reservations. Keep uncertain costs conservatively charged until authoritative reconciliation.

Dynamic policy changes may reduce future spend or revoke disclosure. They cannot erase consumed attempts or silently upgrade an old run's paid authority. Define rules for ongoing work and policy-version mismatch. Bound chain depth, work count, age, fan-out, total provider/tool calls, concurrent calls, prompts, output bytes and stored checkpoints independently.

Recheck current authority at each tool execution and disclosure. A model may choose among granted tools; it cannot select another principal, endpoint, secret path or Resource outside the host's target contract. Side-effect tools use prepared commands and stable idempotency identities. Unknown outcomes stop blind effect retries; compensation remains a separate explicit action.

The current OpenRouter adapter reads a configured key file and uses a configured endpoint. `OpenRouterConfig::validate` does not validate endpoint trust or secret paths. A reusable host boundary must constrain these settings. Do not expose arbitrary endpoint changes or filesystem secret selectors to ordinary client input.

Minimize exported private context. Redact prompts, tokens, tool arguments, provider errors and trace labels. Untrusted source data and structured JSON remain untrusted; schemas do not remove prompt injection or grant tool authority.

## Offline experiments before selection

All experiments below are proposed. They were not executed in this investigation.

1. Extract a public consumer with no vendor aliases. Compose prepare → generate → validate → publish → follow-up through ROM channels/reactions on SQLite and redb.
2. Use a local fake provider. Compare many catalog candidates with slow-valid response, stuck catalog, stalled body, oversized data and concurrent endpoints. Assert total deadline, useful attempt duration and bounded work.
3. Submit two independent runs with the same endpoint/schema. Restart during retries. Assert isolated durable continuation, preserved attempts, no global paid-tier inheritance, and no repeated invalid model within the same run.
4. Inject schema subset errors and HTTP 200 error envelopes. Assert typed categories, permanent request errors, rate-limit delay and invalid-output exclusion. Exercise native tool-call finish reasons separately from `stop`.
5. Change live price/capability fixtures and dynamic policy versions. Race reservations at a small spend limit. Lose responses. Assert no unreserved dispatch, conservative cost retention and no paid escalation without authority.
6. Exercise a tool read, an idempotent mutation and a non-deduplicating external effect. Revoke grants between planning/execution/replay. Test wrong IDs, oversized arguments, malformed results and loop limits.
7. Interrupt before/after prepared state, external dispatch, result commit, publication revision, publication head and acknowledgement. Preserve original identity. Assert one publication when its receiver supports deduplication, with documented partial-commit states.
8. Test channel deadline versus adapter deadline, queue admission, cold startup and shutdown. Verify cancellation retains execution capacity until callback termination.
9. Reload an authorized frontend progress projection. Reconnect transient output after restart. Assert no provisional token becomes a success event or leaks across principals.
10. Repeat the same flow with an unrelated non-AI consumer. Measure author code, internal imports, policy clarity and recovery effort. Run affected checks and full ROM verifier only after authorized implementation.

## Rejected copying

Do not copy Human Design/transit algorithms, topic vocabulary, source-domain allowlists, citation matching, publication licensing or reflection prompts into ROM core. Their guarantees belong to Astral Plane.

Do not copy process-global cursors, repeated polling coordinator, generic string failures, or old cold-start support claims as production contracts. Reuse the observed cases to define tests.

Do not equate free-first routing with correct answers, rate caps with spend caps, generated narration with facts, a saved draft with atomic publication, or process restart with disaster recovery.
