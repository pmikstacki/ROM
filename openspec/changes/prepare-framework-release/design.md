# Framework release design

## Accepted direction

One Resource definition drives application behavior. Only core actions commit
mutations, receipts and events. Persistence and transports remain adapters;
Tokio/Rayon execute supervised work. Prior committed chain steps remain committed;
compensation is an explicit authorized domain action. Default relationship deletion
is restrict. Queries use moving views. Studio remains deferred in favor of CLI.

## Delivery order and boundaries

1. Extend the existing inventory/checkout and task demo into a reference journey.
   Application modules consume public ROM APIs. Exercise pending work across actual
   process termination, replay and live reads; use discoveries from authoring to
   improve API. This does not yet claim referential integrity or schema upgrades.
2. Add transactional relationship enforcement and versioned offline migration,
   then retention with dependency-aware receipts/journal/work identities. Backup
   exists: extend its evidence to upgrades and preserved pending obligations.
3. Promote the selector prototype only with real commit/index maintenance and
   rebuild/recovery. Native DB planners own physical plans. Strategy choice cannot
   change authorization, exact scalar semantics or success/capacity admission.
4. Deliver operator inspection/recovery via CLI, enforced single-writer ownership,
   explicit host identity/bootstrap/secret configuration, extension conformance,
   compatibility documentation, executable skills and verified package artifacts.

The first release profile targets one writer. Offline maintenance is the proposed
initial implementation, not an imposed production downtime policy. Real provider
credentials, retention durations and workload/SLO choices remain host inputs. Local
conformance can progress without pretending those inputs have been supplied.

## First slice

Use existing Stock/Checkout definitions and compensation actions. Prepare two
reservations. Mark one checkout definitively rejected. Stop before its durable
reaction executes. A fresh process rebuilds the same declaration graph, observes
authorized filtered data, drains pending work and replays the original command.
Only the rejected reservation disappears. Unrelated reservations survive, and
replay introduces no new event. The same code runs on SQLite and redb.

Prefer this over a new CRUD sample (would repeat existing evidence) or putting
application orchestration into core (would obscure the generic boundary). The
interactive command uses orderly close/reopen; a subprocess test exits without
shutdown/destructors. Neither experiment certifies power-loss durability.

## Completion evidence

The goal remains open until every stage below has maintained code, relevant fault
tests, independent review and local release evidence. A runnable first slice is
not completion of the full reference application, migrations or release readiness.
