# Disposable provider-neutral authentication probe

**Throwaway executable evidence, not production login/authentication or an adopted public API.** Two bounded synthetic profiles produce one trusted actor: RS256 JWT human access tokens and RFC 7662-shaped opaque-token introspection for service identities. Both authorize ordinary Resources through a separate dependency-free core crate.

Run from the host:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/auth-provider-probe/prototypes/auth-provider-probe && ./verify'
```

`verify` requires Rust/Cargo 1.99 and OpenSSL CLI in the existing native `rom-dev` container. It runs format, Clippy with warnings denied, locked tests and a core dependency-tree check, using two build jobs and its own target directory. After the first fetch, add `CARGO_NET_OFFLINE=true` before `./verify` for a network-free Cargo rerun.

Tests generate two ephemeral 2048-bit RSA keypairs in memory through OpenSSL; private keys and bearer tokens are never printed or written to files. A temporary TCP listener binds only `127.0.0.1` on an ephemeral port. No real account, secret, external IdP or host networking change is used. Synthetic fixture client credentials in the source are intentionally not valid anywhere else.

[Profiles, results, recommendations and remaining work](../../docs/research/auth-provider-probe-results.md).
