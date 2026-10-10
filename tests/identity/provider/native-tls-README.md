# Native Host TLS fixture

This fixture tests the Linux native Host client. It does not certify macOS or Windows trust behavior.
The historical terminated-loopback fixture remains separate. Native TLS uses no `provider_backchannel`.

## Cases

Run one trusted token/JWKS control and six negative cases. Test wrong CA, wrong name and expiry for token and JWKS acquisition.
Use a fresh Host and database for each case. The fixture rejects an existing database.
Run the same cases on SQLite and redb when both adapters are in the admission scope.

The browser obtains an original provider authorization code. It retains that code and its login cookie in a private file.
The browser context and its owned process group must drain before the native callback.
For a token negative, the proxy changes its TLS context before the callback.
For a JWKS negative, the proxy forwards the original token response, changes its TLS context, and closes that native connection.
The next JWKS request must use a new TLS handshake. This prevents connection reuse from bypassing the negative control.
For token wrong CA, the proxy retains the trusted leaf. Only the fresh Host receives an unrelated private CA.
For JWKS wrong CA, the Host initially trusts the provider CA. After the token response, the proxy installs an unrelated-issuer loopback leaf.
The negative leaf must be signed by its declared `negative_ca`. It must retain current validity and the correct loopback SAN.

## Preparation

Create a private options file under `/var/tmp/rom-010-authentik-20261007/run/volume/private/native-tls-*/`.
Supply these references: `binary_identity`, `provider_ready`, `client_input`, `subject_input`, `environment`, `nss_reference`, `native_ca`, `provider_ca`, and `negative_ca`.
The client input contains the original client ID and secret. The subject input contains the verified original `subject`.
Supply `trusted_leaf` and `negative_leaf`, each with `certificate` and `key` references.
Select `adapter`, `case_route`, and `case_certificate`.
Do not place secret values in a command or public evidence.

Run preparation with this command:

```sh
node tests/identity/provider/native-tls-prepare-run.mjs /absolute/private/options.json
```

Preparation launches no process. It records current binary, native source, fixture, browser-library, CA and private input hashes.
It validates certificate signatures, validity intervals, SAN expectations and matching private keys.
It writes a fresh configuration with HTTPS token and JWKS endpoints and no private endpoint override.
The readiness event retains `tls_verified: false`. Readiness is not handshake evidence.

## Finite execution

Obtain the coordinator's independent source review and explicit execution lease before this command.
The lease contains `schema: rom-native-tls-finite-lease-v1`, the exact `preparation_sha256`, and `expires_unix_ms`.
The lease must leave 210 seconds and expire within 300 seconds.
The provider readiness witness must also leave 210 seconds.

```sh
node tests/identity/provider/native-tls-run.mjs /absolute/private/preparation.json /absolute/private/lease.json
```

Only fresh owned process groups enter fresh cgroups. The runner records process birth and physical drain.
The Host environment replaces inherited proxy and trust settings. It does not change system trust.
Each child has a finite deadline and a 1 MiB output limit. Proxy requests and response bytes are bounded.
The planned allocation is 1 GiB. The final inventory checks this budget; it is not an aggregate hard quota.
The runner requires 2 GiB free before admission. The coordinator must retain its shared allocation controls.
All output and failed private inputs remain available. The runner does not remove another project's files or stop the provider.

## Evidence limits

A negative result requires a native TLS connection, rejection before the selected HTTP route, no authenticated session, and no protected value.
A positive result requires original token and JWKS traffic, a linked session, CSRF binding, and an authorized protected read.
`native_tcp_connections` counts TCP acceptances. `native_tls_handshakes` counts server `secureConnection` events.
A TCP acceptance alone is not a completed TLS handshake. A JWKS negative requires an earlier trusted token handshake.
Pure Node checks test the fixture contracts. They are not native TLS acceptance.
Native compilation and all provider/browser cases remain pending until the coordinator executes the reviewed lease.
Final matching installed application acceptance, provider lifecycle, rotation, DeniedGrant, and the supported OS matrix remain separate release gates.
