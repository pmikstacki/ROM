# ROM 0.0.3 release completion

## Accepted source and distribution

The completed local release uses source revision `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Its Git tree is `7c35772c89aa5e78ca70b040adb58293236215a5`.
The [manifest](evidence/rom-0.0.3/release-584c01b/manifest.json) records the clean source, locks, archives, assets, and all eight gates.
The [artifact verifier](evidence/rom-0.0.3/release-584c01b/artifact-verification.json) independently reconstructs the accepted distribution.
The [independent audit](evidence/rom-0.0.3/release-584c01b/independent-review.md) maps the release requirements to evidence.

The local distribution is `dist/release-0.0.3-584c01b-retry`, relative to the repository root.
It contains source, skills, and production Studio archives. It is not a registry or native binary publication.
The source tag is `v0.0.3`. GitHub hosts source only. Actions and registry publication remain disabled.
Later documentation records completion without changing the accepted artifact's source identity.

| Acceptance | Executed result |
| --- | --- |
| Complete producer | All eight fixed gates passed |
| Full local verifier and optimized build | Passed |
| Demo, provider, AI skills, and extracted Rust packages | Passed |
| Studio unit tests and typecheck | 146 passes; zero Svelte errors or warnings |
| Component browser tests | 221 passes; one explicit WebKit physical-touch skip |
| Extracted standard Studio with actual host | 28 passes across SQLite/redb and Chromium/WebKit |
| Independent extracted SDK author | Four browser cases passed, with offline install, typecheck, build, and provider setup |
| Explicit application bundle | Fresh 28-case acceptance passed with the corrected immutable optimized binary |
| Restart provisioning regression | Eight focused cases passed; edited and deleted Resources remain unchanged |
| Permanent VPN preview | Authenticated acceptance passed before and after activation, and after full container restart |

## Completed scope

Studio uses a shared right inspector for Filters, Resource details, Work, Settings, and Attachments.
The right-edge chevron opens or closes it. Quick filters retain their popover.
Desktop and narrow layouts share compact controls, labels, icons, and preserved drafts.
Work presents current authorized state and recovery controls. It does not fabricate historical events.
Plugin Settings are ordinary Resources. They reuse authorized discovery, forms, revisions, and mutation paths.
Human titles remain separate from exact identities.

Ten standard semantic codecs cover dates, times, timestamps, colors, email, URLs, multiline text, JSON, decimals, and units.
Enums, ordered multi-choice, sortable lists, maps, nested objects, references, and Blob controls share the Resource contracts.
Backend validation remains authoritative. Unsupported custom versions preserve values and block unsafe edits.
The application entry explicitly registers `demo-ticket-code` version one. The standard SDK does not load application extensions implicitly.

The Resource derive generates presentation bindings. Manual declarations use the same registration checks.
The module and DRY pass preserves public API paths. Verified skills and examples document the same boundaries.
The [README screenshots](../../README.md#rom-studio) show implemented screens with synthetic data and recorded capture identities.

## Restart and permanent preview

The immutable application binary SHA-256 is `ddeda5f8674333da679703674d679173553fad6294b0a559b7c22b8525109c2b`.
The persistent release directory is `/var/lib/rom-studio-preview/site/releases/rom-0.0.3-584c01b`.
The [application receipt](evidence/rom-0.0.3/release-584c01b/application-acceptance.json) identifies the newly executed custom bundle, native binary, and all browser cases.
The [live preview receipt](evidence/rom-0.0.3/release-584c01b/preview-acceptance.json) records actual gateway acceptance and exact baseline preservation.
The [attachment receipt](evidence/rom-0.0.3/release-584c01b/attachment-preservation.json) records path and byte preservation.

All 14 baseline authorized Resources retained their values and revisions after activation and full container restart.
The existing attachment retained its bytes. Both application and provider units are active and enabled.
Trusted-CA HTTPS serves the accepted index and neutral favicon. Unauthenticated discovery returns HTTP 401.
The previous releases and separate stopped-writer database/attachment backups remain available.
No host networking, WireGuard, SSH, Caddy route, or DNS change was required.

The secondary preview also restarts with its original database and an exact read-only profile copy.
Its [preservation receipt](evidence/rom-0.0.3/release-584c01b/secondary-preservation.json) confirms that existing Rows and the tombstone remain unchanged.
Only the absent showcase is provisioned through an ordinary action. A second restart preserves table counts and hashes.
The [startup investigation](rom-0.0.3-startup-preservation.md) explains the correction and the retained RED/GREEN evidence.
No deleted Resource is resurrected. Core disclosure, receipt replay, and storage integrity rules remain unchanged.

## Failed evidence and support limits

The earlier [c66c470 candidate](rom-0.0.3-candidate-c66c470.md) remains preserved as historical acceptance.
The later deleted-seed finding required the corrected release above.
The first corrected producer failed one WebKit case: the native WPE web process terminated with SIGSEGV on its compositor thread.
An unchanged focused repetition passed four cases and failed one with the same native crash site.
The [bounded diagnosis](evidence/rom-0.0.3/release-584c01b/webkit-diagnosis.md) retains this evidence.
The subsequent complete producer passed without a source change, skipped assertion, or increased deadline.
The first explicit application run also failed both WebKit semantic cases, with 26 of 28 cases passing.
The exact native crash offset matches the [upstream Playwright report](https://github.com/microsoft/playwright/issues/42637).
[WebKit PR 73621](https://github.com/WebKit/WebKit/pull/73621) adds the missing SkImageFilter null check.
[Playwright PR 42748](https://github.com/microsoft/playwright/pull/42748) includes this fix in WPE revision 2364.
These sources strongly support the upstream defect as the local cause. Stripped local stacks alone did not identify its function.
The isolated corrected runtime passed ten repeated semantic cases and the full explicit application suite.
The application, native binary, assertions, deadlines, and locked Playwright 1.63.0 client remained unchanged.
The producer passed with its original runtime; the additional application acceptance used WPE 2364.
These finite passes do not establish all browser protocol features or the universal absence of native crashes.

ROM 0.0.3 remains experimental. Tests do not establish production readiness or human usability.
The physical-touch WebKit case is explicitly skipped. Other keyboard, mobile-focus, and Chromium-touch checks have separate evidence.
The demo provider keeps signing keys and grants in memory. Its restart requires a fresh login.
Trusted curl verifies the private CA. The browser probe bypasses TLS errors only for the known internal preview origin.
Host-side VPN acceptance does not establish remote macOS/iOS ingress or their CA trust.

Moving pagination, bounded relation lookup, exact JSON source editing, and lexical decimal ordering retain their documented limits.
Native storage format eight, archive format six, and query protocol version one remain unchanged.
Read the [compatibility review](rom-0.0.3-compatibility.md) before recompiling extensions.
