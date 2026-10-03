# Framework source release completion audit

Date: 2026-10-03.
Status: source and packaged application checks passed. Complete release execution and artifact generation are pending.
This audit does not yet close release tasks 4.5 or 4.6.

The release goal concerns the maintained native Rust framework and its local source distribution.
It does not require every capability from the original research program.
The [support boundary](../release-support.md) identifies the selected profile and remaining exclusions.

## Requirement mapping

The requirement names below match the [release specification](../../openspec/changes/prepare-framework-release/specs/framework-readiness/spec.md).
Each evidence report preserves its original source identity, tests, review, and limitations.
Final combined acceptance must run the maintained tests again on the artifact source.

| Requirement | Implementation and scenario evidence |
| --- | --- |
| Application authors use one public Resource contract | Public reference application, typed actions, policies, queries, relations, and reactions. [Reference progress](framework-release-progress.md) and [transition checks](resource-transition-validation-results.md) cover pending compensation, process exit, unrelated reservations, and exact replay. |
| Data lifecycle preserves obligations | [Restrict references](restrict-reference-results.md), [Resource migration](resource-migration-results.md), [retention](retention-results.md), and [reference upgrade](reference-upgrade-results.md). Cases include create/delete arbitration, omitted source kinds, historical codecs, incomplete graphs, unfinished work, version fences, sealed epochs, and backup/restore. |
| Optimization preserves observable semantics | [Query foundation](query-integration-foundation-results.md), [read contract](query-read-contract-results.md), and [maintained measurements](maintained-query-cost-results.md). Cases cover authorization order, callback failures, coherent reads, response bindings, transaction rollback/replay, integrity rejection, rebuild, and preserved retry epochs. |
| Query strategy probes preserve the Resource contract | [Maintained query results](maintained-query-cost-results.md). Bounded probes distinguish saturation from exact cardinality and cannot truncate results or bypass semantic admission. |
| Runtime ownership excludes competing supervisors | [Ownership results](runtime-ownership-results.md). Cases cover wrappers, caller cancellation, stopped handles, and failed construction. |
| Native ownership covers open and offline maintenance | [Ownership results](runtime-ownership-results.md). Cases cover process exit, aliases, conversion callbacks, publication guards, and interrupted staging cleanup. |
| Release readiness includes operational recovery | [Operator recovery](operator-recovery-results.md), [provider deployment](provider-deployment-results.md), and native conformance. Final packaged reference acceptance remains pending. |
| Operator work views use current authority and bounded projections | [Operator recovery](operator-recovery-results.md). Tests cover authority changes, history generations, projection bounds, and hidden payloads. |
| Operator controls have atomic version and identity arbitration | [Operator recovery](operator-recovery-results.md). Tests cover changed delivery state, lost responses, replay, and bounded retry allowances. |
| Delivery reconciliation preserves declared guarantees | [Operator recovery](operator-recovery-results.md). Hold and retry profiles distinguish uncertain delivery, inconclusive evidence, and compatible existing work. |
| Operator CLI preserves generic Resource semantics | [Operator recovery](operator-recovery-results.md). Tests reject mismatched valid JSON and preserve control evidence across maintenance. Compensation remains an ordinary explicit Resource action. |
| A release identity profile uses real provider evidence | [Provider deployment](provider-deployment-results.md). Actual pinned service tokens reach ordinary Resources on both native stores. |
| Authentication acquisition has bounded owned execution | [Provider deployment](provider-deployment-results.md). Tests cover cancellation, capacity, provider timeout, drain, and recovery. |
| Provider configuration cannot grant arbitrary acquisition authority | [Provider deployment](provider-deployment-results.md). Approved endpoints and secret references remain host-owned; current revisions invalidate stale evidence. |
| Local provisioning is explicit and restartable | [Provider deployment](provider-deployment-results.md). Explicit commands replay safely and refuse conflicting or unauthorized configuration. Serving does not provision automatically. |
| Native extension releases expose versioned conformance | [Native conformance](native-conformance-results.md). Public fixtures check codecs, atomic bundles, exact replay, exclusive reopen, profile admission, and optional blob behavior. |
| Author skills have portable executable evidence | [Native conformance](native-conformance-results.md). Portable assets, complete inventory admission, baseline attempts, independent review, and separate author evaluation. |

The [scenario map](framework-release-scenarios.md) lists all 55 named scenarios and their maintained evidence.
It preserves source-review qualifications separately from executed cases.

## Dependency review

The current review used Cargo.lock SHA256 `fd8c304a3bde42c05c9503f0c6de201bd23b2d788dafba678c508ebae91b9a83`.
`cargo-audit 0.22.2` reported zero known vulnerabilities and no warnings for 299 locked dependencies.
The RustSec database revision was `ef6173cbc5c50ec8166f9a5b28f07834144373ee`.
The retained [audit output](evidence/framework-release-2026-10-03/cargo-audit.json) provides the authoritative database identity.
The [all-feature dependency inventory](evidence/framework-release-2026-10-03/dependency-notices.json) contains 282 external packages and their declared licenses.
The maintained inventory and third-party notices were also regenerated offline from this lockfile.
Both notice collector tests passed; no missing-notice placeholders remain in the generated collection.
This inventory records package metadata; it does not certify legal compatibility.
The older auditor failed on current advisory metadata. That failed invocation is not a successful audit.

## Evidence distinctions

The provider acceptance is a separate service-token journey.
The default reference upgrade and operator journeys use their explicit synthetic host identity profile.
Do not describe these separate checks as a single provider-to-migration experiment.
The packaged application gate must identify which journeys it actually runs.

Process-exit tests preserve pending work across abrupt application termination.
They do not certify storage behavior during machine power loss.
MinIO acceptance covers the recorded local provider profile, not every S3-compatible service.
Query measurements identify workloads, seeds, allocation scope, and overhead; they are not universal performance guarantees.

The author baselines and skills use agent execution.
They establish executable public workflows, not human usability or a measured productivity gain.
The current Field codec profile remains Resource-owned; independent Field registration is not implemented.

## Final acceptance preparation

The coordinator ran the full local verifier and packaged application verifier on the reviewed release worktree.
Both commands returned zero. The [source log](evidence/framework-release-2026-10-03/check.log) and [package log](evidence/framework-release-2026-10-03/packages.log) preserve execution.
The [package record](evidence/framework-release-2026-10-03/packaged-application.json) binds 13 archives, 27 consumer inputs, and 77 demo inputs.
Both applications resolved ROM dependencies exclusively from the extracted libraries.

The first package run found a missing copied `settings.toml` input.
The next run passed application tests but exceeded the metadata capture buffer.
The final run included the input and used a bounded 32-MiB capture buffer; the metadata path audit then passed.
These earlier attempts are partial failures, not successful package gates.

The [signal report](evidence/framework-release-2026-10-03/signal-results.md) separates readiness tests from component drain evidence.
The [signal review](evidence/framework-release-2026-10-03/signal-review.md) and [package review](evidence/framework-release-2026-10-03/package-review.md) accepted the implementation.

Artifact review found two metadata discrepancies: a changed declared profile and an incomplete declared source inventory could pass verification.
The verifier now compares both complete declarations with the extracted source.
After this repair, the coordinator ran all 24 artifact and package helper tests; all passed.
The [artifact report](evidence/framework-release-2026-10-03/artifact-results.md) and [test log](evidence/framework-release-2026-10-03/artifact-tests.log) preserve the scope.
The earlier full source log predates this JavaScript-only repair. Complete clean-source release execution remains pending.

## Remaining final gates

- Run source, default/provider demo, skills, and package gates on the reviewed final inputs.
- Produce and verify the exclusive source archive, native skill bundle, checksums, and artifact manifest.
- Review this mapping against the final inputs and record any remaining requirement gap explicitly.
