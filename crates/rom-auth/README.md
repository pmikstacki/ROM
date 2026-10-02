# rom-auth

Optional credential verification for ROM hosts. `jwt` and `introspection` features return immutable `VerifiedIdentity` evidence: authority, opaque subject, human/service kind and exclusive expiry. Default features are empty; this crate depends on neither `rom` nor a User/configuration engine.

```toml
rom-auth = { path = "../rom-auth", features = ["jwt", "introspection"] }
```

See the public module docs for configured-key and HTTPS introspection examples. Tokens, client credentials and arbitrary claims are absent from the returned proof. It has no public constructor or deserializer. `Debug` omits its subject; `AuthError` has static categories rather than raw provider responses. Native host code remains trusted.

The JWT profile accepts RS256, exact issuer and intended audience, `typ=at+jwt` or `application/at+jwt`, mandatory `sub`, `exp`, `iat`, `jti`, `client_id`, and the provider extension `principal_kind=human`. Issuer is a required string and must match exactly; singleton and mixed issuer arrays are rejected. Audience accepts a string or a list containing the intended resource. Present claims must have the declared type: explicit null cannot disable optional time checks or proof requirements. It checks optional `nbf`, uses zero leeway and accepts at most a one-hour issued lifetime. It rejects sender constraints and token-specified keys/URLs. The host provides a complete trusted public-key set; refresh is limited to once per five seconds, eight keys and thirty seconds of key/proof validity. Remote discovery, bounded key-source acquisition and fetch coalescing remain host responsibilities.

Client ID and secret are form-encoded before HTTP Basic encoding, as required by [RFC 6749 §2.3.1](https://www.rfc-editor.org/rfc/rfc6749.html#section-2.3.1); reserved characters are exercised against the actual HTTP fixture. The introspection service profile requires authenticated HTTP POST, active status, issuer/audience binding, expiry, Bearer type, `principal_kind=service` and `sub=client_id`. HTTPS is the normal explicit endpoint policy. `LoopbackTestOnly` accepts only numeric loopback HTTP fixtures. Redirects/proxies are disabled, connect/request timeouts are 250/500 ms, token/response limits are 4/16 KiB, and an eight-entry digest-keyed cache expires within five seconds or token expiry. Unsupported or ambiguous profiles fail closed. Use the blocking client outside async runtime workers.

All time checks use the supplied **trusted host** Unix time. Never accept that argument from a request. These fixed profile bounds are current adapter behavior, not selected fleet-wide security policy. A previously returned proof remains evidence only until `valid_until`; invalidating a verifier cache does not revoke an already issued actor.

## Host integration boundary

There is deliberately no compiled `Actor::trusted` conversion. The maintained core expiry API provides `.expires_at(verified.valid_until())`, but its current actor identity still lacks principal kind. Integration must either extend actor/receipt identity with that kind or apply a validated host mapping from `(verified.authority(), verified.principal_kind())` to a distinct authority namespace **before** calling `Actor::trusted(mapped_authority, verified.subject()).expires_at(verified.valid_until())`. An unqualified direct conversion would silently merge human/service namespaces. The core must enforce expiry on every action/read/retry/event delivery and resolve explicit User bindings through its Resource derive and shared persistence/actions. Disabling a User or changing IdentityProvider trust needs current policy/configuration generation checks. Do not create another User store or promote the disposable prototype's Core.

Only the host should construct adapters from authorized, validated IdentityProvider Resource state. Changing a configuration Resource is not yet wired to verifier replacement or existing-proof invalidation. First-admin recovery, durable linking uniqueness, issuer generations, real provider interoperability, TLS failure tests, discovery, secret rotation and delegated client identity remain separate integration work. This crate implements two narrow profiles, not universal OIDC login, sessions, JWE, mTLS or DPoP.

## Verification and dependencies

From this repository in native `rom-dev`:

```sh
CARGO_TARGET_DIR=target/auth ./crates/rom-auth/verify
```

The verifier checks independent feature builds, 19 profile tests, two public source examples, two compile-fail evidence-construction/deserialization tests, documentation and dependency isolation. Tests require OpenSSL CLI to generate ephemeral RSA keys; no private keys, bearer tokens or client credentials are printed or persisted. HTTP fixtures bind `127.0.0.1` only. Production code does not invoke OpenSSL CLI.

Maintained JWT selection is jsonwebtoken 11.1.0 with `aws_lc_rs` and `use_pem`; introspection uses reqwest 0.13.5 with `blocking`, `form`, `rustls`. Both disable default features. AWS-LC also serves the TLS path. This replaces the prototype's RustCrypto RSA dependency, which carries [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html); the described private-key timing exposure was not a claim of a public-key verification exploit. Rustls 0.23.45 satisfies [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285.html)'s patched floor. Versions/features and the whole lockfile were tested on Rust 1.99.0; the package inherits the workspace MIT license and remains unpublished alpha software.

The full workspace lockfile passed `cargo-audit audit --deny warnings --no-fetch` against RustSec commit `117edb3bed98e9be112f277b7615eea3252e7c43` (2026-10-02T10:58:33+02:00), scanning 208 dependencies. This is an advisory snapshot, not a cryptographic audit.
