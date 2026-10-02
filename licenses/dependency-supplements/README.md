# Pinned dependency notice supplements

These are unmodified upstream MIT license files for Cargo distributions that omit their repository-root license. They retain the upstream terms and attribution; ROM's MIT license does not replace them.

`manifest.json` binds every supplement to the exact package version, `.cargo_vcs_info.json` revision, immutable upstream URL and SHA-256 of the saved bytes. Files were retrieved from those URLs on 2026-10-02. JNI and JNI macros use distinct source revisions even though their license contents match. The selected MIT text is an offered alternative; the generator preserves the package's full declared SPDX expression.

The manifest also identifies additional license-bearing paths already in the downloaded `r-efi` and `ring` archives. No duplicate copies of those files are necessary. Missing paths, changed source revisions and edited supplement bytes fail generation.

`scripts/dependency-inventory.mjs` reads these local files and uses `cargo metadata --locked --offline`. It makes no network request. To update a package, independently inspect its published archive and revision, then update this evidence; do not reuse a notice merely because a crate name matches.

Run the focused collector checks with `node --test scripts/dependency-notices.test.mjs`. See `docs/research/maintained-license-review.md` for scope and artifact-specific limitations.
