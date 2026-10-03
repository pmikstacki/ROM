# Maintained credential verification results

2026-10-02. Maintained source: `c560db3` plus review correction `041ee12`.
The optional `rom-auth` crate has independent `jwt` and `introspection` features.
Its default dependency graph is empty. It returns immutable, expiry-bounded
identity evidence. It does not define a second User engine or bypass Resource
policy. User/provider linking and current configuration activation are the next
integration package, not claims of these commits.

The coordinator reran the full main verifier, including the auth feature matrix:
19 profile tests, two public examples and two compile-fail cases passed.
Existing maintained Resource and SQLite/redb tests also passed. Core's ordinary
no-default-feature graph contains no authentication implementation. The test
fixtures use ephemeral signed tokens and actual loopback HTTP, without deployed
identity providers or real credentials.

Independent review showed that `81de695` accepted malformed signed `iss` arrays.
The upstream JWT library validated those arrays by set intersection. ROM promised a string issuer.
The correction accepts only a typed string with an exact binding. Regression tests use real signatures with singleton and mixed arrays.

Further tests of typed claims showed that the decoder treated optional `nbf: null` as absent.
If an optional time claim is present, it now must decode to its declared type. Supported
string/list audiences remain valid. These tests passed in the coordinator's
maintained run; they are not merely documentation assertions.

The JWT profile uses jsonwebtoken 11.1.0 with AWS-LC; introspection uses reqwest
0.13.5 with Rustls. Profile limits, key acquisition responsibilities and explicit
unsupported protocols are in [the crate README](../../crates/rom-auth/README.md).
No universal OIDC/session implementation or real provider interoperability is
claimed.

The combined lockfile audit scanned 211 dependencies with cargo-audit 0.22.2,
`--deny warnings --no-fetch`, against RustSec commit
`117edb3bed98e9be112f277b7615eea3252e7c43`. It reported no advisory findings.
This is an advisory snapshot; native SQLite patches are checked separately in
[the dependency audit](mvp-dependency-audit.md).

Reproduce from the repository root inside `rom-dev`:

```sh
CARGO_NET_OFFLINE=true ./scripts/check
```

The root verifier now always tests all auth features and their independent builds.
It also runs Clippy, docs, and immutable-proof compile failures. A Cargo test with only default features does not establish those results.
