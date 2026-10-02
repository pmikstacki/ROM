# Bevy ecosystem for backend web applications

Research date: 2026-10-02. Status: **historical investigation of an unselected alternative**. The user subsequently selected Tokio plus Rayon for ROM. This note preserves findings from the earlier Bevy evaluation; its recommendations below apply only to that discarded premise and are not the current ROM architecture or specification. No Bevy integration is proposed for implementation. [Rust concurrency research](rust-concurrency.md) covers execution primitives. No combined dependency build, load test, security audit, or production deployment was performed.

## Recommendation

Reuse **Axum, Tower, and Tokio for the optional HTTP adapter**, and Bevy for ROM execution. Evaluate **bevy-tokio-tasks** as the small runtime-lifecycle integration before writing equivalent glue. Keep only ROM's necessary boundary: typed requests/results, bounded admission, and scheduled invocation of the authoritative action path. A standard bounded channel and reply channel are sufficient building blocks; there is no reason to invent a web framework, executor, or generic RPC bus.

There are actual Bevy web-server projects, but they do not yet establish a ready-made, version-compatible transactional resource backend for ROM. `bevy_webserver` is useful inbound-server precedent, while the similarly named `bevy_http` is an outbound asset loader. The strongest starting combination for a Bevy 0.19 experiment is ordinary Axum plus a compatible async bridge, with ROM APIs hiding all ECS types.

## Candidate matrix

“Adopt” means recommended for the first integration experiment, not a dependency already accepted. Versions, release dates, and declared licenses below were checked against the crates.io registry on the research date. Version compatibility means declared dependencies; it is not a tested lockfile. License labels are upstream declarations, not a transitive dependency audit.

| Candidate | Role and declared compatibility | Release / license | ROM disposition |
| --- | --- | --- | --- |
| [Axum 0.8.9](https://docs.rs/axum/0.8.9/axum/) | Inbound HTTP routing, extraction, responses, shared state; designed around Tokio/Hyper. No Bevy dependency. | 2026-04-14; MIT; manifest MSRV 1.80. [Registry](https://crates.io/api/v1/crates/axum/0.8.9) | **Adopt** for the HTTP adapter. Reuse existing middleware; do not expose `World` through handler state. |
| [bevy-tokio-tasks 0.19.0](https://docs.rs/crate/bevy-tokio-tasks/0.19.0) | Tokio runtime in Bevy; background tasks and callbacks on the Bevy thread. `bevy_app`/`bevy_ecs` ^0.19.0, Tokio ^1. | 2026-06-24; CC0-1.0. [Registry](https://crates.io/api/v1/crates/bevy-tokio-tasks/0.19.0) | **Evaluate first** for runtime setup and task spawning. Its generic callback path needs admission/cancellation checks described below. |
| [bevy_webserver 0.2.1](https://docs.rs/crate/bevy_webserver/0.2.1) | Actual inbound Axum server plugin. Bevy components ^0.16.0, `bevy_defer` ^0.14.0, Axum ^0.8.1. | 2025-04-29; MIT OR Apache-2.0. [Registry](https://crates.io/api/v1/crates/bevy_webserver/0.2.1) | **Evaluate as precedent or upgrade candidate**, not a drop-in Bevy 0.19 dependency. A port must be cheaper than direct reuse of Axum plus a bridge. |
| [bevy_defer 0.18.0](https://docs.rs/bevy_defer/0.18.0/bevy_defer/) | Async coroutines with ECS access; supports Bevy ^0.19.0. Its executor runs within the main schedule on one thread. | 2026-06-21; MIT OR Apache-2.0. [Registry](https://crates.io/api/v1/crates/bevy_defer/0.18.0) | **Evaluate when sequential async workflows need ECS access**. Internal implementation detail only; direct mutable-world access is not the public ROM action API. |
| [bevy_async_task 0.13.0](https://github.com/nuzzles/bevy_async_task/tree/111191ddee25c59677d66b2f8b8a6ddd68db9ef9) | System parameters for background tasks, result polling, and timeouts; Bevy ECS/tasks ^0.19. | 2026-06-25; Apache-2.0 OR MIT. [Registry](https://crates.io/api/v1/crates/bevy_async_task/0.13.0) | **Evaluate for finite background jobs**. Not an inbound server or durable worker queue. Verify the specific driver's runtime needs rather than treating every future as portable. |
| [bevy_remote 0.19.1 / BRP](https://docs.rs/bevy/0.19.1/bevy/remote/index.html) | Bevy's JSON-RPC remote ECS inspection/modification; optional HTTP transport. | 2026-09-28; MIT OR Apache-2.0. [Registry](https://crates.io/api/v1/crates/bevy_remote/0.19.1) | **Avoid as the ROM product API**. Optional controlled development inspection is a separate use. Built-in ECS mutation bypasses ROM's domain boundary. |
| [bevy_http_client 0.11.0](https://docs.rs/crate/bevy_http_client/0.11.0) | Outbound HTTP requests via ehttp; Bevy component crates ^0.19.0. | 2026-06-20; MIT OR Apache-2.0. [Registry](https://crates.io/api/v1/crates/bevy_http_client/0.11.0) | **Evaluate only for outbound adapter calls**. It does not accept HTTP requests. Existing OAuth/HTTP client libraries may already cover ROM's needs. |
| [bevy_http 0.2.0](https://docs.rs/bevy_http/0.2.0/bevy_http/) | Downloads assets over HTTP/HTTPS; Bevy ^0.12. | 2023-11-15; MIT. [Registry](https://crates.io/api/v1/crates/bevy_http/0.2.0) | **Avoid for this scope**: neither an inbound server nor a current backend integration. |
| [bevy_easy_database 0.3.0](https://docs.rs/crate/bevy_easy_database/0.3.0) | Automatic component serialization/persistence using Fjall; Bevy ^0.17. | 2025-11-17; MIT OR Apache-2.0. [Registry](https://crates.io/api/v1/crates/bevy_easy_database/0.3.0) | **Study, but avoid as authoritative ROM persistence without conformance evidence**. Its component autosave contract does not establish ROM's atomic state/event/outcome transaction or concurrency guarantees. |
| [rx_bevy 0.4.0](https://crates.io/crates/rx_bevy/0.4.0) | Reactive operators/observables over Bevy; associated 0.4.0 common/resource crates depend on Bevy ^0.19.0. | 2026-06-20; MIT. [Registry](https://crates.io/api/v1/crates/rx_bevy/0.4.0) | **Evaluate only if reactive composition simplifies real backend code**. It does not replace durable ROM reactions; inspect features because the common crate also declares `bevy_window`. |

Dependency evidence is available in the registry's version-specific endpoints, for example [bevy_webserver dependencies](https://crates.io/api/v1/crates/bevy_webserver/0.2.1/dependencies), [bevy_defer dependencies](https://crates.io/api/v1/crates/bevy_defer/0.18.0/dependencies), [bevy_async_task dependencies](https://crates.io/api/v1/crates/bevy_async_task/0.13.0/dependencies), [bevy_http_client dependencies](https://crates.io/api/v1/crates/bevy_http_client/0.11.0/dependencies), and [rx_bevy_common dependencies](https://crates.io/api/v1/crates/rx_bevy_common/0.4.0/dependencies).

The exact crate name `bevy_axum` returned 404 from the [registry endpoint](https://crates.io/api/v1/crates/bevy_axum) on the research date. That is a scoped search result, not proof that no Git-only integration exists. `bevy_signals` 0.0.1 is a [reserved Bevy crate name](https://crates.io/api/v1/crates/bevy_signals), not a usable reactive implementation. Do not select crates merely from plausible names.

## What the server examples actually demonstrate

**bevy_webserver** has a real [headless CRUD example](https://github.com/MalekiRe/bevy_webserver/blob/6ca2490440b69f0233714b903b037131b4006e1f/examples/crud_app.rs): `MinimalPlugins`, Axum routes, HTML forms/HTMX, and component persistence. Handlers directly query, spawn, mutate, and despawn ECS entities. This demonstrates embedding a web application in Bevy; ROM should replace those direct mutations with its authorized action API. It is example source, not evidence of a production deployment or an executed compatibility test here.

Its [server implementation](https://github.com/MalekiRe/bevy_webserver/blob/6ca2490440b69f0233714b903b037131b4006e1f/src/lib.rs#L51) uses `async_io`, `smol_hyper`, Hyper HTTP/1 connections, and `bevy_defer` tasks. It does not use the ordinary `axum::serve` Tokio host. A Tokio-dependent database client or timer cannot simply be assumed to work in those handler futures. The accept loop explicitly yields to the Bevy schedule. These details make the project a useful alternative topology to test, not a transparent wrapper around the recommended Tokio setup.

**async_bevy_web** is another genuine [experimental server-side integration](https://github.com/vertec-io/async_bevy_web/tree/a380e9d4633442ca0003b07dfc2873aa71fe4be7), combining Bevy with Axum/Leptos examples. Its [workspace manifest](https://github.com/vertec-io/async_bevy_web/blob/a380e9d4633442ca0003b07dfc2873aa71fe4be7/Cargo.toml) targets Bevy 0.17.0, and its [Leptos plugin](https://github.com/vertec-io/async_bevy_web/blob/a380e9d4633442ca0003b07dfc2873aa71fe4be7/crates/bevy-leptos/src/lib.rs) starts an application future using a locally included Tokio bridge. Useful architectural reading; avoid adopting its full frontend/macro workspace for a backend library. No top-level license file or package license declaration was found in the inspected root and two integration manifests, so source reuse requires clarifying the applicable license. Do not infer whole-project licensing from license files in individual examples or bundled crates.

These examples establish feasibility, not a large established ecosystem of general-purpose Bevy business backends. Bevy's own [headless example](https://github.com/bevyengine/bevy/blob/main/examples/app/headless.rs) verifies the supported shape of a no-window application and scheduled runner; it is not an HTTP server example. A graphical Bevy client talking to a separate Rust backend is also not evidence that its backend uses Bevy.

## Async bridge details that affect reuse

`bevy-tokio-tasks` provides a configurable runtime constructor and schedule label, plus Tokio task handles. Reuse those if they fit the host lifecycle. Its inspected implementation uses an [unbounded callback channel](https://github.com/EkardNT/bevy-tokio-tasks/blob/b016e01cf87b133e84f5390ee278ff28d0a81bbc/src/lib.rs#L125) and [drains available callbacks in an exclusive system](https://github.com/EkardNT/bevy-tokio-tasks/blob/b016e01cf87b133e84f5390ee278ff28d0a81bbc/src/lib.rs#L173). Consequently, ROM still needs admission limits and a work budget if this callback path carries requests; the plugin is not a bounded request queue.

The cancellation caveat is precise: after `run_on_main_thread` has enqueued its callback, if the waiting future is dropped and the callback later executes, the callback runs the closure and [panics when sending its result to the dropped receiver](https://github.com/EkardNT/bevy-tokio-tasks/blob/b016e01cf87b133e84f5390ee278ff28d0a81bbc/src/lib.rs#L235). This is source inspection, not a reproduced runtime test, and not a claim that every task cancellation panics. A narrow solution is to reuse runtime/spawn support while sending typed requests over a standard bounded channel with cancellation-tolerant replies; alternatively fix/test the callback behavior upstream. Avoid replacing the entire plugin just to address this one path.

The plugin also performs a small `Runtime::block_on` in its tick function. Keep the synchronous Bevy runner outside an already-entered Tokio async task unless that arrangement is specifically tested; make runtime ownership and shutdown explicit. [Tick implementation](https://github.com/EkardNT/bevy-tokio-tasks/blob/b016e01cf87b133e84f5390ee278ff28d0a81bbc/src/lib.rs#L170).

`bevy_defer` is appropriate when suspended workflows genuinely benefit from scheduled world access. Its single-thread executor means a blocking call or long CPU section can delay the schedule. Keep its APIs inside the engine implementation. Its [signals](https://docs.rs/bevy_defer/0.18.0/bevy_defer/signals/index.html) keep one value and can skip rapid updates; they suit wakeups/latest-state hints, not delivery of every committed fact. Start with Bevy's existing observation mechanisms unless a coroutine or reactive library removes demonstrated complexity. [bevy_defer execution model](https://docs.rs/bevy_defer/0.18.0/bevy_defer/).

## Minimal reusable topology

This is a proposed composition, not an implemented API:

```text
HTTP client
  -> Axum / Tower limits, decoding, authentication
  -> ROM handle: typed command + trusted actor + expected revision + retry key
  -> bounded standard channel
  -> headless Bevy schedule running ROM authorization/validation/transition stages
  -> persistence adapter on its required I/O runtime
  -> durable commit: state + events + idempotent outcome
  -> bounded completion into the Bevy schedule
  -> authorized ROM result over a reply channel -> HTTP response

Durable events -> ROM delivery/reaction policy -> optional SSE/WebSocket adapter
```

Bevy drives the ROM execution stages; Tokio serves the optional network/database I/O that requires it. Nothing in HTTP handlers receives a raw `World`, `Entity`, component name, or mutable ROM storage reference. Public resource IDs and action schemas remain ROM-owned. Persistence completions must be reconciled with revision/order guarantees before updating in-memory projections or publishing success. An HTTP timeout means the caller stopped waiting; it is not evidence that a database commit failed.

Reuse [Tower concurrency/rate limits](https://docs.rs/tower/latest/tower/limit/index.html) and [Tower HTTP middleware](https://docs.rs/tower-http/latest/tower_http/) for transport concerns, and [Tokio bounded channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html) for admission. A response receiver disappearing should be an ordinary condition after a committed action. Choose an explicit headless wake/tick strategy and measure request latency and idle CPU; do not inherit 60 Hz merely because an example does. One slow database future must not block Bevy's whole world.

The adapter should reuse Axum's SSE/WebSocket support if required, but ROM still owns authorization, durable cursors, projection, and reconnect semantics. BRP is a different interface: its [HTTP plugin](https://docs.rs/bevy_remote/0.19.1/bevy_remote/http/struct.RemoteHttpPlugin.html) exposes remote ECS inspection/modification and defaults to localhost. CORS headers or loopback binding do not turn built-in ECS operations into ROM actions. Leave it out of the product-facing API.

## Maintenance and compatibility evidence

Latest inspected default-branch commit dates were: [bevy_webserver 2025-04-29](https://github.com/MalekiRe/bevy_webserver/commit/6ca2490440b69f0233714b903b037131b4006e1f), [bevy-tokio-tasks 2026-06-24](https://github.com/EkardNT/bevy-tokio-tasks/commit/b016e01cf87b133e84f5390ee278ff28d0a81bbc), [bevy_defer 2026-09-19](https://github.com/mintlu8/bevy_defer/commit/1af364bb32864f1a8ab891be5308464361b7bb7a), [bevy_async_task 2026-06-25](https://github.com/nuzzles/bevy_async_task/commit/111191ddee25c59677d66b2f8b8a6ddd68db9ef9), [async_bevy_web 2025-11-10](https://github.com/vertec-io/async_bevy_web/commit/a380e9d4633442ca0003b07dfc2873aa71fe4be7), and [bevy_easy_database 2025-11-17](https://github.com/MalekiRe/bevy_easy_database/commit/285e080f852a6b11952a4be5136abb0d8559ba92). These show observed activity, not promises of support, security response, or API stability.

The integrations inspected are pre-1.0 and often track Bevy minor releases. A 0.16 `World` is not interchangeable with a 0.19 `World`; allowing Cargo to resolve both versions does not make plugins interoperable. Pin a coherent version family, inspect the dependency graph and feature union, and keep Bevy-facing types out of ROM's public contract. Registry evidence supersedes stale search snippets: for example, current `bevy_http_client` is 0.11.0/Bevy 0.19, while older indexed documentation still shows 0.10.0/Bevy 0.18.

Before adopting the stack, run one small experiment with a selected supported compiler and lockfile: headless Bevy + Axum + bridge, one authorized action, a delayed persistence completion, a dropped requester after enqueue, bounded overload, and graceful shutdown. Compare the existing plugin plus minimal glue against plain host-owned Tokio integration. Choose the smaller verified composition. Separately test persistence rollback and state/event/outcome atomicity; component autosave examples cannot establish those guarantees.
