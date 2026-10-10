import { requireWorkloadWindow } from "../identity/provider/window-profile.mjs";
import { captureFixtureSources } from "./fixture-witness.mjs";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  existsSync,
  lstatSync,
  readlinkSync,
  statfsSync,
} from "node:fs";
import { randomBytes, createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import {
  OwnedLauncher,
  readProcessIdentity,
  sameProcessIdentity,
} from "../identity/provider/launch.mjs";
import {
  createProviderCgroup,
  admitOwnedWrapper,
  snapshotProviderCgroup,
  requireDrainedCgroup,
} from "../identity/provider/cgroup.mjs";
import {
  requireOwnedListener,
  requireVacantListener,
} from "../identity/provider/network-admission.mjs";
import { namespaceTools } from "../identity/provider/trust-namespace.mjs";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
import { requestOwnedStop } from "../identity/provider/stop-file.mjs";
const root = "/var/tmp/rom-010-authentik-20261007/run/volume",
  hash = (path) =>
    createHash("sha256").update(readFileSync(path)).digest("hex"),
  read = (path) => JSON.parse(readFileSync(path));
process.umask(0o077);
export async function checkRecoveredHttp(readyPath, restorePath) {
  const ready = read(readyPath),
    restore = read(restorePath),
    build = read(restore.build_identity),
    directory = restore.private_root,
    evidence = restorePath.slice(0, restorePath.lastIndexOf("/")),
    base = read(directory + "/prepare.json");
  if (restore.status !== "restore-checks-passed")
    throw Error("actual restore checks required");
  verifyPreservedBinary(build.binary);
  for (const s of build.sources)
    if (hash(s.path) !== s.sha256)
      throw Error("matching current application source required");
  requireWorkloadWindow(ready.ready_admission, 150000);
  requireOwnedListener(44390, ready.relay.target);
  for (const p of [44389, 44391, 44392, 44393]) requireVacantListener(p);
  if (
    Number(statfsSync(root).bavail) * Number(statfsSync(root).bsize) <
    1024 ** 3
  )
    throw Error("private provider envelope headroom");
  const nonce = randomBytes(12).toString("hex"),
    compat = root + "/private/https-host-authoring-" + nonce;
  mkdirSync(compat, { mode: 0o700 });
  const home = directory + "/browser-home",
    temporary = root + "/t-" + randomBytes(4).toString("hex");
  mkdirSync(home, { mode: 0o700 });
  mkdirSync(home + "/.pki", { mode: 0o700 });
  mkdirSync(home + "/.pki/nssdb", { mode: 0o700 });
  mkdirSync(temporary, { mode: 0o700 });
  const trust = read(
      root + "/evidence/tls-trust-ffda05d57c88b265a3fcc9a4/result.json",
    ),
    ca = read(
      root + "/evidence/nss-private-trust-8b864cae6c63b27375327271/result.json",
    ),
    nss = read(
      root + "/evidence/nss-tool-ee61adb2a4df6c0baecd5f6e/result.json",
    );
  const hostLauncher = new OwnedLauncher(),
    browserLauncher = new OwnedLauncher(),
    hostGroup = createProviderCgroup(),
    browserGroup = createProviderCgroup(),
    record = {
      schema: 1,
      status: "starting",
      adapter: base.adapter,
      restore_result: restorePath,
      provider_ready: readyPath,
      source_available_to_parent: lstatSync(
        base.source_directory + "/source-only-canary",
      ).isFile(),
      processes: [],
      fixture_sources_before: captureFixtureSources(),
      host_trust_before: {
        sha256: hash("/etc/ssl/certs/ca-certificates.crt"),
        namespace: readlinkSync("/proc/self/ns/mnt"),
      },
      artifact_admission: false,
      original_consumer_acceptance: false,
    };
  let host, proxy;
  const hostStop = directory + "/destination-stop",
    proxyStop = directory + "/proxy-stop";
  async function launch(
    executable,
    args,
    role,
    launcher = hostLauncher,
    group = hostGroup,
    extra = {},
  ) {
    const child = launcher.launch(executable, args, {
      timeoutMs: 100000,
      graceMs: 1000,
      drainMs: 1000,
      outputBytes: 65536,
      admitIdentity: (id) => admitOwnedWrapper(group, id),
      ...extra,
    });
    const observation = {
      role,
      wrapper: child.identity,
      target: await child.targetStarted,
      stdout_bytes: 0,
      stderr_bytes: 0,
    };
    child.child.stdout.on("data", (v) => {
      observation.stdout_bytes += v.length;
    });
    child.child.stderr.on("data", (v) => {
      observation.stderr_bytes += v.length;
    });
    record.processes.push(observation);
    return { child, observation };
  }
  async function listener(port, target) {
    for (let i = 0; i < 80; i++) {
      try {
        requireOwnedListener(port, target);
        return;
      } catch {
        await new Promise((r) => setTimeout(r, 50));
      }
    }
    throw Error("owned recovery listener missing");
  }
  try {
    for (const args of [
      ["-N", "--empty-password", "-d", "sql:" + home + "/.pki/nssdb"],
      [
        "-A",
        "-d",
        "sql:" + home + "/.pki/nssdb",
        "-n",
        "ROM isolated identity fixture CA",
        "-t",
        "C,,",
        "-i",
        ca.ca.certificate,
      ],
    ]) {
      const value = await launch(
        nss.tool.loader.path,
        ["--library-path", nss.tool.library_path, nss.tool.path, ...args],
        "private-nss",
        browserLauncher,
        browserGroup,
        {
          timeoutMs: 30000,
          env: {
            HOME: home,
            TMPDIR: temporary,
            PATH: "/run/current-system/sw/bin:/usr/bin:/bin",
            LANG: "C",
            TZ: "UTC",
          },
        },
      );
      value.observation.result = await value.child.closed;
      await value.child.physicalClose;
      if (
        value.observation.result.exit_code !== 0 ||
        !value.observation.result.drained
      )
        throw Error("private NSS setup");
    }
    const configuration = directory + "/serve.json";
    writeFileSync(
      configuration,
      JSON.stringify({
        ...base,
        mode: "serve",
        directory: directory + "/destination",
        stop_file: hostStop,
      }),
      { flag: "wx", mode: 0o600 },
    );
    const namespaceConfig = directory + "/serve-namespace.json",
      started = evidence + "/host-started.json";
    writeFileSync(
      namespaceConfig,
      JSON.stringify({
        source: base.source_directory,
        empty: directory + "/empty",
        private_root: directory,
        parent_namespace: readlinkSync("/proc/self/ns/mnt"),
        namespace_result: evidence + "/serve-namespace.json",
        binary: build.binary.path,
        host_configuration: configuration,
        mode: "serve",
        process_result: evidence + "/serve-process.json",
        process_started: started,
      }),
      { flag: "wx", mode: 0o600 },
    );
    host = await launch(
      namespaceTools.unshare,
      [
        "--mount",
        "--propagation",
        "private",
        "--",
        process.execPath,
        fileURLToPath(new URL("./source-isolation-run.mjs", import.meta.url)),
        namespaceConfig,
      ],
      "isolated-recovered-host",
    );
    let identity;
    for (let i = 0; i < 100; i++) {
      if (existsSync(started)) {
        const file = lstatSync(started);
        if (
          !file.isFile() ||
          file.nlink !== 1 ||
          file.size > 4096 ||
          (file.mode & 0o777) !== 0o600
        )
          throw Error("private recovery Host birth record");
        identity = read(started).target;
        break;
      }
      await new Promise((r) => setTimeout(r, 50));
    }
    if (
      !identity ||
      !sameProcessIdentity(identity, readProcessIdentity(identity.pid)) ||
      readlinkSync(`/proc/${identity.pid}/ns/mnt`) ===
        readlinkSync("/proc/self/ns/mnt") ||
      readFileSync(`/proc/${identity.pid}/cgroup`, "utf8").trim() !==
        `0::${hostGroup.membership}`
    )
      throw Error("actual recovered Host namespace/birth mismatch");
    record.host_continuous_birth = identity;
    await listener(44391, identity);
    const proxyConfiguration = compat + "/proxy.json";
    writeFileSync(
      proxyConfiguration,
      JSON.stringify({
        key: trust.leaves.trusted.key,
        certificate: trust.leaves.trusted.certificate,
        result: evidence + "/proxy.json",
        stop_file: proxyStop,
      }),
      { flag: "wx", mode: 0o600 },
    );
    proxy = await launch(
      process.execPath,
      [
        fileURLToPath(
          new URL("../identity/provider/https-proxy-run.mjs", import.meta.url),
        ),
        proxyConfiguration,
      ],
      "closed-tls-proxy",
    );
    for (const p of [44389, 44392, 44393])
      await listener(p, proxy.observation.target);
    const browserConfiguration = directory + "/browser.json";
    writeFileSync(
      browserConfiguration,
      JSON.stringify({
        adapter: base.adapter,
        environment:
          root +
          "/private/authentik-96b706d518977e471f458dfff7b9503a/authentik.env",
        result: evidence + "/browser.json",
      }),
      { flag: "wx", mode: 0o600 },
    );
    const browser = await launch(
      process.execPath,
      [
        fileURLToPath(new URL("./browser-run.mjs", import.meta.url)),
        browserConfiguration,
      ],
      "current-identity-browser",
      browserLauncher,
      browserGroup,
      {
        timeoutMs: 60000,
        env: {
          HOME: home,
          TMPDIR: temporary,
          PATH: "/run/current-system/sw/bin:/usr/bin:/bin",
          LANG: "C",
          TZ: "UTC",
          PLAYWRIGHT_BROWSERS_PATH: "/root/.cache/ms-playwright",
          NODE_EXTRA_CA_CERTS: ca.ca.certificate,
          SSL_CERT_FILE: ca.ca.certificate,
          NIX_SSL_CERT_FILE: ca.ca.certificate,
        },
      },
    );
    browser.observation.result = await browser.child.closed;
    await browser.child.physicalClose;
    record.browser = read(evidence + "/browser.json");
    if (
      browser.observation.result.exit_code !== 0 ||
      record.browser.status !== "passed"
    )
      throw Error("actual fresh-provider recovery checks failed");
    record.whole_application_rto_ms =
      record.browser.authenticated_checks_finished_unix_ms -
      restore.restore_started_unix_ms;
    record.rpo_acknowledged_mutations_lost =
      restore.database_attachment_work_checks.post_snapshot_acknowledged_lost;
    record.rpo_time_ms =
      restore.last_post_snapshot_ack.last_acknowledged_unix_milliseconds -
      restore.source_snapshot.snapshot_finished_unix_milliseconds;
    record.status = "passed";
  } catch (error) {
    record.status = "failed";
    record.failure = error.message;
    process.exitCode = 1;
  } finally {
    for (const path of [hostStop, proxyStop])
      if (!existsSync(path))
        writeFileSync(path, "stop\n", { flag: "wx", mode: 0o600 });
    record.browser_drain = await browserLauncher.drain();
    record.host_drain = await hostLauncher.drain();
    for (const value of [host, proxy])
      if (value) {
        value.observation.result = await value.child.closed;
        await value.child.physicalClose;
      }
    record.host_group = snapshotProviderCgroup(hostGroup);
    record.browser_group = snapshotProviderCgroup(browserGroup);
    requireDrainedCgroup(record.host_group.cgroup_events);
    requireDrainedCgroup(record.browser_group.cgroup_events);
    for (const p of [44389, 44391, 44392, 44393]) requireVacantListener(p);
    record.source_available_in_parent_after = lstatSync(
      base.source_directory + "/source-only-canary",
    ).isFile();
    record.native_source_changed = build.sources
      .filter((s) => hash(s.path) !== s.sha256)
      .map((s) => s.path);
    verifyPreservedBinary(build.binary);
    record.fixture_sources_after = captureFixtureSources();
    record.fixture_sources_unchanged =
      JSON.stringify(record.fixture_sources_before) ===
      JSON.stringify(record.fixture_sources_after);
    record.host_trust_after = {
      sha256: hash("/etc/ssl/certs/ca-certificates.crt"),
      namespace: readlinkSync("/proc/self/ns/mnt"),
    };
    record.host_trust_unchanged =
      JSON.stringify(record.host_trust_before) ===
      JSON.stringify(record.host_trust_after);
    if (
      !record.fixture_sources_unchanged ||
      !record.host_trust_unchanged ||
      record.native_source_changed.length
    )
      record.status = "source-drift";
    writeFileSync(
      evidence + "/http-result.json",
      JSON.stringify(record, null, 2),
      { flag: "wx", mode: 0o600 },
    );
    console.log(
      JSON.stringify({
        status: record.status,
        adapter: record.adapter,
        evidence,
        rpo_mutations: record.rpo_acknowledged_mutations_lost,
        rto_ms: record.whole_application_rto_ms,
      }),
    );
  }
  return record;
}
if (process.argv[1] === fileURLToPath(import.meta.url))
  await checkRecoveredHttp(process.argv[2], process.argv[3]);
