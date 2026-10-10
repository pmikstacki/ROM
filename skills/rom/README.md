# ROM author workflows

This revision contains four source skills and their executable examples for the `0.1.0` source candidate.
Release acceptance remains open. Bundle/profile revision 1 is separate from the ROM package version.
The [native contract](../../docs/native-extensions.md) is the canonical compatibility guide.
The bundle copies that guide and the maintained consumer fixture with their source paths and SHA-256 values.
Edit their maintained sources; assembled copies are distribution artifacts.

## Prerequisites

Use Linux, Node.js 22, the declared Rust toolchain, and an explicit full ROM source checkout.
Read `docs/release-support.md` in the supplied checkout for accepted releases and candidate limits.
Read `extensions/native-alpha-v1.json` for package, toolchain, format, and feature versions.
The source and Cargo.lock must match the assembled bundle's recorded identity.
The local Cargo cache must contain the locked dependencies for offline examples.
Supply the build target through `CARGO_TARGET_DIR` if a shared target is necessary.
The runners use two build jobs and disable development and test debug information.

Revision 1 requires a source checkout for every workflow.
It does not support an extracted-package root as the skills' source argument.
Separate package-consumer tests verify the distributed Rust interfaces.
The bundle contains instructions and assets, rather than compiled ROM or every release artifact.

## Assemble and execute

From the supplied checkout, create a new output directory with:

```sh
node scripts/skills/assemble.mjs "$BUNDLE_DIR"
```

Select `resource`, `native`, `operator`, or `release` as `WORKFLOW`.
Run admission before executing an example:

```sh
node "$BUNDLE_DIR/tools/verify.mjs" "$BUNDLE_DIR" "$ROM_ROOT" "$WORKFLOW"
```

This command performs preflight only.
It rejects incompatible profiles/features, changed source, missing or changed assets, and broken local links.
To execute the selected example, add `--run`:

```sh
node "$BUNDLE_DIR/tools/verify.mjs" "$BUNDLE_DIR" "$ROM_ROOT" "$WORKFLOW" --run
```

The verifier prints the retained evidence directory.
Rust examples use a separate Cargo workspace and copy the supplied lockfile before offline resolution.
Evidence identifies the resulting example lockfile separately from the source lockfile.
Trusted commands own a Linux process group with finite deadlines and output bounds.
The runner terminates that group on timeout or output overflow.

## Evidence limits

The four pre-skill agent baselines passed after recorded author and environment corrections.
They do not prove that these instructions improve productivity or establish human usability.
Held-out author evaluation remains separate from executable asset tests.
The release example runs selected checks; full release acceptance follows the release skill's complete procedure.
