# Compose durable browser mutation recovery

## Why

The public client retains immutable submitted bytes inside one client instance.
Its private prepared-object map cannot restore that command after reload or a session generation change.
Applications repeat pending-command, autosave, draft and navigation coordination.

## What changes

Add a provider-neutral browser recovery helper over the existing prepare/submit contract.
Require host-selected pending-intent storage and a durable principal binding. Preserve exact command bytes, expected revision and idempotency identity.
Keep the editable draft separate from the accepted command. Restore commands into the current client generation without changing identity.
Test actual HTTP receipt replay on SQLite and redb. Test an extracted Svelte consumer through public entries.

## Impact

No new receipt repository, receipt endpoint, automatic localStorage or process-global pending store is proposed.
The host supplies storage, trusted identity mapping and draft policies. Navigation and server cancellation remain outside the helper.
The current release verifier is running; this increment changes design and plan documents only.
