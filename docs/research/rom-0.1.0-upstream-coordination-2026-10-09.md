# ROM 0.1.0: UI and provider coordination

Date: 2026-10-09. Both referenced owner tasks were read through `read_thread` before this record was written.
Their reported checks are not independently executed ROM release acceptance.

## Ownership and inspected state

| Work | Owner task | Current source boundary | Root responsibility |
| --- | --- | --- | --- |
| Reusable controls and gallery | [ROM Studio controls review](thread://01a11c4b-e18e-7ef2-be12-425e2f84ef69?hostId=remote-ssh-codex-managed%3A7455bafd-bcdf-4924-b743-87f13bd5b771) | `/root/.codex/worktrees/rom-ui-gallery/rom-ui`; inspected package version `0.1.0-alpha.5` | Qualify packaged public exports and Studio adoption. Do not copy controls into ROM. |
| External providers and map capabilities | [Generic database providers](thread://01a11b9f-2d2c-7051-88db-6d956ae578e3?hostId=remote-ssh-codex-managed%3A7455bafd-bcdf-4924-b743-87f13bd5b771) | `/root/ROM-extras`; current provider and map changes remain uncommitted | Preserve generic core contracts. Qualify each advertised adapter separately. |
| Core and complete release | This ROM task | `/root/ROM` | Identity, durable work, recovery, public composition, matching artifacts, and final release acceptance. |

Studio's inspected `package.json` still pins `rom-ui` to the `0.1.0-alpha.3` release archive.
The UI worktree's `0.1.0-alpha.5` manifest does not establish publication or adoption by Studio.
The current root verifier therefore does not qualify alpha.5 as the installed Studio dependency.

## Reported upstream progress

The UI task reports passing alpha.5 checks: 78 unit tests, 116 consumer tests, 38 snippet checks, and 72 gallery tests.
It reports deployment and ongoing public-site verification. Map/provider integration remains incomplete.
These counts describe that task's scope. They do not prove matching ROM release packages or backend authorization.

The provider task reports a failed full verifier caused by fixture readiness failures in OpenSearch recovery cases.
Its current investigation separates retained closed indexes from active indexes in the readiness query.
It reports changing the fixture query to use `expand_wildcards=open`, without widening adapter deadlines.
The provider repository is still dirty. A corrected source query is not proof that the full verifier now passes.

## Integration rules for 0.1.0

1. Keep controls in `rom-ui` and external drivers in ROM-extras. Root owns their integration into the matching release profile.
2. Select an exact UI archive only after checking its source identity, public exports, notices, and declared compatibility.
3. Test Studio against that installed archive. Include generic fields, filters, details, recovery, and shared conversational controls.
4. Do not use a workspace source alias as packaged-consumer evidence.
5. Require actual conformance evidence before advertising another database or provider as supported.
6. Keep map capabilities separate from persistence and projection write semantics.
7. Do not delay the core release for unrequested optional map adapters. Do not advertise their unfinished implementation.
8. Preserve each task's source, tests, failed fixtures, and evidence. Assign explicit ownership before any cross-repository edit.

Alpha.3 remains the inspected Studio dependency until a verified upgrade is performed.
Alpha.5 adoption and selected ROM-extras compatibility must appear explicitly in the final release requirement matrix.
The separate tasks remain active. Their activity does not establish completion of ROM 0.1.0.
