# Mutation Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Restore and retry one accepted mutation across reload and session renewal while preserving the latest unaccepted draft.

**Architecture:** A recovery lane composes existing createClient prepare/submit with explicit host storage and durable principal binding. It persists exact wire commands, re-prepares them in the current client generation, and relies on existing authorized receipt replay. Draft and navigation policies remain explicit.

**Tech Stack:** Existing TypeScript/Svelte wire codec, ROM HTTP host, SQLite/redb, Vite/Tailwind and Playwright.

**Spec:** `openspec/changes/compose-durable-mutation-recovery/design.md` and `specs/mutation-recovery/spec.md`.

## Global constraints

The coordinator approved the design and assigned the pure helper, facades and new unit tests.
Actual HTTP, adapter and external browser acceptance remain pending.
Preserve existing receipt semantics and public paths.
Do not add credentials, localStorage defaults, process-global pending storage or new dependencies.
Use exact command identity, expected revision and saved retry epoch. Never generate a new key for retry convenience.
No production consumer, live provider or real credential is used by the acceptance fixtures.

## Planned files and interfaces

Public types and methods are defined exactly in the design document. Task implementations consume that contract without renaming methods.
Create `studio/src/client.ts`, `studio/src/recovery.ts` and `studio/src/lib/recovery/{types,record,storage,snapshot,outcome,controller}.ts`.
Modify root exports and package subpaths only after ownership approval. Add recovery and client entries without browser auth exports.
The client facade exports existing createClient/RemoteError, the exact parseWire/stringifyWire codecs, the recovery helper and their types.
Coordinate export metadata through the root while the R2 fix worker owns the package manifest and lock.
Create `studio/tests/unit/mutation-recovery.test.ts` and `studio/tests/unit/mutation-recovery-record.test.ts`.
Add separate `studio/tests/mutation-recovery/` external fixture and bounded verifier.
Add the separate `tests/recovery-host` crate with `main`, `note`, `storage` and test-authority `control` modules.
Keep existing SQLite-only loopback fixtures unchanged. The coordinator admits workspace and lock metadata.
The fixture host exposes test-only control/inspection outside the product API. It binds loopback and uses disposable disk databases.

## Task 1: Exact record and independent helper red/green

Consumes: existing `RomClient`, `Invocation`, `Operation`, `ProjectedView`, `parseWire` and `stringifyWire`.
Produces: design's `createMutationRecovery(options): MutationRecovery` and public type exports.

- [x] Write the first unit test through real createClient with an injected failing transport and a file-backed test store.
- [x] Make the host store atomic comparison real in the fixture; do not assert against mocked storage callbacks.
- [x] Run the test while the recovery module is absent. Confirm the expected missing-public-helper failure.
- [x] Add bounded strict record codec and minimal lane controller; preserve exact invocation bytes.
- [x] Observe the test pass before adding later behaviors.

The first test must stage `{type:"replace",input:{title:"A",count:9007199254740993n}}` on target `notes/one`.
It begins with expected `7n`, idempotency `save-A`, retryEpoch `0n` and principal `{authority:"fixture",kind:"human",subject:"alice"}`.
Its transport captures the actual invoke body and throws after dispatch. Assert phase unknown and unresolved intent true.
Create a new client/helper against the same file store, restore, and retry with a successful revision `8n` response.
Assert both actual request bodies are byte-identical and exactly retain `save-A`, expected 7 and the large integer token.
Assert the restored draft is not silently replaced by the projected result.
This unit test establishes client composition only; its successful response is not backend commit evidence.

Run:

```sh
cd studio
node --experimental-strip-types --test tests/unit/mutation-recovery.test.ts tests/unit/mutation-recovery-record.test.ts
```

## Task 2: Adversarial record, storage and identity tests

- [x] Test presence/null/false/zero/empty/remove and explicit float token `1.0`, negative zero `-0.0`, maximum revision and epoch tokens.
- [x] Compare actual invoke strings before and after restore. Do not compare only parsed object equality.
- [x] Reject duplicate keys, malformed discriminants, inherited editor keys, unsupported format, wrong target/namespace and byte/depth limits with zero requests.
- [x] Fail initial persistence and CAS: assert no transport dispatch and storage_error state.
- [x] Fail post-success persistence: assert original identity remains recoverable and no second command begins.
- [x] Run two helpers against one slot; fail CAS deletion of a newer record. Real receipt replay remains in Task 4.
- [x] Renew same principal with changed client generation and CSRF; assert current headers used and durable payload contains no auth/session fields.
- [x] Switch authority, kind and subject independently; assert quarantine, empty disclosed draft/result, zero retry dispatch and no automatic storage deletion.
- [x] Switch away then back while a reply is delayed; assert the stale completion cannot disclose data or revive state.
- [x] Retry after unknown with denied, conflict, identity_expired, identity_mismatch and history_gap responses; assert earlier commit knowledge stays unknown.
- [x] Exercise explicit not_committed on the first attempt and after a prior unknown; only the first establishes not_committed knowledge.
- [x] Abort before dispatch: zero requests and prepared intent retained. Abort after dispatch or dispose: unknown retained and no server-cancel call.

Use existing RemoteError and real client response decoding. Inject only transport/failure boundaries, not helper internals.
Store payload is a pending intent, not a receipt. Assert it contains no projected result.

## Task 3: Latest draft and navigation composition

- [x] Stage A, begin A, lose acknowledgement, then stage B and C.
- [x] Assert retry still sends A; C survives unknown state and reload as the latest unaccepted draft.
- [x] Confirm A through receipt response; assert C remains and no next request is sent automatically.
- [x] Explicitly begin C with key `save-C` and expected `8n`; assert the new command differs only through the host's chosen inputs/identity.
- [ ] Test a stale chosen revision against real conflict. No automatic read/rebase or new key follows.
- [ ] Add a Svelte host navigation barrier waiting for its save promise and checking hasUnresolvedIntent/hasDraft.
- [ ] Attempt navigation during unknown and with invalid editor text; retain the current view and draft.
- [x] Require explicit acknowledgePossibleCommit for local discard after dispatch. Assert discard sends no network action.

The generic helper does not receive route names or perform navigation. Invalid editor text stays in host draft storage.

## Task 4: Actual HTTP, durable adapters and server restart

Consumes: existing `rom_http::Http`, runtime declarations, adapter storage and normal invoke/replay.
Produces: `tests/recovery-host` and `studio/tests/mutation-recovery/consumer/tests/http.mjs`, parameterized over SQLite and redb.

- [x] Add fixture declarations for owner-authorized notes with exact numeric fields and a counted event mutation.
- [x] Run both adapters against separate disk paths, finite server budgets and isolated ports.
- [x] Add a response-dropping proxy that forwards to real HTTP and drops only after the store confirms row/event/receipt commit.
- [x] Execute save A, lose ack, close helper, restart host, recreate client and restore same intent.
- [x] Retry via actual HTTP and assert one revision advance, one committed event bundle and original durable receipt identity.
- [x] Repeat after same-principal session renewal and changed host stamp/CSRF fixture credentials.
- [x] Revoke current grants after commit; retry must disclose no result and preserve uncertainty locally.
- [x] Test changed principal, same key/different bytes, stale revision, delete ambiguity and retired retry epoch.
- [x] Test host CAS refusal before dispatch and acknowledgement suppression after confirmed server commit independently.
- [ ] Kill an already accepted server mutation before commit if selected for additional release acceptance. Process restart is not power-loss certification.

Run after approved dependency/ownership changes:

```sh
cargo build --locked -p rom-recovery-host
node studio/tests/mutation-recovery/verify.mjs OUTPUT_DIR --adapter sqlite --host-binary ABS_PATH --port 43282
node studio/tests/mutation-recovery/verify.mjs OTHER_OUTPUT_DIR --adapter redb --host-binary ABS_PATH --port 43282
```

The fixture must report adapter, source identity, lock hash, database path, command and actual row/event/receipt counts.
The fixture authority exposes receipt and journal inspection only on its disposable loopback host.
The proxy drops a response only after real adapter inspection confirms the committed receipt and row revision.

## Task 5: Independently installed Svelte consumer

- [x] Copy/archive producer source and install its public recovery/client/controls/styles entries into a real consumer dependency directory.
- [x] Verify no private source alias, checkout symlink, vendor patch or process-global recovery store is required.
- [x] The host fixture explicitly selects IndexedDB for pending intent and uses authenticated fixture principal metadata.
- [x] Execute save, dropped acknowledgement, page reload, session renewal and explicit retry in both engines on both adapters.
- [x] Stage B/C during A uncertainty; confirm navigation stays in place and C remains after A receipt confirmation.
- [x] Test principal switch: private draft/result hidden, no old-principal retry, explicit owner restore only.
- [x] Add a second non-chat consumer, such as an inventory note editor, with the same helper and a different navigation policy.
- [x] Check exact persisted invocation, real backend counts, active focus and retained invalid editor draft separately.

The supported CLI is `node studio/tests/mutation-recovery/verify.mjs OUTPUT_DIR --adapter sqlite|redb --host-binary ABS_PATH --port PORT`.
Select supplied artifact source with `--source-archive` or `--source-dir`.
`--admit` refuses authoring source and preparation-only execution.
`--prepare-only` performs frozen installation, type checks and builds without native or browser execution.
Its browser executables use the existing ROM_WEBKIT_EXECUTABLE and ROM_CHROMIUM_PATH environment variables.
Each child has a 180000ms timeout and 8MiB output limit; Playwright uses one worker.
The verifier must pass literal argv values, validate adapter/port/path inputs and retain partial evidence on failure.
Reserve separate ports and worker allocation through the root; do not reuse the component server.
Expected acceptance is at least four engine/adapter combinations, each with real HTTP backend assertions.

## Task 6: Integration and evidence

- [x] Run Studio check/unit checks and the new affected recovery browser suites.
- [ ] Confirm combined full-suite and release admission results through the coordinator.
- [ ] Validate `compose-durable-mutation-recovery` with pinned OpenSpec 1.14.0.
- [ ] Run `./scripts/check` through the repository toolchain after coordinator allocation.
- [ ] Obtain independent code and interface review.
- [ ] Run independent artifact/consumer verification against clean source before release integration.
- [ ] Record failed and passing runs, relevant dirty files, lock/tool versions and actual supported storage/identity limits.

## Review focus

A current denied retry must not be relabeled as proof that the original mutation never committed.
Stored intent must preserve exact serializer tokens, rather than rebuild with ordinary JSON or structuredClone.
Same principal means authority/kind/subject, not matching CSRF, host stamp or generation.
Later drafts cannot overwrite accepted identity or cause automatic action rebasing.
Durable storage admission precedes dispatch; CAS and post-commit storage failure cannot authorize a new command silently.

## Ownership state

Assigned implementation ownership covers recovery modules, client/recovery facades, new recovery unit tests and root recovery/codec exports.
Package metadata remains owned by the coordinator. The coordinator assigned `tests/recovery-host/**` and `studio/tests/mutation-recovery/**` for Tasks 4–5.
The coordinator supplied the native compile lease, which was released after the terminal build.
Any additional native build requires fresh coordinator allocation.

## Executed helper evidence

The initial public-helper red failed with `ERR_MODULE_NOT_FOUND`; see `/var/tmp/rom-r4-first-red.log`.
Adversarial red runs separately exposed generation, restore and post-notification disclosure races.
The corresponding failing logs remain under `/var/tmp/rom-r4-*-red.log`.
The final disclosure fix checks the binding after refusal notifications before rejecting the retry promise.

The file fixture performs real synchronous comparison and writes in one test process.
It does not certify filesystem durability, cross-process locking or power-loss behavior.
Injected transports exercise the real client codec and request lifecycle.
Their successful responses do not establish server commits, receipts or adapter behavior.
No maintained usability intake item is marked verified from these helper tests.
Domain workflow behavior stays consumer-owned.
AP-UX-004 maps to pending intent recovery and conservative commit knowledge.
AP-UX-005 maps to separate retained drafts and unresolved-intent state; navigation remains host policy.
AP-UX-006 maps to durable principal binding and current client generation checks; application view sequencing remains separate.

Final source identity, hashes, commands and logs are recorded in `/var/tmp/rom-r4-helper-evidence.json`.
Full local verification, independent review, native adapter acceptance and external browser acceptance remain required before integration.

## HTTP and consumer preparation evidence

At the preparation checkpoint, fixture source existed but native compilation and execution had not run.
The executed acceptance section below supersedes that checkpoint.
The proxy checks real receipt identity before acknowledgement suppression and restarts the actual host process.
Its retention control uses the public offline adapter retention API on a fresh destination.
The consumer selects IndexedDB strict transactions for pending intent CAS and separate editor draft storage.
Notes and inventory profiles apply different validation and navigation decisions to the same generic helper.
The inventory profile uses the fixture note Resource as its stock-note backing data.

An independently installed authoring-source preparation passed frozen `npm ci`, Svelte checks and Vite build.
Evidence: `/var/tmp/rom-recovery-prepare-green-20261007/prepared.json`.
The initial preparation failed on a missing CSS side-effect declaration, not on the recovery helper.
Its failed evidence remains at `/var/tmp/rom-recovery-prepare-20261007`.
Preparation reports `completed:false`; it is not browser, backend or release acceptance.

## Executed actual HTTP and browser acceptance

The fixture built with locked Cargo dependencies in rom-dev, cargo/rustc 1.99.0, jobs 2 and incremental compilation disabled.
`/var/tmp/rom-recovery-native-20261007/source.json` records compile-time source and lock hashes, toolchain and immutable binary hash.
Native source hashes remained unchanged through compilation. The coordinator then received the native lease.
Current workspace lock metadata is recorded separately from the binary's compile-time lock; later metadata updates do not imply a rebuild.

Both authoring-source candidate verifiers completed with exit zero:

- SQLite: `/var/tmp/rom-recovery-sqlite-complete-20261007/result.json`.
- redb: `/var/tmp/rom-recovery-redb-complete-20261007/result.json`.

Each run installed producer source physically through frozen npm ci and verified exact source hashes.
Each run passed Svelte checks, Vite client build and Vite SSR build of the public-import HTTP acceptance entry.
Each adapter passed real HTTP save, dropped acknowledgement, restart, exact accepted-body replay, same-principal session renewal, identity mismatch and stale revision conflict.
Deletion replay preserved its original receipt. Offline retention retired epoch zero; the helper preserved earlier uncertainty after identity_expired.
Host CAS refusal before dispatch left actual backend counts unchanged.

Each adapter passed five Chromium and five actual WebKit 2364 browser cases with one worker.
The 20 passes cover latest C draft retention, invalid editor text, blocked navigation/focus, current revocation, principal quarantine and inventory composition.
A held committed response cannot disclose a late result after principal change.
The actual IndexedDB pending payload contains the accepted identity and excludes fixture transport credentials and session metadata.
Browser executables: `/root/.nix-profile/bin/chromium` and `/var/tmp/rom-studio-webkit-2364/pw_run.sh`.

Counts are ordered `[resources, events, receipts, effects]`.
The HTTP save advanced from `[1,1,1,0]` to `[1,2,2,1]`; replay kept `[1,2,2,1]`.
Deletion advanced to `[1,3,3,1]`; deletion replay kept those counts.
Inspection retains authoritative raw `wire` text because convenience JSON number fields can round large integers.
The public client result and raw wire independently retain `9007199254740993` exactly.

All initial failures remain in their separate evidence directories.
They exposed installed TypeScript Node limitations, SSR entry coupling, static-file error handling and Chromium automatic POST replay.
The proxy now suppresses every acknowledgement for the accepted key until explicit test resume.
No recovery helper or producer source changed during these fixture corrections.

These runs use authoring source and report `admitted:false`. Final matching clean release-source acceptance remains required.
Process restart, fsynced single-process file CAS and strict IndexedDB completion do not certify power-loss behavior or cross-process file locking.
The fixture uses fabricated credentials and does not verify a production authentication provider or application-specific feedback.
