# Development provider fixture

This fixture uses `oidc-provider` 9.12.2 with an exact npm lockfile.
It issues opaque client-credentials tokens through the provider's token endpoint.
The provider supplies introspection responses without a response translation layer.

The fixture is for local tests. It does not provide persistent provider storage, human login, or a production TLS deployment.
Its HTTP listener binds only to `127.0.0.1`.
Tokens expire after 60 seconds. A provider restart removes all tokens from its memory.
A ROM restart does not restart the provider.

## Tests

From this directory, install the exact dependencies:

```sh
npm ci --ignore-scripts
```

Run the provider and file tests:

```sh
node --test fixture.test.mjs commands.test.mjs
```

These tests cover token issuance, introspection, provider restart, private files, command output, and child-process cleanup.
They do not test the complete ROM host deployment.
The [provider probe report](../../docs/research/real-provider-profile-probe.md) records the separate Rust verifier experiment.

## Process interface

Prepare two private UTF-8 files with separate random secrets.
Each file must contain between 1 and 4,096 bytes, with no group or other permission bits.
Use a trusted private parent directory.

Write a private JSON configuration file with these fields:

| Field | Value |
| --- | --- |
| `service_secret_file` | Path to the token client's secret file |
| `introspection_secret_file` | Path to the introspection client's secret file |
| `port` | Optional port; `0` selects a free loopback port |

Start the provider:

```sh
node server.mjs CONFIG_FILE READY_FILE
```

`READY_FILE` must not exist. The process creates it with mode `0600` after the listener starts.
The file contains the issuer, introspection endpoint, audience, service subject, client name, resource indicator, and provider version.
It contains no credentials. The process rejects unknown configuration fields.

Create a CLI authentication file:

```sh
node issue.mjs READY_FILE SERVICE_SECRET_FILE AUTH_OUTPUT_FILE
```

`AUTH_OUTPUT_FILE` must not exist. The command creates it with mode `0600` and writes the complete `Bearer` header value.
The command does not print the token. It does not replace an existing file.
Pass this file to the ROM CLI through `--auth-file`.

Send `SIGTERM` or `SIGINT` to stop the provider.
Wait for process exit before the removal of its private files.
Remove the readiness file before a new start.
After a provider restart, obtain a new token.

## Profile and limits

The issuing client is `rom-service`. The separate `rom-introspector` client can inspect its tokens but has no token issuance grant.
The resource indicator is `https://rom.fixture.invalid/api`; the audience is `rom-api`.
The configured provider hook supplies `sub` and `principal_kind` for this service profile.
The provider supplies the remaining token fields.

Private file reads reject symlinks, non-regular files, public permission bits, empty files, oversized files, and invalid UTF-8.
The Linux file path uses nonblocking acquisition, so a FIFO without a writer cannot block the read.
This assumes trusted parent directories. It does not protect against a privileged process that changes file contents concurrently.

Fixture requests have a three-second deadline and a 16-KiB response limit.
Commands report static failure messages without credential details.
The test harness bounds child waits and removes its temporary inputs.

This document follows the repository's [writing rules](../../docs/writing.md), including their stated dictionary verification limits.
