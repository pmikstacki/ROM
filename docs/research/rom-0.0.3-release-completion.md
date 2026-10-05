# ROM 0.0.3 candidate acceptance record

## Accepted source and artifacts

The complete producer passed all eight fixed gates on 2026-10-05.
Its clean source revision is c66c470eb894b8449f9ab707d94179c88af09113.
Its Git tree is f7503bfd37571376c2d54bdd48053c3394ab305a.
The [manifest](evidence/rom-0.0.3/release-c66c470/manifest.json) identifies every gate, archive, source file, lock, and production asset.
The [separate artifact verifier](evidence/rom-0.0.3/release-c66c470/artifact-verification.json) accepted the exact distribution.

The local distribution is in dist/release-0.0.3-c66c470, relative to the repository root.
It contains source, skills, and Studio archives. It is not a registry publication or a native binary distribution.
GitHub remains the source host. GitHub Actions and registry publication remain disabled.
The accepted source remains fixed when later documentation records its completion.

| Gate | Executed result |
| --- | --- |
| Full local verifier | Passed |
| Optimized workspace build | Passed |
| Demo and provider verification | Passed |
| AI skills examples and source admission | Passed |
| Extracted Rust packages and public consumers | Passed |
| Studio component/browser gate | 221 passes; one explicit WebKit physical-touch skip |
| Extracted production assets with actual host | 28 passes across SQLite/redb and Chromium/WebKit |
| Independent extracted Studio author | Offline install, typecheck, build, provider install, and four browser cases passed |

The producer's eighth gate includes the independent author workflow.
Its inline custom renderer uses the public SDK. Canonical values, revision changes, and ordinary Task mutations remain verified.
The [earlier selector correction](rom-0.0.3-external-author-ergonomics.md) retains failed cases and the reason for the correction.
Failed producer stages remain preserved. They are not accepted artifacts.

## Completed release scope

Studio uses one shared right inspector for Filters, Resource details, Work, Settings, and Attachments.
Quick filters retain their popover. The right-edge chevron controls the inspector.
Desktop and narrow layouts share generic controls and readable labels.
Work displays the current authorized state and recovery controls. It does not invent a historical timeline.
Settings edits ordinary Resources, including settings groups supplied by plugins.

The ten standard semantic codecs cover dates, times, timestamps, colors, email, URLs, multiline text, JSON, decimals, and units.
Enum labels, ordered multi-choice, lists, maps, nested objects, relations, and Blob controls use shared contracts.
The backend validates encoded values. Presentation metadata does not change identity or authority.
Unknown codecs preserve existing values and prevent unsafe edits.
Custom renderers register their exact codec and version in the application entry point.

The Resource derive supplies presentation metadata. Manual implementations use the same registration checks.
Focused module and DRY reviews preserve public API paths.
AI skills, instructions, and executable examples describe the same public boundaries.
The [README screenshots](../../README.md#rom-studio) show implemented screens with synthetic data.

## Permanent preview acceptance

The persistent preview serves https://10.66.0.2/rom-studio/.
Its immutable release directory is /var/lib/rom-studio-preview/site/releases/rom-0.0.3-c66c470eb894.
The application entry explicitly registers demo-ticket-code version one.
The standard SDK bundle does not implicitly register application codecs.

The [application identity bridge](evidence/rom-0.0.3/release-c66c470/application-identity-bridge.json) verifies unchanged production inputs, assets, and native binary.
It references an earlier executed 28-case application acceptance. It does not claim another execution at the current commit.
Fresh complete producer acceptance separately ran all mandatory gates against c66c470.

The [live preview record](evidence/rom-0.0.3/release-c66c470/preview-acceptance.json) records fresh OIDC login before and after activation.
It repeats acceptance after a complete container restart.
All 13 preexisting authorized Resources retained their values and revisions.
The new Field showcase seed adds one row and one kind: the accepted preview has 14 rows across ten kinds.
The checks cover semantic controls, custom editing, plugin Settings, Work's right chevron, and the neutral favicon.
They perform no domain mutations.

The [attachment check](evidence/rom-0.0.3/release-c66c470/attachment-preservation.json) confirms that all preexisting attachment paths and bytes remain present.
Private Resource identifiers, attachment paths, configurations, and credentials remain outside the public evidence.
Both SQLite databases and their matching attachments have separate stopped-writer backups.
The previous release and rollback selector remain available.

After restart, application and provider units are active and enabled.
Trusted-CA HTTPS returns the accepted index and favicon. Unauthenticated discovery returns HTTP 401.
The [identity log](evidence/rom-0.0.3/release-c66c470/preview-postrestart-identity.log) records the exact binary, assets, and NixOS closure.
No host network, WireGuard, SSH, Caddy route, or DNS change was required.

## Support limits

ROM 0.0.3 remains experimental. Automated tests do not establish production readiness or human usability.
The physical-touch WebKit case is explicitly skipped; keyboard, mobile focus, and Chromium touch have separate coverage.
The demo identity provider retains grants and signing keys in memory. Restart requires a new login.
Trusted curl verifies the private CA. The browser probe bypasses TLS errors only for the known internal preview origin.
Host-side VPN acceptance does not establish remote macOS/iOS ingress or their CA trust.

Moving pagination, bounded relation lookup, exact JSON source editing, and lexical decimal ordering retain their documented limits.
Native storage format eight, archive format six, and query protocol version one remain unchanged.
Read the [compatibility review](rom-0.0.3-compatibility.md) before recompiling extensions.

## Later restart-after-delete finding

The old isolated preview could not restart after an ordinary deletion of a seeded Task.
The demo bootstrap replayed the retained create receipt. Current disclosure correctly rejected its deleted result.
Both old and current binaries reproduced the failure on private database copies. Fresh databases started.
Original user data and profiles remain intact, with separate stopped-writer backups.
The healthy persistent preview remains active. It does not contain that deleted seeded Task.
Final release completion now requires a generic bootstrap correction, actual database regressions, and repeated complete acceptance.
Do not restore deleted values or weaken the core disclosure check.
