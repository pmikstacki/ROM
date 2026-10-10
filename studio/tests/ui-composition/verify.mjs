// Frozen-source installed maintenance consumer. This fixture cannot admit a release.
import {
  cpSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  lstatSync,
  realpathSync,
  existsSync,
  readdirSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { isDeepStrictEqual } from "node:util";
import {
  reserveOutput,
  sourceManifest,
  hashBytes,
  assertLockedGraph,
  browserOutcomes,
} from "../public-controls/verification.mjs";
import {
  sourcePrefix,
  assertStudioArchiveFile,
  extractStudioArchive,
} from "../public-controls/archive-admission.mjs";

const here = dirname(fileURLToPath(import.meta.url)),
  studio = resolve(here, "../..");
const args = process.argv.slice(2);
let requested,
  sourceArchive,
  sourceDirectory,
  binary,
  provenance,
  adapter,
  scenario,
  port = 43301,
  prepare = false,
  bootstrap = false;
for (let index = 0; index < args.length; index++) {
  const key = args[index];
  if (key === "--prepare-only") prepare = true;
  else if (key === "--bootstrap-lock") bootstrap = true;
  else if (key === "--admit")
    throw Error(
      "release admission remains open; this fixture is source-candidate evidence",
    );
  else if (
    [
      "--output",
      "--source-archive",
      "--source-dir",
      "--host-binary",
      "--host-provenance",
      "--adapter",
      "--port",
      "--scenario",
    ].includes(key)
  ) {
    const value = args[++index];
    if (!value || value.startsWith("--"))
      throw Error("missing verifier argument");
    if (key === "--output") requested = value;
    else if (key === "--source-archive") sourceArchive = resolve(value);
    else if (key === "--source-dir") sourceDirectory = resolve(value);
    else if (key === "--host-binary") binary = resolve(value);
    else if (key === "--host-provenance") provenance = resolve(value);
    else if (key === "--adapter") adapter = value;
    else if (key === "--scenario") scenario = value;
    else port = Number(value);
  } else if (!key.startsWith("--") && !requested) requested = key;
  else throw Error("unknown verifier argument");
}
if (sourceArchive && sourceDirectory) throw Error("select one supplied source");
if (!Number.isSafeInteger(port) || port < 1024 || port > 65535)
  throw Error("invalid fixture port");
if (!prepare && (!binary || !["sqlite", "redb"].includes(adapter)))
  throw Error("runtime requires fixture binary and adapter");
const output = reserveOutput(requested),
  consumer = join(output, "consumer"),
  extracted = join(output, "extracted");
const archive = join(output, "studio-source.tar.gz"),
  commands = [];
function run(program, argv, label, cwd = consumer, extra = {}) {
  const result = spawnSync(program, argv, {
    cwd,
    env: { ...process.env, ...extra },
    encoding: "utf8",
    timeout: 180000,
    maxBuffer: 8 * 1024 * 1024,
  });
  const record = {
    program,
    args: argv,
    cwd,
    exit_code: result.status,
    signal: result.signal,
    error: result.error?.message,
  };
  commands.push(record);
  writeFileSync(
    join(output, label + ".log"),
    JSON.stringify(record) +
      "\n" +
      (result.stdout ?? "") +
      (result.stderr ?? ""),
    { flag: "wx" },
  );
  if (result.status !== 0 || result.error) throw Error(label + " failed");
  return result;
}
try {
  if (sourceArchive) {
    assertStudioArchiveFile(sourceArchive);
    cpSync(sourceArchive, archive);
  } else {
    const stage = join(output, "source-stage"),
      candidate = join(stage, sourcePrefix),
      source = sourceDirectory ?? studio;
    mkdirSync(candidate, { recursive: true });
    for (const name of ["src", "package.json", "package-lock.json"])
      cpSync(join(source, name), join(candidate, name), { recursive: true });
    for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"])
      cpSync(
        existsSync(join(source, name))
          ? join(source, name)
          : join(dirname(source), name),
        join(candidate, name),
      );
    sourceManifest(candidate);
    run("tar", ["-czf", archive, "-C", stage, sourcePrefix], "archive", stage);
  }
  const archiveHash = hashBytes(readFileSync(archive));
  mkdirSync(join(output, "source-extraction"));
  cpSync(
    extractStudioArchive(archive, join(output, "source-extraction")),
    extracted,
    { recursive: true },
  );
  const expected = sourceManifest(extracted),
    fixtureFiles = {};
  writeFileSync(
    join(output, "source.json"),
    JSON.stringify(
      { archive_sha256: archiveHash, source_files: expected },
      null,
      2,
    ),
  );
  function capture(path) {
    const full = join(here, path),
      info = lstatSync(full);
    if (info.isSymbolicLink()) throw Error("fixture symlinks are forbidden");
    if (info.isDirectory())
      for (const name of readdirSync(full)) capture(join(path, name));
    else fixtureFiles[path] = hashBytes(readFileSync(full));
  }
  for (const path of [
    "consumer",
    "verify.mjs",
    "http.mjs",
    "playwright.config.ts",
  ])
    if (existsSync(join(here, path))) capture(path);
  writeFileSync(
    join(output, "fixture-source.json"),
    JSON.stringify(fixtureFiles, null, 2),
  );
  cpSync(join(here, "consumer"), consumer, { recursive: true });
  const lockPath = join(consumer, "package-lock.json");
  if (bootstrap)
    run(
      "npm",
      [
        "install",
        "--package-lock-only",
        "--ignore-scripts",
        "--prefer-offline",
        "--install-links",
        "--no-audit",
        "--no-fund",
      ],
      "bootstrap-lock",
    );
  const frozen = JSON.parse(readFileSync(lockPath)),
    producer = JSON.parse(readFileSync(join(extracted, "package.json")));
  const local = frozen.packages["node_modules/rom-studio"];
  if (
    local?.version !== producer.version ||
    !isDeepStrictEqual(local.dependencies, producer.dependencies)
  )
    throw Error("source dependency metadata differs from frozen consumer lock");
  run(
    "npm",
    ["ci", "--prefer-offline", "--install-links", "--no-audit", "--no-fund"],
    "install",
  );
  assertLockedGraph(frozen, JSON.parse(readFileSync(lockPath)));
  const installed = join(consumer, "node_modules/rom-studio");
  if (
    lstatSync(installed).isSymbolicLink() ||
    !realpathSync(installed).startsWith(realpathSync(consumer) + "/")
  )
    throw Error("installed package is linked outside consumer");
  if (!isDeepStrictEqual(expected, sourceManifest(installed)))
    throw Error("installed source mismatch");
  for (const [path, entry] of Object.entries(frozen.packages)) {
    if (!path || !entry.version) continue;
    const manifest = join(consumer, path, "package.json");
    if (!existsSync(manifest) && entry.optional) continue;
    if (
      !existsSync(manifest) ||
      JSON.parse(readFileSync(manifest)).version !== entry.version
    )
      throw Error("realized dependency graph differs: " + path);
  }
  run("npm", ["run", "check"], "type-check");
  run("npm", ["run", "build"], "build");
  let outcomes, native;
  if (!prepare) {
    if (!process.env.ROM_WEBKIT_EXECUTABLE)
      throw Error("actual WebKit executable required");
    if (provenance) {
      native = JSON.parse(readFileSync(provenance));
      if (native.binary_sha256 !== hashBytes(readFileSync(binary)))
        throw Error("native binary provenance mismatch");
      cpSync(provenance, join(output, "native-provenance.json"));
    }
    cpSync(join(here, "http.mjs"), join(output, "http.mjs"));
    run("node", ["tests/bind-failure.mjs"], "bind-failure", consumer, {
      ROM_UI_HOST: binary,
      ROM_UI_ADAPTER: adapter,
      ROM_UI_BIND_EVIDENCE: join(output, "bind-failure"),
      ROM_UI_PORT: String(port),
    });
    cpSync(
      join(here, "playwright.config.ts"),
      join(consumer, "playwright.config.ts"),
    );
    const browser = run(
      "npm",
      [
        "exec",
        "--offline",
        "--",
        "playwright",
        "test",
        "--config",
        "playwright.config.ts",
        "--reporter=json",
        ...(scenario ? ["--grep", scenario] : []),
      ],
      "browser",
      consumer,
      {
        ROM_UI_HOST: binary,
        ROM_UI_ADAPTER: adapter,
        ROM_UI_DATABASE: join(output, adapter + ".db"),
        ROM_UI_EVIDENCE: output,
        ROM_UI_PORT: String(port),
      },
    );
    const report = JSON.parse(browser.stdout);
    writeFileSync(
      join(output, "browser-results.json"),
      JSON.stringify(report, null, 2),
    );
    outcomes = browserOutcomes(report);
    for (const engine of ["chromium", "webkit"])
      if (
        !outcomes[engine]?.length ||
        outcomes[engine].some((value) => value !== "passed")
      )
        throw Error("incomplete browser matrix: " + engine);
  }
  if (
    hashBytes(readFileSync(archive)) !== archiveHash ||
    !isDeepStrictEqual(expected, sourceManifest(installed))
  )
    throw Error("source changed during verification");
  const result = {
    prepared: true,
    completed: !prepare && !scenario,
    diagnostic_scenario: scenario,
    admitted: false,
    source_mode: sourceArchive
      ? "supplied_source_archive"
      : sourceDirectory
        ? "supplied_source_directory"
        : "authoring_checkout",
    archive_sha256: archiveHash,
    source_files: expected,
    installed_source_matches: true,
    source_lock_sha256: hashBytes(
      readFileSync(join(extracted, "package-lock.json")),
    ),
    consumer_lock_sha256: hashBytes(readFileSync(lockPath)),
    fixture_source_sha256: fixtureFiles,
    dependency_bootstrap: bootstrap,
    browser_executed: !prepare,
    browser_outcomes: outcomes,
    adapter,
    native_binary_sha256: binary ? hashBytes(readFileSync(binary)) : null,
    native_provenance: native,
    commands,
    limits: [
      "Fixed fixture credentials are not production identity.",
      "Native coverage is restricted to the recorded immutable fixture source witness; later core changes are not inferred.",
      "Human, original-consumer and release acceptance remain open.",
    ],
  };
  writeFileSync(join(output, "result.json"), JSON.stringify(result, null, 2));
  console.log(
    JSON.stringify({
      output,
      prepared: true,
      browser: !prepare,
      admitted: false,
    }),
  );
} catch (error) {
  writeFileSync(
    join(output, "failure.json"),
    JSON.stringify(
      { completed: false, error: error.message, commands },
      null,
      2,
    ),
  );
  console.error(error.message);
  process.exitCode = 1;
}
