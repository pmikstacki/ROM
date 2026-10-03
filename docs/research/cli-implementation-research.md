# CLI implementation choices and failure boundaries

Research checked 2026-10-02. This supplements the [deferred capability roadmap](deferred-capabilities-roadmap.md) and [accepted CLI design](../../openspec/changes/harden-core-and-add-cli/design.md). The current deliverable is a generic CLI; Studio remains excluded. Library documentation establishes available mechanisms. ROM's tests must establish its actual behavior. This is not a comparative parser benchmark.

## Argument parsing

| Candidate | Documented mechanism | ROM assessment |
| --- | --- | --- |
| [clap 4.6.7](https://docs.rs/clap/4.6.7/clap/) | Derive and builder APIs, subcommands, generated help and argument validation | Selected, pinned. The declarative command tree keeps help and parsing aligned across discovery, queries, mutations and streams. ROM still owns semantic input checks and safe diagnostics. |
| [lexopt 0.3.2](https://docs.rs/lexopt/0.3.2/lexopt/) | Stream of options and values; application supplies interpretation | Viable small parser, but maintaining command semantics and help manually offers no demonstrated benefit for this CLI. Not selected; no claim that it is slower or less correct. |
| [argh 0.1.19](https://docs.rs/argh/0.1.19/argh/) | Derive-based parsing with a code-size focus and Fuchsia command-line conventions | Viable alternative for a smaller command surface. No measured binary-size requirement justifies switching from the selected parser. |

The selection is an engineering judgment based on the necessary commands. It is not evidence that one parser is universally best. Acceptance must exercise the actual binary. Cases include help, missing keys/revisions, invalid combinations, malformed JSON and secrets mistakenly supplied in arguments. Parser errors must not echo arbitrary credential-bearing input.

Readable diagnostics and composable JSON output have priority over a smaller line count.

## HTTP client and explicit retry ownership

Reuse pinned reqwest 0.13.5, already used elsewhere in ROM. Its [ClientBuilder](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) exposes timeout, redirect, proxy and TLS controls. Configure these deliberately: verified TLS, numeric loopback HTTP for local use, no redirects, no implicit proxy and bounded connect/request timeouts. Mark authorization headers sensitive. Credentials come from a file, not an inline command argument; reject URL userinfo and header injection. Broader proxy, trust-store and interactive login profiles belong to separately tested future work.

Explicitly select [retry::never](https://docs.rs/reqwest/0.13.5/reqwest/retry/fn.never.html): it disables even the library's default protocol-level retry behavior. ROM owns mutation identity and recovery. A timeout, dropped response or post-commit projection failure does not prove that a mutation failed to commit. The CLI must report an unresolved outcome, preserve the user's ability to replay the exact original invocation and avoid inventing a new idempotency key. Receipt replay still requires current authorization. This is ROM's application-level contract, not a guarantee supplied by reqwest.

Direct hyper would move additional client composition into ROM; an external curl process would add process and error-mapping boundaries. Neither currently answers an unmet requirement. They were not benchmarked against reqwest. Real TCP fault fixtures, rather than mocked response objects alone, must check credential handling, dropped replies, malformed success responses and explicit rejection categories.

## Streaming profile

The [WHATWG SSE parsing specification](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream) defines UTF-8, line boundaries, data fields and event dispatch. ROM consumes SSE framing but does not adopt browser EventSource's automatic reconnection behavior. A live snapshot and a durable journal batch have different recovery semantics. A journal cursor must survive intact. A history gap must not silently become a new subscription at the head.

Use incremental bounded parsing, not collecting the response body. Limit each complete frame, handle split UTF-8 and CRLF, preserve multi-line data, and reject incomplete or invalid ROM frames. Comments count as connection activity, not domain progress. Flush each complete result and retain no unbounded output queue. Slow or closed stdout and Ctrl-C must terminate the client predictably. Cancellation stops the wait. It does not stop a committed or supervised server mutation. A small parser matching ROM's restricted wire profile is acceptable only with adversarial framing tests. It is not presented as a general EventSource implementation.

## What verification must demonstrate

- One executable addresses two unrelated Resource kinds through actual HTTP on both SQLite and redb, without per-kind client code.
- Discovering metadata grants no data or operation permission; known-kind commands do not require discovery access.
- JSON preserves false, zero, null and omission, and rejects nested duplicate keys before sending.
- Requests, responses, frames and waits are bounded. No automatic retry, credential persistence or hidden reconnection occurs.
- A committed mutation followed by a lost or rejected response is recoverable through explicit same-request replay, without duplicate state changes or events.
- Terminal output cannot execute control sequences from field data, and diagnostics do not leak credentials or raw server bodies.
- The CLI runs from its extracted source package outside the workspace.

These are acceptance criteria, not claims of completed test runs. Executed results, exact revisions and remaining limitations belong in the release evidence after integration. Human-usability validation with external application authors remains separate from automated CLI correctness.
