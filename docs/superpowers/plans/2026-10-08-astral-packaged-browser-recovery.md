# Actual packaged Astral browser recovery

Date: 2026-10-08. Status: actual Chromium and WebKit recovery passed; final release admission remains open.

## Acceptance boundary

Use the actual Astral binary built from extracted ROM candidate packages.
Serve its installed-package frontend through that binary. Use the existing, isolated Authentik provider.
Keep the original application and its deployment unchanged.
This trial uses the supported loopback HTTP profile. It does not establish TLS or production deployment acceptance.

Preserve the original four recovery cases. Replace only their Dex-specific login with actual Authentik login.
Verify the callback, linked user and session cookie before each case.
Two cases lose a response after an actual mutation commit and repeat the identical request.
One case injects a definitive HTTP 409; this does not prove an actual backend conflict.
The fourth case injects a session transport failure, then forwards the actual session response.

## Evidence and preparation

The [package adoption report](../../research/rom-0.1.0-astral-installed-adoption-2026-10-08.md) binds the executable and source packages.
The [preparation review](../../research/rom-0.1.0-astral-browser-preparation-review-2026-10-08.md) covers private inputs and preserved cases.
`tests/astral-adoption/prepare-browser.mjs` produces a fresh private nonce directory for each browser engine.
It verifies the previously accepted synthetic identity input before reading its stable subject.
It records the complete frontend asset inventory and original/generated recovery hashes.
Credentials and host configuration remain private, with file mode `0600` and directory mode `0700`.

The first prepared Chromium capsule is:
`/root/ROM/.worktrees/astral-010-public-adoption/web/.superpowers/rom-actual-recovery-lowXAY`.
Its actual host smoke test returned HTTP 200 for HTML and the initial unauthenticated session.
The listener matched the launched process identity. Owned shutdown completed, and the target process was absent.
This smoke test did not perform provider login or browser recovery.

## Caller requirements

1. Confirm that the allocated provider and application ports are vacant before launch.
2. Start a fresh, exclusively owned provider window. Do not extend a running window.
3. Check the actual remaining window before admission. Refuse an insufficient window before configuration changes.
4. Admit at least 330 seconds for a 240-second browser command, setup and finite cleanup.
5. Use the existing finite long-window profile when the default profile cannot meet this requirement.
6. Verify the actual provider and application listener ownership before browser traffic.
7. Append only the exact application callback. Confirm its readback before login.
8. Restore the exact original callback list on success, failure and unknown PATCH outcomes.
9. Stop and drain only the launched process groups. Verify their absence and the released listeners.
10. Compare executable, assets, inputs and generated cases after execution. Preserve every failure.

The callback helper has 12 maintained passing cases, including failed activation and failed restoration.
Its initial absent-module failure is setup evidence, not a reproduced production defect.
Concurrent provider writers and abrupt parent termination are outside this helper's guarantees.
The process owner must supply finite deadlines and cleanup supervision.

## First provider window

The default window reached readiness with identifier `dc4b956ed68cf3c5ae494d5de78b036b`.
Before browser admission, only 245,774 ms remained against the then-declared 285,000 ms requirement.
The caller refused admission and requested stop through the exact owned stop-file identity.
All three provider containers stopped, and the launcher exited zero.
No callback or provider configuration changed. No browser recovery result is claimed.

The next preparation requires 330,000 ms to include worst-case API and cleanup allowance.
This changes the fixture admission requirement, not a product deadline or an existing provider lifetime.

## First long-window attempt

The fresh `--load-window` attempt failed before readiness with `invalid supervision limits`.
Its identifier is `373b47c367cadfddd2824e0c617e3a43`.
The declared relay lifetime was 1,440,000 ms. The shared supervisor accepted at most 1,200,000 ms.
The launcher exited one. All three launched containers stopped, and the provider ports were vacant after cleanup.
The terminal witness confirms unchanged fixture sources.
No callback change, application launch or browser login occurred in this attempt.
The failure is preserved in `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/authentik-terminal-373b47c367cadfddd2824e0c617e3a43.json`.
The provider owner must correct and test this admission mismatch before the next fresh window.

## Completion gate

The corrected long-window provider reached readiness as `db9b07bddfe3ae27bd10bfb2b71d0146`.
The first actual consumer invocation failed before application launch.
Its strict redirect parser rejected the provider's `redirect_uri_type` field.
The [provider API](https://api.goauthentik.io/reference/providers-oauth-2-retrieve/) defines `authorization` and `logout` values for this field.
The corrective fixture accepts only those values. It preserves original entries and distinguishes login callbacks from logout callbacks.
The new regression failed before the correction. All 22 Astral runner tests passed after the correction.
The failed trial is preserved in `rom-actual-recovery-hmzXFq/execution.json` beneath the consumer's private `.superpowers` directory.
No callback changed, and no browser recovery result is claimed.
The provider launcher reported unsuccessful stop acknowledgements for server and worker.
Fresh, read-only reconciliation confirmed all three exact containers stopped before another launch.
Its witness is `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-reconciliation-db9b07bddfe3ae27bd10bfb2b71d0146-1791458787378.json`.

Require four passing cases per browser engine against the actual host and provider.
Require successful callback restoration, unchanged source witnesses and verified process cleanup.
Do not substitute fixture browser results, a successful build or the host smoke result for this gate.
Dirty-source candidate packages remain separate from final clean-source ROM 0.1.0 artifact admission.

## Executed outcome

Fresh Chromium and WebKit windows each passed the four original recovery cases.
Both executions confirmed callback restoration, unchanged inputs, absent process groups and vacant application endpoints.
Both provider windows stopped all three containers with unchanged fixture sources and empty cgroups.
The caller verified vacant provider ports after each shutdown.
The [actual adoption report](../../research/rom-0.1.0-astral-installed-adoption-2026-10-08.md) records the capsules, durations and remaining limits.
This completes the defined loopback browser subset. It does not admit the release or prove TLS deployment.
