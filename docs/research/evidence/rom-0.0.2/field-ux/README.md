# Field editor browser evidence

These checks ran on 2026-10-04. The browser used the maintained existing
`rom-demo` executable with SHA-256
`c93a20cfc1d92cc6e3e6f613105bbbcc61468179008fc118fafabb994d1c88f3`.
It was not built from this final source revision. The current production
frontend bundle was `studio/dist/assets/index-D-dNoHf8.js`, SHA-256
`c3ddafcafb1aef73b636f6699e251c860762aaec6ff208595921ea885cda7097`.
The frontend lock was
`44a7ef3624f657b82b69731c13c48d69a3054f9c9a18d87a97406d6d9742c7e8`.

| Log | Executed scope | Result |
| --- | --- | --- |
| [Chromium host](chromium-host.log) | Real host, SQLite and redb, generic frontend after collection-control changes | 12 passed. This run preceded the final null/absent display note. |
| [Current presence host](current-presence-host.log) | Real host, SQLite, complete Task and Inventory workflow with the final production bundle | 1 passed. |
| [WebKit host](webkit-host.log) | Real host, SQLite and redb, final production bundle | 12 passed. |
| [Demo codec host](demo-codec-host.log) | Real host, SQLite and redb, explicit `demo-ticket-code` renderer | 2 passed. This run preceded the final null/absent display note. |

The generic and demo browser assets were separate builds of the same Studio
source. The demo build registered one versioned codec renderer. It did not
enable a fallback editor for the opaque codec. Component tests covered null,
absence, false, zero, empty string, removal, and unchanged patch entries.

These runs do not satisfy release gate 8. That gate must build the native host
from the accepted source, use extracted frontend assets, and run the external
author check. The temporary VPN preview has no release-artifact identity.
