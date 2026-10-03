---
name: rom-resource
description: Add or change a compiled Rust ROM Resource and its registration, including a native custom field. Use when an application needs a Resource declaration; adapter implementation and runtime schema authoring are separate tasks.
license: MIT
compatibility: Experimental ROM 0.1.0-alpha.1 fixture, baseline 64240cf, Rust 1.99; linked source checkout required.
metadata:
  rom-contract: "64240cf"
  status: "research-fixture-not-installed"
---

Implement the requested domain declaration through ROM's public interfaces.
Inspect the application's existing definitions and `Cargo.lock` before adapting
this baseline fixture. A mismatched ROM contract needs source verification;
this draft does not claim compatibility with arbitrary alpha revisions.

Use [the author template](../src/lib.rs) for a native `Field`, two derived
Resources, a custom `Action` and one registration function. Reuse the host's
Runtime, storage and generic transport. Supply the requested field names and
actual application policy; the fixture's `owner` actor is synthetic.

For optional values, read the checked-out `docs/presence-and-patch.md`:
`Option<T>` requires a value/null; `Presence<T>` allows omission. Keep codecs and
typed selectors on one definition. Choose `Command::patch` for partial changes,
with an explicit revision and idempotency key.

Run [the author-task fixtures](../tests/author_tasks.rs) from their standalone
manifest. Done means the intended domain behavior passes with invalid-input,
denied-actor and stale-revision cases, and adding the kind required no per-kind
repository, HTTP route or worker. The fixture verifies APIs; it does not measure
agent skill selection or human author usability.
