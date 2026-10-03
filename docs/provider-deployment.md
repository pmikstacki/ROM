# Provider deployment profile

This opt-in reference host connects ROM to an OAuth service provider through the existing introspection adapter.
The complete acceptance command passed on SQLite and redb. The [results report](research/provider-deployment-results.md) records the executed checks and support limits.

The host uses the same Resource pipeline for Tasks, Users, IdentityProviders, and IdentityLinks.
The core has no provider client, secret-file reader, or HTTP dependency.
The default synthetic demo remains separate from this profile.

## Supported boundary

| Component | Profile |
| --- | --- |
| Identity | One explicitly configured service subject, provider, and local User |
| Provider fixture | `oidc-provider` 9.12.2, opaque client-credentials tokens, exact npm lockfile |
| Persistence | SQLite or redb through the same public contracts |
| Host transport | HTTP on numeric loopback; `rom-http` with async authentication |
| Secrets | Approved versioned references to private Linux files |
| Provisioning | Explicit local commands; no public enrollment endpoint |
| Authorization | Current identity bindings and declared Task permissions; operator access denied |
| Provider restart | Fixture tokens are lost; obtain a new token explicitly |

This profile does not establish arbitrary OAuth compatibility, human login, persistent provider storage, or a production TLS deployment.
The provider fixture and npm dependencies are development tools. They are not dependencies of the Rust core.

## Run acceptance

Use Linux with the repository's Rust toolchain, Node.js, and npm.
From the repository root, execute:

```sh
./demo/verify-provider
```

The command builds the opt-in host and the actual `rom` CLI.
It installs the provider's locked dependencies without lifecycle scripts.
It creates private temporary inputs and starts loopback processes.
The actual token-expiry case waits for the provider's 60-second lifetime.
The command stops its children and removes its temporary credentials.

This command supplements [the local quality gates](quality.md) and `./demo/verify`.
It does not publish packages or start a permanent provider service.

## Host configuration

Enable the profile when you build the reference host:

```sh
cargo build --locked -p rom-demo -p rom-cli --features rom-demo/provider-profile
```

Store the host configuration in a private UTF-8 JSON file.
Use a trusted private parent directory.
The file must be regular, must not be a symlink, and must have no group or other permission bits.
Its maximum size is 64 KiB. Duplicate and unknown fields are rejected.

This example contains references, not credentials:

```json
{
  "version": 1,
  "provider": {
    "authority": "provider",
    "issuer": "http://127.0.0.1:4100",
    "audience": "rom-api",
    "endpoint": "http://127.0.0.1:4100/token/introspection",
    "introspection_client": "rom-introspector",
    "credential_ref": "secret-v1"
  },
  "user": { "id": "service-user", "display_name": "Service" },
  "service_subject": "rom-service",
  "secrets": {
    "secret-v1": "private/introspection-v1",
    "secret-v2": "private/introspection-v2"
  },
  "endpoint_policy": "loopback-test-only",
  "auth": { "jobs": 4, "response_timeout_ms": 2000 }
}
```

Use the exact issuer and endpoint from your approved provider configuration.
The sample port is not reserved by ROM.
Relative secret paths resolve from the configuration directory.
The map permits at most eight approved references.
The default endpoint policy is `https-only`; the example explicitly permits numeric loopback HTTP for tests.

Each secret file contains between 1 and 4,096 UTF-8 bytes.
File acquisition preserves the bytes, including a final newline.
The host validates the opened file handle and rejects public permissions, symlinks, directories, and FIFOs.
Linux acquisition uses `O_NOFOLLOW | O_NONBLOCK`.
The profile assumes trusted parent directories and immutable versioned secret files.
It does not claim protection from privileged concurrent file changes or memory zeroization.

## Provision and serve

Provision the three identity Resources explicitly:

```sh
rom-demo provider-provision sqlite DATABASE CONFIG_FILE
```

Use `redb` instead of `sqlite` for the other native adapter.
Provisioning commits the provider, User, and canonical IdentityLink separately.
Each step uses a stable idempotency identity.
If the process stops between steps, repeat the exact command and configuration.
Changed input under an accepted step identity is rejected.

Start the host:

```sh
rom-demo provider-serve sqlite DATABASE CONFIG_FILE 0
```

Port `0` selects a free loopback port. The readiness line is JSON with an `endpoint` field.
Serving does not provision missing identities.
It excludes the provisioner allowance and never accepts the synthetic `Demo local` header.

Acquire a token through the approved provider's token endpoint.
For the development fixture, use [its separate commands](../demo/provider-fixture/README.md).
Store the complete `Bearer` header in a private file.
Then use the generic CLI:

```sh
rom --endpoint http://127.0.0.1:PORT --auth-file AUTH_FILE discover tasks
rom --endpoint http://127.0.0.1:PORT --auth-file AUTH_FILE query tasks
```

The linked service can use only this profile's declared Task operations.
Its token does not grant operator work controls or identity maintenance.
Consult [the CLI contract](cli.md) before retrying a mutation with an uncertain result.

Send `SIGINT` to stop the reference host.
The stop path first closes authentication admission and drains accepted authentication jobs.
It keeps Runtime open during that drain, then lets HTTP close and drain Runtime.

## Maintain identity Resources

Stop the host before offline maintenance.
Prepare a private JSON invocation file with an explicit revision and idempotency key:

```json
{
  "kind": "identity-providers",
  "id": "provider",
  "expected": 1,
  "idempotency": "rotate-v2",
  "operation": {
    "type": "patch",
    "input": { "credential_ref": { "op": "set", "value": "secret-v2" } }
  }
}
```

Execute the local maintenance command:

```sh
rom-demo provider-maintain sqlite DATABASE CONFIG_FILE INVOCATION_FILE
```

The command accepts only the configured provider, User, or canonical link key.
It permits revision-checked replacement, patch, or deletion; it does not expose arbitrary actions or creation.
The input limit is 16 KiB.
Native ownership prevents maintenance from competing with an active host on the same database.

The maintainer is an explicit embedded host identity.
Network authentication never returns it.
Host-local control code validates the exact configured key before it invokes the shared Resource pipeline.

## Rotate credentials and recover

Create a new private file for the replacement introspection credential.
Add its opaque reference to the host's approved map.
Coordinate credential replacement with the provider's documented mechanism.
Update the provider Resource through explicit maintenance at its current revision.
Restart the host with the approved configuration.

Serving reads the current Resource's credential reference.
The configuration's initial reference is used for provisioning; it does not overwrite a later Resource change.
An unknown reference, missing file, or rejected replacement credential denies authentication.
There is no automatic fallback to the old credential.

The fixture replaces its introspection credential through a provider restart.
That restart also removes its opaque tokens.
Obtain a fresh token after the restart.
A ROM-only restart preserves accepted Resource changes and receipts while the provider remains available.

Disabling a provider, link, or User invalidates its identity binding.
Re-enabling it changes the revision and does not revive an old Actor stamp.
A new authentication must bind against current state.
Provider changes during verification also prevent stale binding.

## Authentication lifecycle

The example defaults to four admitted authentication jobs and a two-second caller deadline.
Overload does not enqueue unbounded work or automatically retry a provider request.
A caller timeout or disconnect does not terminate the host-owned job.
Its permit remains occupied until verification and binding finish.
The blocking verifier has its own finite network limits.

The host constructs, uses, and drops the blocking client inside a blocking worker.
Provider I/O does not hold the core commit gate.
Close prevents new admission. Drain waits for accepted work and can be awaited again after a waiter is canceled.
The host and Runtime must share the same clock.

Credentials do not belong in Resource values, receipts, events, archives, process arguments, or logs.
Credential references are configuration identifiers, not secret bytes.
The [provider selection report](research/real-provider-profile-selection.md) and [probe report](research/real-provider-profile-probe.md) describe the earlier interoperability evidence.
