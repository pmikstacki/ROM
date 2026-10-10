# Installed public AI consumer acceptance

This runner consumes a canonical producer extraction, its source archive, and exact crate archives.
It copies the maintained publication and ticket-triage examples. It does not duplicate their business logic.
It also copies the public read-only compile fixture.

No native installed-consumer run has been executed for this increment.
The Node contract tests establish parser and fence behavior only.
Browser progress, original application adoption, human ergonomics and clean release admission remain separate gates.

## Inputs

Prepare a JSON options file outside the exclusive output directory.
Use absolute canonical paths. Supply all maintained crate archives from one matching producer.
The source extraction must contain exactly the witness's files, without generated output or symlinks.

```json
{
  "sourceRoot": "/var/tmp/rom-010-ai-producer-current/source/rom-source",
  "sourceArchive": "/var/tmp/rom-010-ai-producer-current/rom-source.tar.gz",
  "sourceArchiveSha256": "REPLACE_WITH_ACTUAL_SHA256",
  "sourcePrefix": "rom-source",
  "source": {
    "source_mode": "provisional-working-source",
    "revision": "REPLACE_WITH_FULL_HEAD",
    "package_version": "0.0.3",
    "lock_sha256": "REPLACE_WITH_EXTRACTED_CARGO_LOCK_SHA256",
    "source_inventory_sha256": "REPLACE_WITH_DIGEST_OF_FILES",
    "files": []
  },
  "crateArchives": [
    {
      "path": "/var/tmp/rom-010-ai-producer-current/package/rom-ai-0.0.3.crate",
      "prefix": "rom-ai-0.0.3",
      "sha256": "REPLACE_WITH_ACTUAL_SHA256"
    }
  ],
  "output": "/var/tmp/rom-010-ai-installed-current-20261008",
  "target": "/var/tmp/rom-010-ai-installed-current-20261008-target",
  "compiler": "1.99.0",
  "metadataSeconds": 60,
  "testSeconds": 300,
  "compileSeconds": 300,
  "outputBytes": 8388608
}
```

The template is deliberately incomplete and will fail admission.
List every crate archive, not only `rom-ai`. Current package inspection also requires `rom`, `rom-sqlite`, `rom-identity` and `rom-studio-host`.
AI acceptance additionally requires `rom-ai`, `rom-openrouter` and `rom-redb`; include their complete maintained dependency set.

Each source file record is `{ "path": "relative/path", "mode": "100644", "sha256": "actual digest" }`.
Use `100755` for an executable file. Preserve the producer's file-array order.
Compute `source_inventory_sha256` with `digest(files)` from `scripts/skills/files.mjs`.
The witness's package version must match `[workspace.package]` in the extracted `Cargo.toml`.

Use `committed-release-source` only for a matching clean committed producer.
That mode also requires the Git source archive's commit identity to match `revision`.
Provisional mode verifies the archive's exact bytes and extracted file witness without claiming a clean Git artifact.
Both modes require independent root selection of the expected archive digests and source witness.

## Finite execution

Run only inside the root-coordinated native allocation:

```sh
timeout 2400s node scripts/packages/ai-flows-consumer.mjs /var/tmp/rom-010-ai-installed-options.json
```

The process helper limits each command and its captured output. It drains owned process groups after success or failure.
Every Cargo child uses offline dependencies, two jobs, disabled incremental compilation and the exclusive target directory.
The outer timeout is an additional deadline, not permission to abandon the owned process group.

The inherited process helper signals a recorded process group. An escaped descendant that holds stdio can delay its close wait.
This increment does not establish adversarial process containment or replace that shared helper.

Crate extraction accepts genuine Cargo archives without an explicit root-directory entry.
It requires exact-prefix paths, regular files or directories, and unique canonical entry names.
Links, special files, traversal and file/directory collisions are rejected before output creation.
Each crate has 128 MiB limits for compressed input, decompressed tar data, each logical file and aggregate logical size.
The logical size checks also cover GNU sparse files before extraction.
The three tar commands each have a ten-second timeout and an 8 MiB output limit.
They consume one immutable byte snapshot with a minimal environment and fixed C locale.
Source archive extraction retains its separate strict explicit-root requirement.

The runner executes `rustc -Vv`, initial and locked metadata, native tests, and final locked metadata.
Its four subjects are the copied public example, copied compile fixture, extracted `rom-ai`, and feature-enabled extracted `rom-openrouter`.
The example executes both real SQLite/redb domain journeys and their maintained recovery cases.
The unchanged compile checker requires positive read/query/action APIs and the exact negative `ReadContext.execute` diagnostic.

Initial metadata may update only the subject's Cargo.lock.
Every registry entry must match an admitted producer, packaged or original consumer lock entry.
The resolved lock is then frozen. Every resolved ROM dependency must select its exact extracted package.
Unchanged source archives, producer files, crate inputs and copied consumer inputs are checked between commands.

With the example budgets, the native command maxima total 1,980 seconds.
Archive inspection/extraction adds finite helper deadlines. The proposed outer allocation is 2,400 seconds.
Expected new debug target growth is provisionally 6–12 GiB; this estimate has not been measured.
Source/crate copies and retained command output add smaller evidence growth dependent on the actual archives.
Captured command output is bounded at 8 MiB per command. A native command can still consume its full target estimate.
The root must choose an allocation with sufficient free disk before execution.

All outputs are preserved. The runner performs no cleanup.
`acceptance.json` appears only after execution and input fences succeed.
Its `acceptance_scope` and `publication: false` prevent provisional evidence from being presented as publication admission.
Failed executions retain `failure.json`, command logs/results and final observed inputs when evidence storage remains available.

## Small local checks

```sh
node --test scripts/packages/ai-flows-consumer.test.mjs
node --test tests/ai-flows-installed/crate-extraction.test.mjs
```

These checks create isolated fixtures. Crate tests run finite tar commands and small fixture extractions.
Their oversized sparse fixture is rejected from metadata before its logical file can be expanded.
They run no Cargo, browser or provider command.
After native acceptance and independent review, the coordinator must rerun affected checks and the full local verifier before integration.
