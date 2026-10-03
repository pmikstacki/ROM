# Resource-wide transition validation

## Why
Demo actions enforce reservation capacity and terminal payment outcomes, but generic patch and replace currently bypass those business invariants. A Resource invariant must hold for every mutation path without adding per-kind controllers.

## What Changes
Add opt-in typed `Definition::validate_transition` with actor and previous/candidate values. Apply it before atomic commit for create, replace, patch, custom actions and delete. Use the same declaration in the reference compensation Resources.

## Impact
Existing definitions have no additional invariant. No storage format, wire schema or dependency changes. Replayed receipts retain existing authority checks and do not rerun historical transition validation. New demo declarations reject invalid direct writes; removing validation would reintroduce the bypass. Existing invalid stored records require an explicit repair rather than automatic migration.
