# ROM 0.0.2 actual Studio host trials

Status: the scoped Studio integration trials passed. These trials use the Rust host and two actual persistence adapters.

## Test boundary

The launcher registers Task, InventoryItem, and a held-out MaintenanceTicket through the same runtime.
It also registers User, IdentityProvider, IdentityLink, and StudioSettings as Resources.
SQLite and redb use the same declarations, browser controls, and transport routes.
The application has no Resource-specific API endpoints.

The provider fixture uses oidc-provider 9.12.2. A human selects a declared account and approves consent.
The browser follows the real authorization-code flow with PKCE. The fixture is not a production identity provider.
The host binds numeric loopback. Trusted test controls use a private Unix socket in a mode-0700 directory.
These controls are not browser capabilities or HTTP endpoints.

## Executed checks

The initial four-case matrix passed: Chromium and WebKit, each with SQLite and redb.
The matrix covered real login, generic discovery, queries, exact u64 values, unknown custom codecs, and logout.
It also covered a lost response after commit, receipt replay, moving pagination, three lease renewals, and a native restart.
After receipt replay, the Resource revision remained 2. The mutation did not run twice.
After restart, a new login read the committed value from the same database.
The Work page acquired real operator capabilities and listed work through the shared contract.

The browser must receive the initial authorized live response before the fixture advances its clock.
Advancing before admission tests an expired invocation, rather than an existing stream's lease renewal.
The corrected test waits for each real stream response before advancing the next lease.

The expanded four-case revocation matrix also passed. Disabling the current User removed live rows and invalidated the session.
Re-enabling the User allowed a new human login. The fixture did not retain the old browser authority.
The wrapped-codec tests are recorded separately below.

## Integration defects found

| Defect | Observation | Correction ownership |
| --- | --- | --- |
| Mount root missing | `/rom-studio/` returned 404; `index.html` returned 200. Login reached the missing root. | Host worker added exact root routing and regression checks. |
| Fixture CSP blocked callback | Chromium blocked post-consent redirection under `form-action 'self'`. | Root allowed the exact configured callback origin. No wildcard was added. |
| Wrapped custom codec identity missing | `Option<OpaqueHandle>` had a nullable shape without its leaf codec identity. | Root owns the native descriptor correction. Renderer changes follow the agreed contract. |
| Demo entry transformed too late | The demo build retained the ordinary entry. | The explicit demo Vite transform now runs before HTML entry processing. |
| Demo renderer props mismatch | Svelte diagnostics rejected `label` in RendererProps. | The extension now uses `descriptor.name`. |

## Reproduce

The gate builds the native demo with `--features studio` and locked workspace dependencies.
It rejects native source changes during the build and records the built binary hash.
The gate prepares the local pinned WebKit wrapper with `scripts/studio-browser-runtime`.

Use the already-built asset directory. This command does not rebuild or replace it:

```sh
ROM_STUDIO_CONTAINER=rom-dev ./demo/verify-studio --assets-dir /absolute/path/to/studio/dist
```

For a host with Cargo, omit `ROM_STUDIO_CONTAINER`.
The explicit container runner requires the accepted checkout under the `/root/ROM` bind mount.
The binary runs on the host and reads the exact provided host asset directory.

The package gate uses the ordinary production assets. Unknown custom codecs remain read-only there.
The separate `studio-demo` entry registers a renderer by codec identity to test a positive extension.
It does not change the ordinary Studio entry or select renderers by Resource kind.

Each run retains its database and launcher logs under `/var/tmp/rom-studio-host-browser-*`.
Playwright annotations record the run directory. Logs exclude session cookies, tokens, codes, state, and nonce values.
Native source, trial source, and previous evidence remain available.

## Wrapped codec rendering

The component trial uses the leaf codec identity with an explicit, ordered `codec_wrappers` path.
Absent or empty paths preserve the existing whole-field renderer contract.
For nonempty paths, builtin optional, nullable, list, and map controls consume each wrapper before invoking the leaf renderer.
Unknown leaf codecs disable the field's mutation selector. Existing values remain visible and unchanged.
This prevents a missing renderer from silently submitting the shape's guessed default value.

The trial passed on Chromium and WebKit. It tested optional-plus-nullable values, list elements, map entries, and read-only table values.
It also tested explicit null and omission. Submitted patches did not include the unknown codec field.
The complete component and application browser suite passed 30 tests after these changes.
A previous selector matched both `code` and `maybe_code`; an exact label selector corrected that test ambiguity.

The native fixture retains the original optional unknown codec case.
It also declares a required unknown codec, an optional registered codec, and a list of registered codecs.
Their actual-host acceptance passed with the updated native descriptor implementation.
The ordinary production assets passed four cases: two browser engines and two persistence adapters.
The explicit registered-renderer demo assets passed the same four cases.
Two additional SQLite runs asserted the committed optional and list values on Chromium and WebKit.
These are separate runs of the same contract, not ten distinct scenarios.

## Evidence

The source and built native binary hashes are in `evidence/rom-0.0.2/actual-host/`.
`production-gate.log` records the fresh-source build and four production-asset browser cases.
`registered-wrapper-browser.log` records the four registered-extension cases.
`registered-wrapper-canonical-results.log` records the two explicit canonical-value checks.
The fresh-source gate retained its complete build evidence at `/var/tmp/rom-studio-accepted-source-gpSHT3`.

This result does not cover attachment UI or process-level shutdown with simultaneous accepted authentication and upload.
Those are the next release slice.
