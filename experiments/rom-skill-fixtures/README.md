# ROM skill author fixtures

Research-only template and one draft skill, not an installed skill library or an
agent/human usability result. The draft expects this linked source checkout.

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-skills-probe-target cargo test --locked --manifest-path experiments/rom-skill-fixtures/Cargo.toml
```

`src/lib.rs` supplies two Resources, one native custom Field, an Action and one
registration function. Three actual SQLite runtime fixtures check canonical
querying, all queries, live observation, partial presence, explicit revision and
idempotent replay, plus invalid data/denial/conflict with no extra state/events.
There is no per-kind storage repository, transport handler or worker. The skill
points to this executable template so code—not copied prose—anchors its examples.

Agent routing, task completion time, token cost and human readability have not
been evaluated. The draft's sibling source pointers are fixture-only; release
packaging must bundle assets/references and validate all pointers after extraction.
