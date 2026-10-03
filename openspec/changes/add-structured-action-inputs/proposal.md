# Structured action inputs

## Why
Authors currently implement `Input`/`Field` or encode several action parameters in maps. Command payloads need a strict object codec that is easy to use, without a second domain entity.

## What Changes
- Add an opt-in `Input` derive exported by the `rom` facade.
- Share named-field parsing, wire names and codec generation with Resource derive.
- Verify typed actions, rejection diagnostics and renamed Cargo dependencies.

## Impact
- Additive authoring API; no new dependencies or Resource registry entries.
- Existing scalar inputs and standalone Presence envelopes remain unchanged.
- No action discovery schema, nested input structs, generics or independent Serde rules in this change.
