# ROM 0.0.2 preview restart results

Date: 2026-10-05.

## Result

The preview recovered after a Caddy restart and a full `rom-studio` container restart.
Both the Studio and provider services started inside the container.
The Studio route and the provider discovery route returned HTTP 200 after startup.

The first provider request raced its service startup and returned HTTP 502.
Caddy recorded `connect: connection refused` for `127.0.0.1:44174`.
The provider then started and returned HTTP 200.

Both preview routes now set `lb_try_duration 5s`.
Caddy documents this setting for retries while an upstream is unavailable, including restarts.
The setting retries connection failures; it does not retry application errors after a connection succeeds.

## Executed checks

- Caddy 2.8.4 `caddy validate` accepted the active preview configuration.
- `systemctl reload caddy.service` kept the Studio and provider routes at HTTP 200.
- `systemctl restart caddy.service` kept both routes at HTTP 200.
- `systemctl restart container@rom-studio.service` started both in-container services.
- A controlled provider interruption returned HTTP 200 after its delayed restart in 1.259 seconds.
- The unprivileged profile test suite passed five tests as `nobody`.

The route checks used the host's internal CA root and these HTTPS URLs:

```text
https://10.66.0.2/rom-studio/
https://10.66.0.2/rom-studio-provider/.well-known/openid-configuration
```

The active preview binary SHA-256 was `81599303a927451874908fe571ed87304431b3c77f547a1ef4da21010655cd7a`.
The active Caddy configuration is `/var/lib/rom-studio-preview/caddy/active.caddyfile`.

The provider uses in-memory grants and signing keys for this demonstration.
A provider restart requires a new login. This test does not establish production identity-provider durability.
The local Caddy check does not verify access from a remote Mac or iPhone.

See the [Caddy reverse proxy documentation](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) for `lb_try_duration` behavior.
