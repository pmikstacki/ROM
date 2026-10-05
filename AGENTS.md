# ROM agent instructions

Before you change code, read [the quality gates](docs/quality.md).
Apply its module boundaries and DRY rules to implementation and review tasks.
Keep public API paths stable during structural refactors.

Before you change documentation, read [the writing rules](docs/writing.md).

For Studio controls, presentation or plugin settings, read [the project Studio skill](skills/project/rom-studio/SKILL.md).
For other authoring and operator tasks, use [the AI workflow map](docs/ai-development.md).

For parallel work, assign separate file ownership before edits start.
Preserve historical prototypes, worktrees and test evidence unless the task explicitly includes them.
After a refactor, run the affected checks and the full local verifier before integration.
