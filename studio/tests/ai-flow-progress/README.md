# Application-owned AI flow progress consumer

This isolated fixture composes public Studio controls and strict IndexedDB intent storage with an application-owned transport to a real public Rust FlowClient. Both publication and ticket triage reuse the maintained `examples/ai-flows` domain code. It introduces no product REST endpoint and discloses no private AiRun Resource.

The host owns authorization. The browser submits only a run handle, exact expected revision, operation key, and resume or cancel action. An unknown acknowledgement retains the original identity through reload. Advisory Queued, Active, AwaitingRecovery, and ActivityUnknown labels do not certify committed effects or authorize retry. Only completed authorized RunView output is rendered as a result.

The owner composition uses public Button, Badge, wire codecs, and `openIndexedDbIntentStore`. Studio Resource mutation recovery currently constrains an acknowledgement to the expected revision or its immediate successor; FlowClient recovery can commit several transitions. The fixture therefore has one shared domain-independent operation identity controller instead of a fabricated Resource client adapter.

Prepare a native host from an already executed installed-consumer acceptance and its immutable inputs:

```sh
node studio/tests/ai-flow-progress/prepare-host.mjs OPTIONS_JSON INSTALLED_ACCEPTANCE_JSON PRODUCER_ROOT EXCLUSIVE_OUTPUT EXCLUSIVE_TARGET
```

The preparer copies the current fixture host and unchanged producer domain code, selects only admitted extracted ROM crates, checks archive and package identities, seeds the exact immutable producer lock before resolution, audits the offline dependency graph and registry lock, and records compiler, lock, binary and source provenance. It budgets compiler identity 30 seconds, each metadata phase 60 seconds, and build 600 seconds with two Cargo jobs and 8 MiB output per command. A distinct target prevents competing native writes.

Supply an extracted Studio source package and both actual browser executable paths, then run one adapter at a time with distinct outputs and ports:

```sh
node studio/tests/ai-flow-progress/verify.mjs --adapter sqlite --source-dir STUDIO_PACKAGE --host-binary HOST_BINARY --host-provenance NATIVE_SOURCE_JSON --fixture-freeze INDEPENDENT_FIXTURE_FREEZE_JSON --port 43286 EXCLUSIVE_OUTPUT
node studio/tests/ai-flow-progress/verify.mjs --adapter redb --source-dir STUDIO_PACKAGE --host-binary HOST_BINARY --host-provenance NATIVE_SOURCE_JSON --port 43287 EXCLUSIVE_OUTPUT
```

Set `ROM_CHROMIUM_PATH` and `ROM_WEBKIT_EXECUTABLE` before execution. The verifier physically installs the matching Studio package with the frozen npm lock, runs Svelte checks and Vite build, and requires all 12 domain cases in each browser engine. Each adapter budgets install 180 seconds, check 120 seconds, build 120 seconds, and browser execution 600 seconds. Tests use one worker, 45-second cases, separate preserved databases, and 60-second native host lifetimes.

Supply an independently recorded fixture SHA-256 inventory outside this folder. The verifier binds all maintained fixture inputs and the copied consumer/runtime inputs before build and after browser execution. It excludes only named build, dependency, browser report and native target directories. Both input inventories are retained in result.json.

For a failed startup, append `--diagnose-startup` to run only one real Chromium startup/inspection case, with a 60-second browser command budget. It writes `diagnostic-result.json` with acceptance_complete false; it cannot establish domain or release acceptance. The private fixture-failure-stages.jsonl records bounded phase, errno, native exit, output-byte and thread observations without exception text or capabilities. Case directories are bound to configured test/engine identities, so failed worker restarts cannot reuse another test's directory.

For small controller and protocol checks:

```sh
node --test studio/tests/ai-flow-progress/protocol.test.mjs studio/tests/ai-flow-progress/consumer/src/recovery.test.mjs studio/tests/ai-flow-progress/consumer/src/binding.test.mjs studio/tests/ai-flow-progress/fixture-fence.test.mjs studio/tests/ai-flow-progress/lock-seed.test.mjs
```

These unit tests do not prove native or browser acceptance. The browser cases require the actual Rust host and actual engines; they do not substitute an in-memory flow implementation. Fixture credentials remain server-side, except the separate test-control credential held by Playwright's Node process. Process restart is not power-loss certification. The reused command helper's process-group assumptions do not adversarially contain escaped descendants. This fixture records provisional composition evidence and does not admit a final release or replace human usability review.
