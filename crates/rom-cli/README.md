# rom-cli

A generic HTTP client for ROM Resources. Builds the `rom` executable.

```sh
cargo build -p rom-cli
rom() { cargo run --quiet --locked -p rom-cli -- "$@"; }
rom --help
rom --endpoint http://127.0.0.1:8080 --auth-file /path/to/header-file discover
rom --endpoint http://127.0.0.1:8080 --auth-file /path/to/header-file --output json query tasks
```

Use explicit idempotency keys and expected revisions for mutations. Remote errors
do not prove rollback; preserve the original request for same-principal recovery.
Credentials are read from a user-managed file and never persisted by the CLI.
There is no login implementation, automatic replay, redirect following or proxy.

See `docs/cli.md` in the source repository for the complete command, stream,
limits and exit-code contracts. Run `./crates/rom-cli/verify` from the repository
for the focused formatter, Clippy and actual-binary/TCP acceptance tests.
