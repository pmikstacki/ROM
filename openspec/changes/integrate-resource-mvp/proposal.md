# Integrated Resource MVP

## Why

ROM's experiments validate individual mechanisms but have different interfaces.
An application cannot yet use one reusable library to obtain the complete
Resource flow. The owner explicitly requested that research, recommendations
and prototypes finish in one working MVP, rather than another collection of
reports.

## What changes

Promote the typed integrated probe into a maintained Cargo workspace. Then correct
its documented correctness and lifecycle gaps. Deliver a transport-free `rom`
facade with derive support, generic persistence and policy contracts, bounded
execution and observation, durable reaction work, and separate adapters. Test
the same behavior through a packaged Rust consumer and an actual HTTP binding.
Retain the original probes as evidence, not production dependencies.

The first verified deployment profile is one runtime owning writes. SQLite and
redb are reference implementations of one contract, not concepts in the core.
Advanced queries, independently writing processes, WASM, and a production Studio
remain outside this bounded MVP; their research and explicit recommendations
still belong to the completion inventory.

## Impact and compatibility

There is no released ROM database or public Rust API to migrate. Experimental
probe data is not silently imported. New stored formats carry a version and
unsupported versions fail before writes. The MVP is pre-1.0 and does not promise
stable Rust ABI or arbitrary provider interoperability. Reversion means that the new application stops and its versioned database remains. Reversion does not erase
resources or pending obligations. No existing research artifacts are deleted.

## Evidence

See `docs/research/mvp-decision-register.md` and the linked trial reports. Passing
standalone experiments does not satisfy this change's integrated acceptance.
