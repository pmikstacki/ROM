# Reference application upgrade acceptance

## Purpose

Complete release stage 2.4 with the actual reference application. Use only public
ROM APIs and native maintenance ports. Do not add a test-specific storage path.
Keep Studio and additional backends outside this stage.

## Schema change

Evolve `Checkout` from version 1 to version 2. Version 1 stores `stock_id: String`.
Version 2 stores `stock_id: ResourceRef<Stock>`. The wire value remains a string,
but the descriptor now declares a target and native restrict enforcement applies.
Preserve the exact version-1 type and codec for migration and receipt replay.
The converter validates the ID and constructs the typed reference. Missing targets
reject migration. Do not weaken checkout transition rules in either runtime profile.

Share application composition and business rules. The old profile replaces only
the checkout definition and its reaction binding. Avoid two copies of the app.
Register the old codec on the new definition. Retain callback names, versions and
service identities for unfinished work. Use an explicit work compatibility validator.

## Executable journey

Add `rom-demo upgrade [sqlite|redb]` with a finite scratch-directory journey:

1. Run the old application profile, attach folder content, and prepare the existing
   reference scenario with a confirmed rejection and pending compensation.
2. Stop cleanly in the command. The process-exit acceptance test instead exits
   immediately after that commit, without finalizers or worker drain.
3. Migrate the offline database into a fresh destination. Preserve the original.
4. Export a checked backup and restore it to another fresh destination. Preserve
   the external blob folder separately; database archives do not include its bytes.
5. Start the current application and recover the existing reference scenario.
6. Verify old receipt replay, current authorization, typed queries, live updates,
   the unrelated reservation and the Task-to-Dashboard reaction.
7. Verify rebuilt restrict references and readable external attachment content.

For restrict evidence, use a separate empty stock referenced by a pending checkout.
Deletion must fail because of the reference, not because stock reservations are
nonempty. After deleting that nonterminal checkout, target deletion must succeed.

Compare source bytes while the source remains offline. On a dirty redb source,
maintenance may recover only its private copy. Check that invalid conversion/work
compatibility and an existing destination fail without activating partial output.
Prove late replay adds no mutation, event or work. One old receipt must exercise
the retained codec, not only a fresh current-version command.

## Boundaries

This is a two-backend application acceptance journey. It does not certify online
cutover, physical power loss, external notification delivery or automatic blob
backup. The application is still synthetic; its local session is not a production
identity provider. Stage 3 and stage 4 remain incomplete.
