# Provider deployment results

Date: 2026-10-03.
Status: stage 4.3 passed combined verification and independent source review. The supported profile covers both native stores.

This stage adds an opt-in reference host for one configured OAuth service profile.
It keeps authentication outside the Resource core and preserves the existing synchronous HTTP API.
The [deployment guide](../provider-deployment.md) describes commands, authority boundaries, and operational limits.

## Implemented composition

| Layer | Responsibility |
| --- | --- |
| `rom-http` | Resolve owned headers asynchronously, then use the existing admission, decoder, and Resource routes |
| Host configuration | Approve exact provider settings and a finite map of versioned secret references |
| `HostAuth` | Bound admitted jobs, preserve ownership after caller cancellation, and drain accepted work |
| Existing verifier | Validate the original provider response against the configured service profile |
| Existing identity Resources | Bind the verified identity against current provider, link, and User revisions |
| Reference application | Provision explicitly, grant declared Task operations, and keep maintenance local |
| Generic CLI | Invoke the same Resource operations over TCP with a private authentication file |

Implementation belongs in named modules. The crate and profile facades contain declarations and exports.
The synchronous and asynchronous HTTP constructors share one decoder and one limits validator.
Host configuration and maintenance inputs share private-file acquisition and duplicate-key rejection.
Test drivers share provider, subprocess, and file helpers.

## Actual-provider experiment

The maintained fixture pins `oidc-provider` 9.12.2 and its complete npm lockfile.
It has separate token and introspection clients.
The provider issues opaque tokens through its client-credentials endpoint and supplies its original introspection response.
The configured claim hook declares the service subject and principal kind.
ROM does not rewrite that response or relax its verifier.

The actual `rom` executable ran the same journey against SQLite and redb.
The trial proved these assertions:

- Explicit provisioning can resume with identical inputs; changed accepted input fails.
- The linked service can discover the Task descriptor, create a Task, and invoke its `complete` action.
- Query results contain the expected Task and revision.
- Journal results contain exactly the creation and completion events, in order.
- Exact receipt replay returns the original accepted result without another Task event.
- A ROM restart preserves the Task and receipt while the provider remains available.
- Disabling a link, provider, or User denies access. A disabled link also prevents receipt disclosure.
- A mismatched audience denies a real token, even when host settings agree with the changed Resource configuration.
- Ordinary service access grants no operator inspection, retry, or reconciliation.
- A provider restart replaces its introspection credential and removes its opaque tokens.
- A fresh token fails with the obsolete introspection credential and succeeds after the explicit reference update.
- The pre-restart token fails before its expiry margin; this distinguishes provider state loss from natural token expiry.
- An unapproved credential reference denies access. Restoring an approved reference recovers authorized receipt replay.
- Natural token expiry denies access. A newly issued token succeeds afterward.
- Captured child output and closed native database bytes contain none of the generated credential markers.

The expiry check uses the provider's 60-second token lifetime and actual wall time.
It does not replace the clock or fabricate an inactive introspection response.
The finite harness owns its child processes and private temporary inputs.
Raw database scans supplement structured tests; they do not replace archive and receipt inspection.

The earlier [selection analysis](real-provider-profile-selection.md) and [verifier probe](real-provider-profile-probe.md) remain separate evidence.
That probe rejected provider 9.5.1 after an advisory check and verified the replacement 9.12.2 profile.
The maintained fixture changes the probe's package name, so its root lockfile hash differs.

## Focused resilience and authority tests

| Scope | Evidence and limitation |
| --- | --- |
| HTTP seam | 34 tests passed before integration. TCP cases cover async progress, body admission, denial before body consumption, synchronous compatibility, and shutdown. |
| Host lifecycle | 14 driver tests passed, plus two explicitly executed subprocess fixtures. Synthetic response servers permit deterministic cancellation, deadline, overload, and revision races. |
| Secret acquisition | Regular private UTF-8 files succeed. Invalid sizes, encoding, permissions, paths, and references deny before provider contact. |
| Provider outage | A paused synthetic endpoint reaches the verifier transport timeout before the caller deadline. The same host then recovers and authenticates successfully. |
| Drain | Caller cancellation retains capacity. A canceled drain waiter does not discard tracking. A later waiter still waits for accepted work. |
| Current binding | Provider reference changes during verification deny stale binding. The configuration mutation can commit during provider I/O. |
| Reference profile | 12 focused tests passed after review fixes. Immediate SIGINT follows the graceful drain path. |
| Provisioning | Actual native stores preserve each of the three separately committed setup steps after a lost acknowledgement and reopen. |
| Local authority | Serving excludes provisioning. Maintenance checks configured keys, permitted operations, explicit revision, and idempotency identity. |
| Identity and archives | Both-store tests reject forged or obsolete actors and inspect native backup Snapshots for credential markers in rows, receipts, events, and persisted state. |
| Native ownership | Maintenance cannot acquire the database while the serving host owns it. |
| Fixture commands | Three Node tests cover real issuance/introspection, restart, private files, command output, and finite subprocess ownership. |
| Public API | An independent Cargo workspace compiled the sync/async HTTP and host authentication surface. |

Synthetic host tests establish mechanism behavior. They are not proof of arbitrary provider compatibility.
The earlier host suite used SQLite; application tests and the actual CLI journey cover both native stores.
Native restart tests do not establish machine power-loss guarantees.

## Findings and corrections

1. The first actual CLI journey found missing Task discovery permission.
   The default metadata denial worked; the reference profile omitted its explicit grant.
   A focused failing test reproduced the missing descriptor. The fix exposes Task metadata only.

2. The first operator assertion expected capability discovery itself to fail.
   The existing contract returns denied capability flags to an authenticated actor.
   The harness now asserts all flags are false and separately requires work inspection to fail.
   This corrected the test oracle; it did not change operator policy.

3. Preliminary review found incomplete success assertions in the CLI harness.
   The harness now compares Task identities, values, revisions, event counts, event order, and exact resumed receipts.
   It uses a fresh pre-restart token and a validity margin to distinguish restart from expiry.
   Teardown attempts every owned child, even when another child's cleanup fails.

4. The first combined gate found a temporary-directory collision in a test fixture.
   Process ID plus wall-clock nanoseconds did not guarantee uniqueness under concurrent execution.
   The fixture correction preserves parallel tests; this failure did not establish an authentication defect.

5. Review found that the SIGINT handler registered after readiness.
   An immediate interrupt could bypass the intended authentication drain.
   A readiness-to-interrupt subprocess test reproduced nongraceful termination before the fix.
   Both Unix signal receivers now register before the readiness message.

A separate Node probe did not reproduce blocking after removal of `O_NONBLOCK` on this environment.
That result is not a successful regression test.
An explicit synchronous FIFO block did exercise the subprocess deadline and forced cleanup.
The maintained private-file test also runs its FIFO read in an isolated child.

## Verification record

The evidence directory is [provider-deployment-2026-10-03](evidence/provider-deployment-2026-10-03/).
It retains the first combined failure, actual CLI trial, fixture checks, external consumer log, source hashes, and toolchain versions.
The [final combined log](evidence/provider-deployment-2026-10-03/combined-final.log) records the complete command sequence with exit 0.
All 441 entries in [the source manifest](evidence/provider-deployment-2026-10-03/source-sha256.txt) matched live files after execution.
The earlier 440-file gate remains separate evidence. The final gate includes the composed provider-outage test.

The verification commands are:

```sh
./scripts/check
./demo/verify
./demo/verify-provider
```

The external API consumer also uses its own Cargo workspace and lockfile. Its locked compile check passed after the three commands.
The dated [npm audit](evidence/provider-deployment-2026-10-03/npm-audit.json) reported zero known vulnerabilities for the pinned fixture graph.
This result is not a security certification.
Review records distinguish author-reported checks, independent source review, and coordinator execution.
Stage 4.3 passed its final gate. The independent review records distinguish source assessment from coordinator-executed checks.
Stages 4.4–4.6 still require extension conformance, executable author skills, package acceptance, artifacts, and the completion audit.

## Support limits

The reference profile requires trusted Linux directories, private versioned files, and an explicitly approved service identity.
It provides no zeroization guarantee, automatic secret fallback, public enrollment, human login, or arbitrary provider support.
The fixture is an ephemeral development provider. A ROM restart and a provider restart have different consequences.
This evidence does not certify a hosted service, persistent provider cluster, or production TLS deployment.

## Requirement audit

The following table covers the four deployment requirements added to the release specification.
Other release requirements retain their separate evidence and task states.

| Requirement | Implementation and evidence |
| --- | --- |
| A release identity profile uses real provider evidence | The original provider response passes the existing verifier. The actual CLI journey covers both native stores, expiry, audience binding, access revocation, and receipt replay. |
| Authentication acquisition has bounded owned execution | One HTTP decoder supports both resolvers. Host tests cover admission, caller cancellation, deadlines, repeatable drain, and stale activation. A composed provider-outage test checks static denial, bounded drain, no automatic retry, and recovered capacity. |
| Provider configuration cannot grant arbitrary acquisition authority | Approved settings precede acquisition. Private-file tests cover reference, type, size, encoding, and permissions. Revision-race tests reject stale binding. Structured archive tests and native byte scans cover credential markers. |
| Local provisioning is explicit and restartable | Three ordinary Resource commits use stable identities. Both-store tests interrupt each step, reopen, and resume. Serving excludes provisioning; maintenance validates exact configured keys, revisions, and operations. |

## Documentation review

The guide and report use the [ROM writing rules](../writing.md) and the globally installed ASD-STE100 skill.
Resource, action, commit, receipt, Actor, provider, and credential reference retain their technical meanings.
Commands, paths, versions, limits, and test counts retain their source values.
Raw logs and independent review records are retained without prose edits.

Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard.
