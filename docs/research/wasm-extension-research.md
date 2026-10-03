# WASM extension boundary: evidence and next integration experiment

Research date: 2026-10-03. ROM source baseline:
`64240cf99e4632378e62db91f3bf151f2d306e6f`. This extends the deferred-roadmap investigation. It does not add a maintained runtime. The premise remains unchanged: application authors declare Resource types in Rust and compose native modules.

**Recommendation:** first evaluate a pure, bounded validator/proposal guest with
no host imports. Treat its output as untrusted input to the existing ROM command
path. Keep authorization, identity, revisions, idempotency, storage and events
in native ROM. Prefer a standalone experiment before introducing a new public
callback seam. Wasmi is sufficient for this initial core-module experiment;
Wasmtime's component/WIT route deserves a separate typed-contract experiment.

**Observed** means source inspection or the experiments recorded below.
**Documented** means a linked primary source describes the mechanism.
**Proposed** means a design, budget or experiment that is not yet implemented.

## Current native boundary

[Resource, Field, Action and Definition](../../crates/rom/src/resource.rs) are
compiled Rust contracts. Public `Action::new` accepts a function pointer,
`fn(&mut R, I) -> Result<Vec<Intent>>`; [Reaction::new](../../crates/rom/src/reactions.rs)
also takes a function pointer. The internal erased action callback can hold
state, but it is not a public factory for captured application state. A Wasm
engine/module cache therefore cannot simply be captured by a public action
constructor today. A global registry or thread-local bridge would hide ownership, versioning and lifecycle. This report recommends neither.

Field decoding is also a static native contract. A separate guest validator is
not automatically a mandatory invariant across create, replace, patch and
action execution. Making it mandatory requires a reviewed integration seam
covering every relevant path. An outer application wrapper can demonstrate a
proposal safely, but cannot claim that all callers must use it.

ROM's public `Input` codec does not supply a universal action-input schema from
which arbitrary component types can be generated. Resource descriptors and
field shapes help discovery; they do not establish a complete, versioned guest
ABI. The first experiment should state its small schema explicitly.

A native Rust plugin is trusted application code, not a sandbox. Rust's native
ABI has no stability guarantee, while `cdylib`/C-compatible interfaces solve a
different linkage problem. A native dynamic-loader design would need its own
ABI and lifecycle review. [Rust external ABIs](https://doc.rust-lang.org/reference/items/external-blocks.html),
[Rust linkage](https://doc.rust-lang.org/reference/linkage.html).

## Engine choices and their actual scope

| Choice | Documented mechanism | ROM consequence / recommendation |
|---|---|---|
| Linked native Rust | Current Resource/Field/action/adapter contracts and normal Rust compilation. | Default authoring model; maximum host trust and existing type integration. Measure this as the baseline. |
| Wasmi 2.0.0 core module | Interpreter with fuel and store resource limits; this investigation uses core `Module`, `Linker` and typed exports. | Small standalone probe is practical. Define and validate a core-module ABI. No component implementation was exercised or established here. |
| Wasmtime component + WIT | Component bindings describe typed imports/exports; generated host bindings support the component interface. | Better candidate for a versioned multi-language interface; extra component tooling, lifting/lowering, runtime and build costs must be measured. |

Wasmi's project describes its constrained-environment/interpreter focus. This
supports trying it, not a claim that it is always smaller or faster for ROM.
The component model's WIT describes types, interfaces and worlds rather than
application behavior. Wasmtime can generate host bindings from WIT; neither
mechanism supplies ROM authorization or transaction semantics.
[Wasmi project](https://github.com/wasmi-labs/wasmi),
[Wasmi API](https://docs.rs/wasmi/latest/wasmi/),
[WIT design](https://component-model.bytecodealliance.org/design/wit.html),
[Wasmtime bindings](https://docs.rs/wasmtime/latest/wasmtime/component/macro.bindgen.html).

Do not compare the same textual WAT core module against a Rust component and
attribute every difference to the engine. First compare equivalent algorithms
and contracts; then report component conversion and data-transfer cost separately.
No Wasmtime dependency, benchmark or component executable was built here.

## Proposed authority and data flow

1. The host selects a pinned, locally approved guest artifact and contract.
2. It reads only the authorized fields necessary for the operation and builds a
   bounded request. Hidden fields must not become guest input merely because the
   host can access their raw storage representation.
3. A fresh, limited guest instance computes a rejection or candidate proposal.
4. The host checks the output envelope, version, size, schema and semantic limits.
5. The host constructs the ordinary ROM command with the original caller,
   host-selected target, expected revision and invocation identity. ROM repeats
   its current authorization and decoding checks at the real operation boundary.
6. ROM alone determines the committed result, receipt and emitted events.

The guest must not choose a principal, target key, invocation ID, expected
revision or bypass policy. An allowed kind/field manifest is an additional
restriction, not a grant. Do not expose raw storage, transaction handles,
filesystem, sockets, clocks, randomness or unrestricted logging in the first
profile. No WASI imports are needed for a pure proposal. Wasmtime's sandbox
security depends on the capabilities the embedder makes available.
[Wasmtime security model](https://docs.wasmtime.dev/security.html).

A host wrapper can fail before submitting any ROM command. After command submission, a timeout or cancellation must not be described as proof that nothing committed. Use the existing invocation/receipt recovery semantics.
Likewise, guest failure must not produce a synthetic successful event or bypass
core validation. A guest “compensation” proposal remains domain-authored input
for a later authorized command, not automatic rollback.

## Wire contract and allocation boundaries

**Proposed manifest:** ABI version, plugin semantic version, artifact digest,
engine/configuration profile, allowed exports/imports, supported kinds and field
codec versions, input/output byte limits, fuel/memory limits and determinism
policy. Reject incompatible versions before running guest code. Pin the exact
manifest for an invocation, not only a mutable plugin name.

For the first core-module experiment, return a pointer/length pair into bounded
linear memory. Interpret integer widths explicitly, reject length above the
configured cap before allocating or copying, use checked range arithmetic,
then decode exactly one schema. Reject invalid UTF-8, unknown/duplicate fields,
unsupported enum cases, invalid numbers and trailing data. Typed Rust codecs
must still validate the resulting ROM value. The probe implements a very small
`{quantity: u64}` output, not a universal JSON transformation API.

WIT `list` and `string` types do not themselves express a maximum length. A
high-level generated return value may already have been lifted into host-owned
storage before application code examines its size. Consequently, “typed WIT”
is not evidence that ROM's preallocation byte bound holds. Begin a component
probe with fixed-size scalar records/enums, or separately establish and test a
bounded transfer interface. The exact generated bindings and runtime version must be inspected. Do not assume that all lifting paths are bounded.
[WIT types](https://component-model.bytecodealliance.org/design/wit.html),
[Wasmtime typed component calls](https://docs.wasmtime.dev/api/wasmtime/component/struct.TypedFunc.html).

Illustrative fixed-size experiment interface, **not an accepted ROM ABI**:

```wit
package rom:proposal@0.1.0;
interface quantity-proposal {
    record request { abi-version: u32, quantity: u64, decrement: u64 }
    enum rejection { invalid-input, unsupported-version }
    propose: func(input: request) -> result<u64, rejection>;
}
world pure-proposal { export quantity-proposal; }
```

This domain-specific fixture is useful for testing bindings. A later reusable
host adapter should own transfer, limits, versions and error mapping, so each
Resource author supplies domain logic rather than a new sandbox implementation.

## Resource control and cancellation

| Resource | Documented mechanism / observed limitation | Proposed acceptance condition |
|---|---|---|
| Guest instructions | Both runtimes support fuel. Wasmtime additionally has epoch interruption; these are distinct controls. | Enable before instantiation/start, set a finite per-call budget, test both loops and start functions. Calibrate fuel against useful guest work. |
| Native host work | Guest fuel does not bound arbitrary native import work; the probe demonstrates this. | First profile has no host imports. Future imports need their own admission, deadline, byte and concurrency limits. |
| Linear memory/tables/instances | Store limiters bound selected Wasm resources; limits are not a universal process-memory cap. | Bound each memory and the number of memories/tables/instances, plus table elements. Exercise failed growth and initial allocation. |
| Compilation | Wasmi has configurable compilation mode and enforced limits; default guest fuel alone is not a compilation budget. | Cap artifact bytes and parser structure, bound compile workers/queue, reject oversized modules before compilation, measure peak RSS. |
| Output/host heap | Guest memory limits do not cap all host allocations. | Check transfer length/range before copy; use bounded schemas, depth and collection counts; test malformed envelopes. |
| Wall clock/cancel | Wasmtime epochs require the engine epoch to advance; interruptible guest execution does not make blocking host I/O interruptible. | Keep CPU permits until execution actually stops. Do not drop an async waiter and assume its blocking worker stopped. |
| Cache/state | Reusing compiled modules can avoid parse/translation work, but mutable instances can retain guest state. | Cache approved immutable modules; use a fresh store/instance per request initially. Test cross-caller data isolation before any instance pooling. |

Sources: [Wasmi configuration](https://docs.rs/wasmi/latest/wasmi/struct.Config.html),
[Wasmi store](https://docs.rs/wasmi/latest/wasmi/struct.Store.html),
[Wasmi store limits](https://docs.rs/wasmi/latest/wasmi/struct.StoreLimitsBuilder.html),
[Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html),
[Wasmtime resource limiter](https://docs.wasmtime.dev/api/wasmtime/trait.ResourceLimiter.html).

An initial integration experiment could start at a 1 MiB artifact, one memory
up to 16 MiB, 64 KiB request/output caps, one compilation worker and two active
runs. These are **proposed trial values**, not justified defaults or measured
ROM requirements. The actual tiny probe uses one 64 KiB memory, 64 output bytes,
10,000 fuel units, one instance, one table and at most 16 table elements. Its
synthetic timing loop raises the fuel budget only to accommodate repeated calls.

Compilation caches must bind artifact digest, engine version, configuration and
target. Never accept untrusted precompiled native cache blobs as equivalent to
validated Wasm bytes: Wasmtime's deserialization API has explicit safety
requirements. Module caches and durable command receipts serve different
purposes. [Wasmtime module deserialization](https://docs.wasmtime.dev/api/wasmtime/struct.Module.html#method.deserialize).

If a retry recomputes a proposal after a plugin upgrade, it may change the command
payload. Freeze the chosen plugin version and final canonical invocation before
submission, and recover that invocation through existing receipt semantics.
Do not silently rerun “latest plugin” while recovering an uncertain mutation.

## Executed standalone experiment

The isolated branch `codex/prototype-wasmi-capability` contains
`experiments/wasmi-capability-probe`, an independent Cargo workspace with no ROM
dependency. Correctness source initially frozen at
`049f90009f1f0130e41979c0eb08a00ff5f36c4e`; final source and the cost-observation
log are frozen at `7f099baa3bc167208d53a7e3655c9b1c41703137`.
Wasmi is pinned to 2.0.0, with a committed lockfile whose SHA-256
is `eb9ba25f09243f4681c7df6b4dd1f726a174ba22372905684e57bd2333ec6e5b`.

Nine tests pass:

| Executed condition | Observed result |
|---|---|
| Infinite exported guest loop | Fuel exhaustion trap. |
| Infinite guest start function | Fuel exhaustion during instantiation/start. |
| Unprovided `host/database-write` import | Instantiation rejected. No actual database import exists. |
| Explicit synthetic native import performs 100,000 atomic increments | Completes with guest fuel remaining; native work is not metered like guest instructions. |
| Guest requests memory growth beyond host's one-page cap | `memory.grow` returns -1; memory remains 65,536 bytes. |
| Valid `{quantity:7}` output | Accepted after length, range, JSON and semantic checks. |
| Claimed output length `u32::MAX` | Rejected as oversized before memory access/copy. |
| Out-of-range pointer | Rejected by checked range validation. |
| Invalid UTF-8, duplicate/unknown fields, quantity 101 or null | Rejected by schema/semantic validation. |

A negative control temporarily increased the memory limit to ten pages. The
memory-growth test then failed with actual growth result 1 instead of -1
(exit 101). Restoring the original limit restored the green suite. This shows
that the fixture detects loss of that guard; it is not a fuzzing or engine
security audit. Formatting and Clippy with warnings denied also pass.

Reproduce from the immutable prototype checkout:

```sh
cd experiments/wasmi-capability-probe
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo test --locked --release
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo test --locked --release repeated_release_cost_observation -- --ignored --nocapture
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo clippy --locked --all-targets -- -D warnings
```

The environment is the native `rom-dev` NixOS container, Rust
`1.99.0 (b940084d7 2026-09-28)`, two Cargo jobs, debug information disabled for
dev/test. No Wasm target standard library, Wasmtime executable or component
compiler was installed when inspected; `rustc --print target-list` advertising
a target does not mean its standard library is installed. Wasmi dependencies
were downloaded for this small workspace. WAT fixtures avoid requiring a Rust
guest cross-compilation toolchain. No full Wasmtime build was needed.

### Cost observation

The release correctness suite passed 9 tests, with the timing test ignored by
default; running that test explicitly also passed. Five samples each performed
200 module/instance iterations and 2,000 call/validation iterations. The table
reports median and range of the five aggregate per-iteration means, rounded to
integer nanoseconds. These are not per-call latency percentiles.

| Operation | Median ns/iteration | Sample-mean range ns |
|---|---:|---:|
| Textual WAT parse/validate/module construction, existing engine | 9,852 | 9,668–11,746 |
| Fresh store + instantiate/start cached module, no proposal call | 1,005 | 974–1,312 |
| Warm export lookup + two guest calls + bounded output validation | 162 | 162–208 |
| Warm cached typed exports, two calls only | 76 | 76–94 |
| Host JSON/schema/semantic validation only, 13-byte output | 15 | 15–19 |

Raw samples are committed as `experiments/wasmi-capability-probe/release-observation.log`
at the final prototype SHA. The release build took 18.59 seconds in this run. This is one environment observation, not a comparative build benchmark. The
target occupied 334 MiB after dev and release builds. The author-skill fixture
target occupied another 246 MiB. Neither number measures runtime memory.

Module reuse avoids repeated module construction in this fixture. Cached export
handles are a plausible smaller optimization, but the calls-only row omits
validation and therefore cannot establish a full-path speedup. The warm cases
reuse mutable state for measurement and are not the recommended cross-caller
isolation policy. A production comparison must include fresh-instance cost.

Wasmi's lazy translation was warmed before the call samples; module construction
does not necessarily account for all eventual function translation. Timing
includes loop/drop overhead and lacks process isolation or confidence intervals.
This tiny static WAT output is not representative of a Rust guest, realistic
domain work, sustained throughput or production isolation. Peak RSS and allocator
counts remain unmeasured. There is no measured native-versus-Wasm speed ratio.

## Next executable trial and decision gates

1. **Keep correctness first, then optimize the boundary.** Record repeated release
   samples for a native function and equivalent Wasmi guest, separate cold module
   construction from warm module reuse, and include fresh-instance cost. Measure
   wall time, guest fuel, peak RSS and host allocations; do not infer allocation
   counts from source inspection. Cache typed exports where measured worthwhile,
   while keeping per-request state isolation.
2. **Demonstrate proposals through actual ROM, on a prototype branch.** Use two
   unrelated Resource types and SQLite/redb. Read authorized input, run the
   pinned guest, construct a normal command and assert real receipts/events.
   Change the revision or revoke authorization between proposal and command;
   expect core rejection. Trap/oversize/malformed guest output must emit no
   command. Replay the exact submitted invocation after a plugin-version change
   and demonstrate receipt stability. This is not implemented by the engine test.
3. **Assess the smallest public integration seam.** A captured, bounded proposal
   adapter may suffice outside core for optional automation. A mandatory validator
   needs a deliberate, comprehensive mutation-path contract. Do not introduce a
   global function-pointer workaround, a second persistence path or a separate
   authorization DSL to avoid discussing that seam.
4. **Run a separate fixed-size WIT component trial.** Pin Wasmtime and build tools,
   compile a tiny Rust component after installing only the necessary target/tool,
   validate imports/exports and cancellation, then measure equivalent work.
   Variable-length transfer is a later gate requiring preallocation evidence.
5. **Require deployment evidence before promotion.** Repeat limits under concurrent
   callers, malformed-module fuzz inputs, plugin upgrades, shutdown/cancellation,
   cold starts and configured cache eviction. Record binary/build cost, peak
   memory and p50/p95 latency separately. Review dependency/advisory/license
   policy for the selected pinned release; this research does not approve a
   universal production runtime profile.

The current evidence supports feasibility of a small capability-free proposal
sandbox. It does not establish ROM integration, universal provider compatibility,
component portability, wall-clock cancellation of native work, malicious-module
compilation bounds, or production performance. The adjacent
[skills-library research](rom-skills-library-research.md) keeps the author-facing
workflow generic while distinguishing this future execution boundary.
