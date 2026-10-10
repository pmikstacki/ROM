# AI-assisted ROM development

The same Resource contract applies to work written by humans and agents.
Use focused skills and source maps to find the relevant boundary. Do not copy a historical prototype into maintained code without conformance checks.

These workflows target the `0.1.0` source candidate. Complete release acceptance remains open.
Read [release support](release-support.md) for the last accepted artifact and current candidate limits.
Read the selected checkout's manifests and extension profile before choosing dependency or format versions.

## Start from the task

| Task | Entry point |
| --- | --- |
| Build an application, compose sessions, or recover browser saves | [Application skill](../skills/project/rom-application/SKILL.md) |
| Integrate AI providers, routing, tools, or durable flows | [AI integration skill](../skills/project/rom-ai-integration/SKILL.md) |
| Declare Resources and actions | [Resource author skill](../skills/rom/rom-resource/SKILL.md) |
| Add a native extension | [Native extension skill](../skills/rom/rom-native-extension/SKILL.md) |
| Add a database, delivery provider, search index, or vector projection | [Provider skill](../skills/project/rom-provider/SKILL.md) |
| Inspect or recover durable work | [Operator skill](../skills/rom/rom-operator/SKILL.md) |
| Prepare a release artifact | [Release skill](../skills/rom/rom-release/SKILL.md) |
| Develop Studio, controls, and plugin settings | [Project Studio skill](../skills/project/rom-studio/SKILL.md) |

The four release-bundle skills have executable assets and admission checks.
The four project skills are source guidance. They are not packaged executable workflows or evidence of a held-out author evaluation.
See the [bundle procedure](../skills/rom/README.md) for source identity and execution requirements.

## Select the implementation owner

Use `rom-ui` for shared controls and generic UI behavior. Use Studio for ROM semantics and public recovery composition.
Keep domain policy and application orchestration in the host. Select ROM-extras for its external provider contracts.
Read the affected package's exports, declared package manager, and lockfile before choosing imports or verification commands.
Verify exact installed archives independently from workspace source aliases.

## Map the boundary before editing

Read [quality gates](quality.md) before implementation. Read [writing rules](writing.md) before documentation changes.
For a Studio task, trace Rust declaration → discovery → client validation → renderer → invocation → returned authorized value.
For a persistence task, trace the generic commit contract and shared adapter conformance tests.
For an operator task, trace expected work version, idempotency identity, and unknown-outcome recovery before changing the UI.

Keep `lib.rs` and `mod.rs` as facades. Put behavior in named modules. Preserve public paths through exports during refactors.
Assign separate file ownership before parallel edits. Coordinate shared browser servers and build targets before running checks.
Keep sources, historical prototypes, worktrees, and evidence unless an explicit task authorizes removal.

Before combined verification, capture the source revision, dirty diff, source hashes, lockfiles, and tool identities.
Assign one owner to each browser server, provider window, and build target.
Keep the captured sources unchanged while their checks run. Use an isolated snapshot when other work must continue.
If an executed dependency changes, retain the interrupted result and repeat affected checks against a new snapshot.
Release preparation additionally requires the clean checkout and exclusive output specified by the release procedure.

## Produce reviewable evidence

Record the source revision and dirty changes with each verification result.
Separate source observations, proposals, fixture tests, real backend journeys, packaged-consumer acceptance, and human feedback.
An imagegen mock is not a browser screenshot. A screenshot is not proof of authorization or durable commit behavior.
An isolated custom renderer test does not prove that the released entry point bundles that renderer.

Use [the 0.1.0 progress report](research/rom-0.1.0-full-feedback-coordination-2026-10-08.md) for recorded candidate checks and open gates.
Use [0.0.3 acceptance](research/rom-0.0.3-release-completion.md) for the last accepted release.
Use [the 0.0.3 research](research/rom-0.0.3-studio-release-research.md) and [0.0.2 acceptance](research/rom-0.0.2-release-completion.md) as historical context.
Research recommendations do not expand the advertised release support matrix by themselves.
