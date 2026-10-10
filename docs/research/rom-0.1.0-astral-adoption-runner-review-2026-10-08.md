# Independent review: Astral package adoption runner

Date: 2026-10-08. Status: two process-cleanup failures reproduced; real adoption is not admitted.

The review inspected `scripts/packages/astral-adoption.mjs`, its Node tests, and the Rust adoption plan.
It read the real Astral manifest and relevant source paths. It changed no original application or implementation source.
All executable probes used private fixtures and fake Node tools. No Cargo command or native build ran.

## Findings

| Priority | Location in reviewed source | Finding |
| --- | --- | --- |
| P1 | `astral-adoption.mjs:157` | A successful command can leave descendants alive. Cleanup kills a process group only after deadline or output overflow. |
| P1 | `astral-adoption.mjs:134`, `:205` | Runner interruption leaves its detached active command alive. No scoped SIGINT/SIGTERM cleanup handler exists. |
| P2 | `astral-adoption.mjs:203` | Failure evidence does not audit final original-consumer or package inventories. Drift during a failed command remains unreported. |

The real run must wait for the process-cleanup corrections and maintained regression tests.
Failure-path source audits must retain the primary command error. They must not create acceptance evidence after drift.
The review does not authorize broad process termination or deletion.

## Successful-command descendant probe

Evidence: `/tmp/rom-astral-review-leak-node-wzdPxN/review-result.json`.
The fake compiler starts a Node child with ignored stdio, records its PID, and exits successfully.
The next fake Cargo command exits 2. The runner reports failure at `resolve`.
The owned child, PID 1140096, remains alive after the runner rejects the command sequence.

The reviewer sent SIGKILL only to that exact owned child. The subsequent `/proc` entry was absent.
The private fixture, fake tools, command results, and failure record remain preserved.
An earlier shell control did not reproduce the leak. It remains separately preserved under `/tmp/rom-astral-review-leak-VLUUN8`.

The defect is not specific to failure of the next command. Normal completion does not inspect or terminate the previous detached group.
A test can start a background helper whose stdio is closed. Cargo completion alone does not prove helper drainage.

## Parent interruption probe

Evidence: `/tmp/rom-astral-review-interrupt-Yf6EIl/review-result.json`.
The runner starts a fake compiler that remains active. The probe captures its `/proc` process identity.
The reviewer sends SIGTERM to the runner, PID 1141427. The runner exits by SIGTERM.
The owned compiler, PID 1141440, remains alive. Its captured PGID and session are both 1141440.

The reviewer sends SIGKILL only to owned process group -1141440. Its subsequent `/proc` entry is absent.
The private fixture and interrupt evidence remain preserved. No unrelated process was signalled.

Add maintained cases for normal-exit descendants, command failure, deadline, output overflow, and parent interruption.
Record escalation and final disposal. Do not report acceptance while an owned command group remains active.
Process-group supervision does not contain a descendant that deliberately creates a new session. State this limit or use an admitted containment boundary.

## Package and source boundary review

The four direct ROM paths change to exact package versions. Other manifest content remains intact in the maintained case.
The runner copies source, tests, knowledge, and unchanged Astrorust inputs into a new application directory.
The original ROM vendor tree is not copied. The real application uses `knowledge/library.json` in compile-time includes.

Resolved ROM crates must match exact extracted package names, versions, manifest paths, and captured inventories.
Unknown local dependencies are rejected. Astrorust dependencies are restricted to the copied vendor tree.
Registry lock entries must match the original application lock or candidate ROM lock, including source and checksum.
The acceptance record distinguishes candidate-lock dependency drift from the original consumer graph.

The runner records inventories of extracted packages. It does not establish archive-to-extraction identity by itself.
The plan states this caller prerequisite explicitly. The real run needs the witnessed `.crate` archive and extraction proof first.

The original-input audit covers the selected Cargo, Rust, knowledge, and Astrorust inputs.
It does not audit all files in the original application directory. This Rust adoption proof does not establish frontend adoption or deployment.
Inventories deliberately omit `.git`, `target`, and `node_modules` directories.

Successful admission rechecks package inventories, the isolated application inputs, original selected inputs, and frozen lock.
Failed commands currently bypass those final source checks. Preserve a failure-path audit even when metadata cannot be obtained.

## Reproduced maintained checks

The reviewer ran:

```sh
node --test scripts/packages/astral-adoption.test.mjs
```

Seven tests passed with no failure, skip, or cancellation.
They cover manifest preservation, isolated copies, symbolic paths, unknown packages, graph escapes, checksum drift, and missing budgets.
They do not exercise real Cargo, command drainage, parent interruption, or adoption by the original application.

Each command has an explicit deadline and combined output limit. These are not disk quotas or a bound on build allocation.
Actual native admission, package closure, external consumer tests, and final executable identity remain pending.
The report follows `docs/writing.md` and the ASD-STE100 guide. It does not certify dictionary compliance.

## Corrective review

The coordinator added `scripts/packages/astral-owned-command.mjs`. The reviewer inspected this module and the revised adoption runner.
The command now performs process-group cleanup after successful exit, failure, and interruption.
It sends TERM, performs bounded absence checks, escalates to KILL, and performs another bounded absence check.
An unresolved process group prevents command success. The result records group identity, disposal, stop reason, and output limits.

Scoped SIGINT and SIGTERM handlers request owned-group termination. The handlers are removed after command cleanup.
The tests cover each signal separately. They do not establish containment against SIGKILL or descendants that create another session.
Use the admitted host containment boundary for those broader limits. Do not infer arbitrary descendant containment from a process-group check.

The revised runner records final original-consumer, extracted-package, and isolated-application inventories after failure.
An audit observation error records `unchanged: false`. A secondary audit-write failure does not replace the primary command failure.
On success, the final audit is written and its unchanged conditions are tested before `acceptance.json` is created.
The expected application inventory includes the frozen resolved lock hash. Initial resolution does not incorrectly count as unexpected source drift.

The reviewer independently ran the complete native-free cases on the corrected source:

```sh
node --test scripts/packages/astral-adoption.test.mjs scripts/packages/astral-adoption-process.test.mjs
```

All 13 tests passed with no failure, skip, or cancellation. The process cases assert that owned descendants no longer run.
Deadline and output-overflow cases also assert `groupAbsent: true`. The failed-tool case verifies retained source drift and the original command error.

| Evidence under `/var/tmp/` | Result | SHA-256 |
| --- | --- | --- |
| `rom-010-astral-adoption-process-red.log` | Two intended process-cleanup failures before correction. | `66574d773e44956fbd8d3be867a8a5cbd761f9c0979ae9c4dbfdb5c05f6fee2a` |
| `rom-010-astral-adoption-final-audit-red.log` | Seven passed cases and one intended missing-audit failure. | `6de86ed87286e6d24bd597ffdda54a437ebb4075012a43ace3a8709956ee8ab1` |
| `rom-010-astral-adoption-final-admission-green.log` | Coordinator's 13 corrected cases passed. | `de23c45f1572401fe39e995d53eb272be17a67d0bdb6360ed2fe28c175f992ad` |
| `rom-010-astral-adoption-independent-corrective.log` | Reviewer's 13 corrected cases passed. | `f833f7508db11e237a5105de3f3b66452c483912147d14986cb2a19778b4af92` |

The original findings are resolved within the owned process-group contract. The initial private failure witnesses remain preserved.
No reviewed runner blocker remains for an admitted real run. Archive-to-extraction proof and native allocation admission remain prerequisites.
This review does not establish that the real Astral application compiles, tests, or runs with the candidate packages.
Its actual package adoption, binary identity, source preservation, and deployment evidence remain pending.
