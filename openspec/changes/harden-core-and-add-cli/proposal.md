## Why

The first integrated MVP works, but authors cannot construct a typed unfiltered
query, extreme concurrency limits can panic, and users have no generic command
line client. The owner explicitly narrowed the continued goal to complete
research of deferred capabilities, API ergonomics, core resilience and CLI.
Studio is excluded for now. Research is not blanket implementation approval for
every deferred profile.

## What Changes

- Complete a primary-source research/decision map of every deferred profile.
- Add typed all-queries and validate runtime/HTTP semaphore limits without panic.
- Add bounded, explicitly authorized metadata discovery to core and generic HTTP.
- Add a real generic CLI with human/JSON output, bounded inputs/streams, and
  truthful mutation uncertainty/recovery behavior.
- Test and review the combined consumer journey against SQLite and redb.

## Impact

The Resource premise, storage format, dynamic action authorization and existing
transport separation remain. Metadata disclosure requires a new opt-in policy;
existing definitions expose nothing by default. CLI is optional host tooling,
not a driver or terminal dependency in core. Studio and new distributed/storage/
identity product profiles remain deferred pending their individually scoped work.
