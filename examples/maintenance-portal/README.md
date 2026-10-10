# Maintenance portal consumer

This independent application uses public ROM APIs. Its first increment defines Equipment, Inspection, WorkOrder, and PortalSettings Resources.
One registration helper applies the same ownership checks to all four declarations.
The WorkOrder completion action records a domain event through the normal mutation path.

## Current evidence

The SQLite and redb tests create all four Resource kinds.
They reject another owner's action and exclude that owner's rows from queries.
They replay the same completion command with its original revision and idempotency key.
The final revision remains two. The WorkOrder journal contains exactly two events: creation and completion.
Two field tests verify typed equipment references, invalid date rejection, and canonical UTC values with descriptor codec identity.

Run the source tests:

```sh
cargo test --locked --manifest-path examples/maintenance-portal/Cargo.toml
cargo clippy --locked --manifest-path examples/maintenance-portal/Cargo.toml --all-targets -- -D warnings
```

The test host constructs trusted human actors. This construction does not verify browser credentials.
Two loopback HTTP tests now exercise the same four declarations on both databases.
They check access denial, retained receipt replay, invalid dates, ownership protection, and persisted Settings changes.
They release every storage handle before reopening the database and replaying the original completion.
The HTTP fixture uses fixed test credentials. It does not establish production authentication.
An installed Svelte portal and runnable deployment remain incomplete.
Equipment links use `ResourceRef<Equipment>`. Inspection timestamps use the validated `rom_fields::DateTime` codec.
Settings include a complete layout, encoded as JSON document text through the ordinary field contract.
The application validates widget identities, duplicate IDs, dimensions, bounds, and the selected column count before commit.

## Private browser fixture

Build the bounded loopback fixture:

```sh
cargo build --locked --features http-fixture --manifest-path examples/maintenance-portal/Cargo.toml --bin rom-maintenance-http-fixture
```

The fixture requires `--fixture-only`, an adapter name, and absolute database, readiness, and stop paths in one private directory.
It binds an ephemeral loopback port and publishes its exact address in the readiness file.
It seeds the four application Resources. It drains when the stop file appears or after 300 seconds.
Its fixed credentials are test inputs. Do not deploy this binary as a production authentication service.

## Remaining admission

Complete Task 4 in `docs/superpowers/plans/2026-10-07-ui-composition-admission.md`.
Add the public Svelte workspace, streaming and interruption journeys, and production identity lifecycle integration.
Verify navigation, unknown outcomes, private/public views, selected history, and export identity on both adapters.
Repeat the complete path from an admitted source archive. Obtain the separate human authoring and accessibility assessment.
These declaration tests do not establish production readiness or release acceptance.
