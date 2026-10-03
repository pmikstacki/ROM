# Pinned WebKit runtime and frontend gate

Date: 2026-10-03. The earlier missing WebKit acceptance is resolved for the component and application-fixture suites.

## Diagnosis

The downloaded WebKit revision 2359 uses the Linux ELF interpreter path. This NixOS host supplies a stub at that path. Four WPE executables require a real glibc interpreter. The upstream MiniBrowser wrapper also replaces `LD_LIBRARY_PATH`, so a parent environment change alone cannot repair the launch.

The first copied-runtime probe failed with `libharfbuzz-icu.so.0`. Subsequent probes required the JPEG 8 ABI and `libhyphen.so.0`. The repair fetched `harfbuzzFull`, `libjpeg8`, and `hyphen` from the existing pinned nixpkgs source. It did not substitute an incompatible JPEG SONAME. The previously fetched libavif, enchant, and libmanette outputs remain part of the accepted library set.

The diagnostic probe initially searched existing store library directories. The accepted runtime no longer uses that unbounded search. Its manifest lists 78 exact immutable library roots resolved from the four helper executables. These include existing host outputs and the additional pinned nixpkgs outputs. This is a tested bounded set, not a proof of the smallest possible closure or a claim that every existing output came from one nixpkgs revision.

## Reproducible preparation

Run `./scripts/studio-browser-runtime`. It verifies the downloaded WPE tree and the recorded source hashes. It copies only WPE and the Playwright launcher into a separate local directory. It patches all four copied ELF interpreters and RPATH values, and sets the wrapper's library path to the accepted roots.

The defaults use revision 2359 in the Playwright user cache and `/var/tmp/rom-studio-webkit-2359`. `ROM_PLAYWRIGHT_WEBKIT_SOURCE` and `ROM_STUDIO_WEBKIT_RUNTIME` can select separate paths. Preparation rejects overlapping paths and unrecognized existing destinations. It never deletes or modifies the downloaded browser. Repeated preparation checks the copied runtime tree before returning its executable path.

The manifest retains source hashes, the WPE tree hash, immutable Nix store paths, the glibc loader, the patchelf tool, and the nixpkgs source NAR hash. Missing immutable store roots are obtained with `nix-store --realise`. No system configuration or global library environment changes are required. The runtime copy is a local test dependency; do not include it in the release archive.

## Executed gate

Run `./scripts/studio-browser-runtime-check`. This script enables WebKit explicitly, then runs diagnostics, SDK tests, application/auth tests, the production build, and both browser projects.

The retained [evidence](evidence/rom-0.0.2/webkit-runtime) records:

- 29 SDK tests passed.
- 12 application/auth tests passed.
- Svelte and TypeScript diagnostics: zero errors and warnings.
- Production build passed.
- 20 browser tests passed: 10 in Chromium and 10 in actual WebKit.
- Repeated runtime preparation returned the accepted executable.
- An unrecognized destination was rejected and its protected marker remained intact.
- All recorded original source hashes remained unchanged.

Browser tests include exact integer and field-presence edits, custom renderers, nested validation, enum/reference/map controls, opaque inputs, keyboard interaction, accessibility, two-kind application composition, logout clearing, and unavailable-session behavior.

These application tests use explicit intercepted transport fixtures with the real SDK. Actual Rust host, human OIDC, restart, lease, and authorization acceptance remain a separate gate. This report does not convert fixture tests into real-host evidence.
