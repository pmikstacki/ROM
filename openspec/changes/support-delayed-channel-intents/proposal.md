# Durable delayed channel intents

## Why

A confirmed external refusal can require a later attempt. Current channel intents cannot freeze a first eligible time.
The existing Work ledger already stores due times. Applications should not need a second scheduler for this contract.

## What changes

Add `Channel::intent_at(payload, not_before_unix_seconds)`. Preserve immediate `Channel::intent(payload)` behavior.
Freeze optional `not_before` fields in `Intent` and `PendingWork`. Preserve the floor through claims, retries, operator recovery, and restore.
Advance native storage format 8 to 9 and archive format 6 to 7. Explicit upgrades preserve populated source data.
Old readers must reject the new format before mutation. Binary rollback and data restore remain separate operations.

## Impact

The generic channel and Work contracts change. No AI-specific policy, provider retry logic, dependency, or scheduler is added.
Public struct literals require the new optional field. Supported method paths remain stable.
SQLite and redb share the scheduling and upgrade acceptance cases. Existing archives and evidence remain unchanged.
