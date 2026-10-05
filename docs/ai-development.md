# AI-assisted ROM development

The same Resource contract applies to work written by humans and agents.
Use focused skills and source maps to find the relevant boundary. Do not copy a historical prototype into maintained code without conformance checks.

## Start from the task

| Task | Entry point |
| --- | --- |
| Declare Resources and actions | [Resource author skill](../skills/rom/rom-resource/SKILL.md) |
| Add a native extension | [Native extension skill](../skills/rom/rom-native-extension/SKILL.md) |
| Inspect or recover durable work | [Operator skill](../skills/rom/rom-operator/SKILL.md) |
| Prepare a release artifact | [Release skill](../skills/rom/rom-release/SKILL.md) |
| Develop Studio, controls, and plugin settings | [Project Studio skill](../skills/project/rom-studio/SKILL.md) |

The four release-bundle skills have executable assets and admission checks.
The project Studio skill is source guidance. It is not a fifth packaged workflow or evidence of a held-out author evaluation.
See the [bundle procedure](../skills/rom/README.md) for source identity and execution requirements.

## Map the boundary before editing

Read [quality gates](quality.md) before implementation. Read [writing rules](writing.md) before documentation changes.
For a Studio task, trace Rust declaration → discovery → client validation → renderer → invocation → returned authorized value.
For a persistence task, trace the generic commit contract and shared adapter conformance tests.
For an operator task, trace expected work version, idempotency identity, and unknown-outcome recovery before changing the UI.

Keep `lib.rs` and `mod.rs` as facades. Put behavior in named modules. Preserve public paths through exports during refactors.
Assign separate file ownership before parallel edits. Coordinate shared browser servers and build targets before running checks.
Keep sources, historical prototypes, worktrees, and evidence unless an explicit task authorizes removal.

## Produce reviewable evidence

Record the source revision and dirty changes with each verification result.
Separate source observations, proposals, fixture tests, real backend journeys, packaged-consumer acceptance, and human feedback.
An imagegen mock is not a browser screenshot. A screenshot is not proof of authorization or durable commit behavior.
An isolated custom renderer test does not prove that the released entry point bundles that renderer.

Use [the 0.0.3 research](research/rom-0.0.3-studio-release-research.md) for proposed UI and descriptor work.
Use [0.0.2 acceptance](research/rom-0.0.2-release-completion.md) for the released baseline.
Research recommendations do not expand the advertised release support matrix by themselves.
