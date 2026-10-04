# Trusted HTTPS Studio demo profile

Date: 2026-10-04.

The demo now has an additive `studio-profile` command. The existing `studio` command still requires the approved loopback fixture issuer. Both commands use the same Runtime, Resource declarations, seed receipts, durable adapters, blob service and shutdown path.

```text
rom-demo studio-profile sqlite|redb DB PORT ASSETS ABSOLUTE_PROFILE_JSON
```

The new mode listens on `127.0.0.1`. A separately configured HTTPS proxy exposes the public origin. This command does not terminate TLS itself. It does not enable public HTTP or private fixture controls.

The profile uses these exact fields:

| Field | Meaning |
| --- | --- |
| `public_origin` | Exact public HTTPS origin |
| `issuer` | Public HTTPS OIDC issuer |
| `authorization_endpoint` | Public authorization URL |
| `token_endpoint` | Public token URL |
| `jwks_endpoint` | Public signing-key URL |
| `client_secret_file` | Absolute private credential file |
| `backchannel_token_endpoint` | Optional trusted numeric-loopback HTTP token URL |
| `backchannel_jwks_endpoint` | Optional trusted numeric-loopback HTTP signing-key URL |

Both backchannel fields must be present together. The host validates their binding to the exact public issuer. They change only server-side token and signing-key acquisition. Browser authorization and callbacks retain the public HTTPS URLs. Unknown profile fields are rejected.

This is a seeded demo profile. Its identity authority is `local`, its client ID is `studio`, and its provider label is `Local demo provider`. The identity Resources and current User checks remain active. This profile is not a general production identity provisioning tool.

The JSON file is an absolute regular file, bounded to 16 KiB. The credential file is an absolute regular file, bounded to 4096 bytes. Group and other access bits must be absent. The process owner must own the credential file. The parser rejects a direct symlink and checks the opened file metadata again. Reads are bounded. Invalid UTF-8 returns a generic error that does not contain credential bytes. One terminal newline is removed from the secret. Credentials are not printed, copied to Git or accepted as inline profile fields.

## Executed checks

The native build and Clippy passed. Three admission tests passed. They cover valid private credentials, public HTTP rejection, public credential permissions, direct symlink rejection, unknown fields, incomplete backchannel configuration, oversized credentials and redacted invalid UTF-8 errors. See [the native results](evidence/rom-0.0.2/studio-profile/native-results.log) and [the admission results](evidence/rom-0.0.2/studio-profile/admission-results.log).

An actual subprocess probe started this command on SQLite and redb. It used the shared HTTPS-issuer OIDC fixture with exact internal loopback token and signing-key URLs. Readiness reported the configured public HTTPS origin and no fixture controls. The real host returned the configured provider listing. SIGTERM exited with code 0. See [the launcher results](evidence/rom-0.0.2/studio-profile/launcher-results.log).

The launcher probe does not test a TLS proxy or human browser login through a deployed HTTPS origin. Those checks belong to deployment acceptance. Synthetic profile files, databases and logs remain in `/var/tmp/rom-studio-https-profile-*`.

## Shared file-reader correction

A later review replaced the pathname precheck and blocking open with shared opened-handle admission. The current reader uses `O_NOFOLLOW | O_NONBLOCK` and validates the opened descriptor. It preserves the Studio ownership, absolute-path and size requirements. It shares the provider profile's Linux-only boundary. See [the correction and executed checks](rom-0.0.2-secure-host-files-results.md).
