# Astral Plane installed Studio adoption

Date: 2026-10-08. This report records an isolated consumer trial. It does not admit ROM 0.1.0 for release.

## Source and installation

The original application commit is `86dca6fa514895f94fd09ea788ebf07fa21cf648`.
The trial worktree is `/root/ROM/.worktrees/astral-010-public-adoption`.
The original application and production deployment remain unchanged.

The trial installs the current Studio source package with `npm ci --install-links`.
The installation is a regular directory, not a source symlink.
Its package manifest, sources, MIT license and third-party notices match the staged package byte-for-byte.

Vite and TypeScript no longer route ROM imports to `vendor/rom` or `$lib` aliases.
The consumer imports authentication through the public `rom-studio/auth` entry.
Tailwind scans the installed package for its classes.

The source and command witness is `.superpowers/rom-public-adoption/session-acceptance.json` in the trial worktree.
It contains seven selected consumer input hashes, the complete installed package source hashes, result hashes and browser outcomes.
The witness sets `release_admitted` to `false`.

## Development-server failure

The first browser run failed before its assertions because the development server stopped.
Vite 8.3.2 and `@sveltejs/vite-plugin-svelte` 7.3.1 attempted to prebundle a TypeScript Svelte module.
The optimizer passed its TypeScript declaration directly to `svelte.compileModule`, which rejected the declaration.
The production build had already passed; that result did not establish development-server support.

The consumer now uses `optimizeDeps: { exclude: ["rom-studio"] }`.
This keeps source modules on the TypeScript-aware transform path without changing the installed package.
The plugin documents this per-library exclusion in its public options.
See the [Svelte Vite plugin](https://github.com/sveltejs/vite-plugin-svelte).

The corrected development server returned HTTP 200 before the repeated browser run.
The original connection-refused log remains in `session-denied-red.log`.

## Session-denial regression

Four tests reproduced private workspace retention after a confirmed HTTP 401 or 403.
Each status failed in both Chromium and WebKit at the expected public-login assertion.
The application handled `SessionExpiredError` but did not handle `SessionDeniedError`.

The application now clears the workspace for either public error type.
Transient transport failures still preserve the current view until the session recovers.
No new provider-specific authentication controller was added.

| Executed check | Result |
| --- | --- |
| HTTP 401/403 before the consumer correction | Four expected assertion failures |
| Complete session/navigation suite after the correction | 18 passed: nine Chromium, nine WebKit |
| Chat, reactive selection and Studio access | 26 passed: 13 Chromium, 13 WebKit |
| Consumer `npm run check` | 0 errors, 0 warnings |
| Consumer `npm run test:unit` | 77 passed |
| Consumer `npm run build` | Passed |

The session suite also covers expiry, overlapping checks, session renewal, view preservation and duplicate live snapshots.
The production build retains a large-chunk warning and an ineffective dynamic-import warning in the application's knowledge module.
These warnings were not suppressed.

## Remaining acceptance

The browser tests use fixture authentication and API responses. They do not prove a production provider session.
The additional 26 browser checks passed with unchanged selected consumer inputs and installed package sources.
Their separate witness is `.superpowers/rom-public-adoption/workflow-acceptance.json` in the trial worktree.
An initial combined run also included `recovery.spec.ts`, which requires a real application backend and login provider.
It reached the login form wait without that infrastructure and timed out before testing recovery.
The owned run was stopped after this cause was confirmed; its logs and traces remain preserved.
These cases need the actual packaged application host. Fixture results cannot replace them.
The frontend worktree retains its original Rust vendor paths. The separate package trial below does not use those ROM paths.

## Actual Rust package trial

The separate trial completed on 2026-10-08.
It uses nine fresh ROM crate archives and a private copy of the actual application.
The archive payloads match the selected producer sources. Their extracted inventories remain unchanged after execution.
The application resolves ROM dependencies to these extracted packages, not its vendor tree.
Its Astrorust dependency remains the original application dependency. No Astrorust source correction was made.

The package capsule is `/root/ROM/.superpowers/rom-010-astral-packages-US3g7u`.
The successful execution evidence is `adoption-evidence-retry-1` within that capsule.
The initial offline resolution failed because the local registry cache lacked `chrono-tz`.
That failure and its final input audit remain preserved in `adoption-evidence`.
A separate, bounded dependency fetch populated the cache before the successful offline retry.

| Actual application check | Result |
| --- | --- |
| `cargo test --offline --locked --all-targets` | 184 passed, zero failed, one ignored |
| `cargo clippy --offline --locked --all-targets -- -D warnings` | Passed |
| `cargo build --offline --locked --bin astral-plane` | Passed |
| Final dependency audit | No registry drift; ROM packages resolve to the extracted inventories |
| Command process groups | Absent after every completed command |

Clippy succeeded with warnings denied for the application targets.
Its log retains existing warnings from the Astrorust dependency. This is not a warning-free dependency graph.

The built executable contains 359,280,904 bytes.
Its independently read SHA-256 is `6dc6d80f4f5423e3477541ad3e6be122f5d5e06070063d053c8aad7e5eff707b`.
The source application, extracted packages and resolved lock are covered by the final input audit.
The original application deployment remains unchanged.

| Evidence file within the capsule | SHA-256 |
| --- | --- |
| `archive-extraction-witness.json` | `d31485822b21e1dca502679c5d447c1ce00f15155341ff297d5715fddd2704b5` |
| `adoption-evidence-retry-1/acceptance.json` | `1c052a7b7f3d63e2316f2ec93478a131735036c66d77f3ee31dc48fd0c576d3c` |
| `adoption-evidence-retry-1/final-input-audit.json` | `91ad1a21478777e61b0cbf5da6d414ac04329670c60cec65edfdc4222e8a7c7c` |

These are dirty-source candidate archives with the current `0.0.3` package versions.
They are not clean-source ROM 0.1.0 release artifacts.
The successful build does not prove an actual host session, provider login or browser recovery against that executable.
The browser fixture results above and the native results establish separate acceptance boundaries.
The [independent review](rom-0.1.0-astral-packaged-adoption-independent-review-2026-10-08.md) found no blocker for this native package subset.
It repeated archive, source, dependency, result and executable-hash checks without a new native build.

Final release acceptance also needs matching clean-source artifacts and the complete local verifier.
This trial must not replace those gates or claim that the original deployment uses ROM 0.1.0.

## Actual Chromium recovery

The actual packaged application passed all four original recovery cases in Chromium on 2026-10-08.
Only the original fixture's provider-specific login function changed. The recovery case bodies remained unchanged.
The trial used a fresh isolated Authentik window and the verified synthetic linked user.
Two cases committed a friend create or delete, lost the response, and repeated the identical request.
Each case verified the resulting friend count after reload. Neither operation produced a duplicate.
The conflict case injected HTTP 409; it does not establish a backend-generated conflict.
The session case injected one transport failure, then forwarded the actual authenticated session response.

The private capsule is `/root/ROM/.worktrees/astral-010-public-adoption/web/.superpowers/rom-actual-recovery-Iy8pqs`.
Its `results.json` reports four expected passes, zero skipped, zero unexpected and zero flaky cases.
The recorded browser duration is 46,367.74 ms. `browser.result.json` records exit zero and absent process group.
`execution.json` confirms unchanged inputs, an absent host target, a vacant application endpoint and successful callback restoration.
The final artifact inventory before the execution record contains 121,120 bytes.

The provider identifier is `535112ef70cb20ae996b780cc9f1c655`.
Its terminal witness confirms all three containers stopped, unchanged fixture sources and an empty cgroup.
The caller separately verified all five allocated provider ports were vacant.
The provider evidence is beneath `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/`.

This is loopback HTTP acceptance with dirty-source candidate packages. Trusted TLS and clean-source release admission remain separate gates.
The fresh WebKit 2359 execution passed the same four original recovery cases.
Its capsule is `/root/ROM/.worktrees/astral-010-public-adoption/web/.superpowers/rom-actual-recovery-XnwOYb`.
The browser duration is 44,567.45 ms. The result has zero skipped, unexpected or flaky cases.
The command exited zero with an absent process group and no truncated output.
Its execution witness confirms exact callback restoration, unchanged inputs, absent host and vacant application endpoint.
The artifact inventory before its execution record contains 121,096 bytes.
The fresh provider identifier is `cca0ef193f90e87cdb0102ff0727fde1`.
Its terminal witness confirms all three containers stopped, unchanged fixture sources and an empty cgroup.
The caller separately verified all allocated provider ports were vacant after shutdown.
These eight passes establish the defined actual-browser recovery subset for both engines.
They do not replace TLS deployment, final-source verification or clean-source release packaging.
