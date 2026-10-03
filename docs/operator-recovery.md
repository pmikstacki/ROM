# Inspect and recover durable work

Status: release stage 4.2 is complete. Native controls, Runtime inspection, provider verification, HTTP, CLI and reference journeys passed combined verification.
See the [results and support limits](research/operator-recovery-results.md).

Resource remains the application entity. Work records and operator receipts are infrastructure metadata.
An operator control changes the schedule or recorded outcome of existing work. It does not create a Resource mutation.
A compensation is a separate Resource action that the application author defines.

## Host authority

The host installs `rom::operator::OperatorAuthorizer` through `Builder::operator_authorizer`.
The default policy denies inspection, retry and reconciliation. Resource discovery and a service identity do not grant these capabilities.
The policy receives the current actor, requested capability and an optional redacted `WorkScope`.
It can use bounded `AuthorizationRead` to evaluate current policy.

The recorded service actor still executes the work. Operator authority does not replace its Resource or field permissions.
ROM evaluates current authority before it discloses an old operator receipt.
A work handle identifies work; it is not an authorization credential.

`Builder::operator_limits` controls public response and raw snapshot bounds.
The public defaults are 64 records and 64 KiB. Raw snapshots default to 2,048 records and 4 MiB.
The raw record count includes work and operator receipts. The byte bound covers the full snapshot envelope.
A list applies authorization before its visible page limit. It never returns part of a serialized record.

## Inspect work

Use an endpoint with a host-configured operator policy. The default workshop session has no operator grant.
Use the host's credential file for these commands:

```sh
rom --endpoint https://example.org/rom-api --auth-file ./operator-auth work capabilities
rom --endpoint https://example.org/rom-api --auth-file ./operator-auth work list
rom --endpoint https://example.org/rom-api --auth-file ./operator-auth work list --query-file ./work-query.json
rom --endpoint https://example.org/rom-api --auth-file ./operator-auth work show HANDLE
```

A query file can select a definition and category:

```json
{"category":"Notification","definition":"account-notices","limit":8}
```

A work view contains its handle, version, definition, state, attempts, due time and delivery status.
It can also contain authorized Resource keys. It contains no frozen values, action input, service principal key or causal root identity.

List cursors use moving pagination. They bind the storage generation, filter and handle ordering.
Each page evaluates current authority. Restore invalidates an old cursor with `HistoryGap`.
The cursor grants no access to a record that is now hidden or absent.

## Retry an unchanged intent

1. Read the current work view.
2. Copy its handle and full version into a request file.
3. Select an explicit operator key and the current retry epoch.
4. Keep the file unchanged after submission.
5. Submit the request with `work retry --request-file ./retry.json` and the same connection options.

Example request structure:

```json
{
  "handle":"0000000000000000000000000000000000000000000000000000000000000000",
  "expected":{"generation":"COPY-FROM-WORK-VIEW","revision":0},
  "key":"operator-retry-001",
  "retry_epoch":0,
  "operation":"Retry"
}
```

The handle and generation above are placeholders. Use values from the authorized view.
Retry preserves the payload, work identity, causal root, original start time, service identity and consumed budgets.
It cannot reset attempts, extend chain age, steal a lease or change a completed record.
The worker executes scheduled work later under its current service authority.

The native transaction commits the scheduling transition and operator receipt together.
After current authority and retry epoch checks, an exact request replay returns the stored result before the stale-version check.
The same principal, epoch and key with changed input returns `IdentityMismatch`.
Operator requests use the [admission and replay floors](retention.md).
A request below the replay floor returns `IdentityExpired`, even while its receipt remains as audit evidence.

If the response is lost or reports an unknown outcome, retain the original request file.
Repeat that exact request with the same principal. A new key can create a separate control; it cannot resolve the old identity.
The CLI does not retry automatically. A disconnect ends the caller's wait, not accepted Runtime work.

## Reconcile and compensate

`work reconcile --request-file ./reconcile.json` uses the same exact-request contract.
Its operation is `{"Reconcile":{"evidence_ref":null}}`, or a bounded opaque evidence reference.
The client cannot assert that a provider accepted or rejected a delivery.

For frozen Resource actions, ROM resolves the original durable receipt and verifies the request fingerprint and target.
A confirmed commit can complete the work without another Resource mutation or event.
Provider reconciliation requires a host-registered verifier; it is not permission to mark arbitrary work complete.
The optional `evidence_ref` is a lookup identifier, not a credential or provider response.
Control results repeat the submitted operation, including this reference, to verify response correspondence. Do not put secrets in the reference.

## Register a delivery profile

Existing `Builder::channel` registrations use `DeliveryProfile::AtLeastOnce`.
For an explicit profile, use `Channel::delivery_profile` and `Builder::channel_with`.
The typed registration accepts a trusted `verifier` callback and an optional `verification_timeout`.
Without an explicit timeout, verification uses the Runtime delivery timeout. An explicit timeout must be positive and representable by the timer.

| Profile | Contract |
| --- | --- |
| `AtLeastOnce` | Retry eligible failures within the original budgets. An uncertain outcome can cause a duplicate external effect. |
| `ProviderDeduplicated` | Keep the delivery identity stable across retries. The provider must enforce deduplication; ROM cannot supply that external guarantee. |
| `ReconcileBeforeRetry` | Hold uncertain started deliveries until a registered verifier establishes their outcome. Ordinary retry cannot bypass the hold. |

ROM stores the selected profile with the work. A different registration cannot silently reinterpret existing work.
For `ReconcileBeforeRetry`, an unknown result, timeout or panic after delivery starts enters `AwaitingReconciliation`.
Recovery also holds an expired started delivery. A restart does not authorize another send.

The verifier receives `DeliveryReconciliation` with a stable delivery ID and the optional lookup reference.
It returns a trusted `DeliveryVerification` value:

- `Accepted { evidence }` completes work without another send.
- `NotAccepted { evidence }` confirms that the old attempt cannot succeed later. Retry remains subject to the original budgets.
- `Unresolved` leaves work unchanged. A lookup miss from an eventually consistent provider belongs here.

The callback runs outside the core commit gate and native transactions.
Host callbacks must use cooperative async execution and bounded I/O.
On timeout, ROM aborts and joins the task. It retains the execution permit until that task ends.
Tokio cannot preempt a callback that blocks or never yields; the timeout is not a hard limit for arbitrary Rust code.
ROM checks current operator, service and Resource authority again before committing a result.
An existing exact receipt wins before a stale-version check. Provider evidence is retained in the receipt, not the public work response.
A missing verifier, timeout, panic or invalid evidence cannot establish acceptance or authorize another send.

After recovery, the application can submit an explicit compensation with the existing `action` or `invoke` command.
Use the action's expected Resource revision and a separate mutation idempotency key.
ROM does not infer a compensation from a timeout or restore an earlier Resource snapshot automatically.

## Audit and adapters

The default operator ledger admits at most 1,024 receipts and 1 MiB of serialized receipt data.
At capacity, a new control returns `Overloaded`.
Existing receipts can replay after current authority and retry epoch checks.
Receipt capacity does not consume the capacity reserved for workers to finish accepted claims.
Backup, restore and maintenance retain operator receipts in this slice.
Restore changes the storage generation, but preserves the identity of an accepted operator request.

A custom adapter implements atomic `Storage::control_work` and coherent bounded `Storage::work_snapshot`.
It advertises this contract through `Storage::supports_operator`. The default is unsupported.
A public read followed by an unconditional write does not satisfy atomic control.
The native SQLite and redb implementations use the same pure transition logic inside their transactions.

## Reference journey

Run the finite local journey against each database:

```sh
./demo/run operator sqlite
./demo/run operator redb
```

The journey revokes a synthetic worker's authority, observes a stopped notification, and restarts the host with its declared authority.
It schedules unchanged work, replays the operator receipt, and verifies one delivery with no additional Resource event.
It then invokes the existing reservation-release action and verifies idempotent compensation.
This fixture grants operator authority only to its bootstrap actor. It does not change the default demo HTTP policy.

## Open policy choices

The existing delivery default remains `AtLeastOnce`. The new tools do not imply exactly-once delivery.
A change to that default is a separate product decision.
Human attestations that force a provider outcome are outside this slice. A future policy for them needs an explicit design.
