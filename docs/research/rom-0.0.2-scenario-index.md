# Prepared 0.0.2 scenario and support index

Date: 2026-10-04. This index identifies scoped evidence, not a completed release.
The historical [native-alpha scenarios](framework-release-scenarios.md) retain their original acceptance scope.
Do not use their six-gate result as evidence for the current eight-gate profile.

## Current scoped evidence

| Scenario | Evidence | Boundary |
| --- | --- | --- |
| Typed Resources use shared browser operations | [Full workflows](rom-0.0.2-full-browser-workflows-results.md) | Task and Inventory; actual provider; both stores and browser engines. |
| Live observers receive another page's mutations | [Full workflows](rom-0.0.2-full-browser-workflows-results.md) | No extra observer query requests in the recorded flow; no universal latency bound. |
| Logout clears two views and terminates live delivery | [Session and rendering measurements](rom-0.0.2-multi-tab-and-renderer-results.md) | Shared browser session; recorded periodic clearing and stream termination. |
| Descriptor changes preserve drafts and block invalid submission | [Draft acceptance](rom-0.0.2-draft-acceptance-results.md) | Explicit reopening and focused validation; no general server-error focus claim. |
| Inventory accepts scalar actions and rejects overflow | [Inventory action](rom-0.0.2-inventory-action-results.md) | Common invocation path; durable replay and unchanged counts after rejection. |
| Human identity survives the declared maintenance journey | [Provider maintenance](rom-0.0.2-provider-maintenance-journey.md) | Backup, restore, migration, disk reopen, fresh login, replay, and current denial on both stores. |
| Attachments preserve bytes across process interruption | [Attachment lifecycle](rom-0.0.2-attachment-lifecycle-results.md) | Actual SIGTERM/restart acceptance and corruption control; no power-loss guarantee. |
| SDK rejects malformed values and inconsistent responses | [SDK resilience](rom-0.0.2-sdk-resilience-results.md) | Recorded wire and lifecycle cases; current authority remains a server responsibility. |
| A custom renderer works from independent source extraction | [Studio author workflow](rom-0.0.2-studio-author-workflow.md) | Public source imports, actual provider, both stores and engines, and source-drift rejection. |
| Actual wire responses support parser comparison | [Parser measurements](rom-0.0.2-wire-parser-results.md) | Small seeded responses; adoption-order deviation recorded; no universal speed claim. |
| Production assets include source-bound notices | [Runtime notices](rom-0.0.2-runtime-notices-results.md) | Scoped fresh build and notice admission; final producer binding remains pending. |
| Local supervision rejects invalid process limits | [Process admission](rom-0.0.2-process-admission-results.md) | Pre-spawn rejection and 57 focused tests; no aggregate disk bound. |
| Independent review examines shared acceptance invariants | [Final slices review](rom-0.0.2-final-slices-review.md) | Source/evidence inspection and named mechanism probes; no independent full producer run. |

Each linked report identifies its inputs, failures, executed checks, and limits.
Earlier counts remain historical results. This index does not change them.

## Pending complete acceptance

| Obligation | Evidence necessary for completion | Current state |
| --- | --- | --- |
| Complete local acceptance | All eight fixed commands from one clean admitted revision | Pending; producer deferred for shared capacity. |
| Release artifact identity | Exclusive completed output, source/asset archives, manifest, checksums, and independent extraction verification | Pending. |
| Extracted native consumers | Current complete package gate with external library, CLI, reference application, and resolved-path audit | Pending final execution. |
| Preview input binding | Final source, native executable, production assets, and pinned provider tree bound together | Pending. |
| Persistent protected preview | Scoped activation and actual trusted HTTPS login, mutations, streams, attachments, and restart checks | Pending; old mock remains active. |
| MacBook VPN access | Independent client route, connection, and certificate-trust evidence | Unverified; local host checks cannot prove it. |
| Main integration | Accepted source integrated and pushed with the final release identity | Pending. |

Use the [fixed producer procedure](../../scripts/release-artifacts/README.md) for complete local acceptance.
Use the [scoped preview procedure](rom-0.0.2-scoped-preview-activation.md) for the selected existing-container installation.
The [progress record](rom-0.0.2-progress.md) preserves earlier failures and later corrections.
The [support boundary](../release-support.md) distinguishes prepared Studio from the historical accepted alpha.
