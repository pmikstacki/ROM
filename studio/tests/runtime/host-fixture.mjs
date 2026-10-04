// Real loopback Rust host and human OIDC fixture. Retain run databases and logs.
import { spawn } from "node:child_process";
import { createServer, connect } from "node:net";
import { mkdtemp, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { once } from "node:events";
import { startHumanProvider } from "../../../demo/provider-fixture/human-provider.mjs";
export async function startHost(backend, options = {}) {
  const directory = await mkdtemp("/var/tmp/rom-studio-host-browser-");
  const reserved = createServer();
  reserved.listen(0, "127.0.0.1");
  await once(reserved, "listening");
  const port = reserved.address().port;
  await new Promise((r) => reserved.close(r));
  const origin = `http://127.0.0.1:${port}`;
  const provider = await startHumanProvider({
    clientId: "studio",
    clientSecret: "controlled-host-test-secret",
    redirectUri: `${origin}/rom-studio/auth/callback/local`,
    beforeRequest: options.beforeRequest,
  });
  let child,
    generation = 0,
    socket;
  async function stop() {
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    const exited = once(child, "exit");
    child.kill("SIGTERM");
    const timer = setTimeout(() => child.kill("SIGKILL"), 8000);
    await exited;
    clearTimeout(timer);
  }
  async function launch() {
    socket = join(directory, `controls-${++generation}.sock`);
    child = spawn(
      process.env.ROM_STUDIO_DEMO_BINARY ??
        "/var/lib/nixos-containers/rom-dev/var/tmp/rom-release-measured-verification-target/debug/rom-demo",
      [
        "studio",
        backend,
        join(directory, "database"),
        String(port),
        process.env.ROM_STUDIO_ASSETS ?? resolve(".demo-dist"),
        provider.issuer,
        "--fixture-controls",
        socket,
      ],
      { stdio: ["ignore", "pipe", "pipe"] },
    );
    let out = "",
      err = "";
    child.stderr.on("data", (c) => (err = (err + c).slice(-65536)));
    await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(Error(`Readiness timeout: ${err}`)),
        15000,
      );
      child.once("error", (e) => {
        clearTimeout(timer);
        reject(e);
      });
      child.once("exit", (code) => {
        clearTimeout(timer);
        reject(Error(`Host exited ${code}: ${err}`));
      });
      child.stdout.on("data", (c) => {
        out = (out + c).slice(0, 65536);
        if (out.includes("\n")) {
          try {
            if (!JSON.parse(out.split("\n")[0]).ready)
              throw Error("Missing readiness");
            clearTimeout(timer);
            resolve();
          } catch (e) {
            clearTimeout(timer);
            reject(e);
          }
        }
      });
    }).finally(() =>
      writeFile(join(directory, `launch-${generation}.log`), out + err),
    );
  }
  try {
    await launch();
  } catch (e) {
    await stop();
    await provider.close();
    throw e;
  }
  return {
    origin,
    url: `${origin}/rom-studio/`,
    directory,
    alive() {
      return child.exitCode === null && child.signalCode === null;
    },
    terminate() {
      const exited = once(child, "exit").then(([code, signal]) => ({
        code,
        signal,
      }));
      child.kill("SIGTERM");
      return exited;
    },
    async control(request) {
      const client = connect(socket);
      let text = "";
      return await new Promise((resolve, reject) => {
        client.setTimeout(5000, () => client.destroy(Error("control timeout")));
        client.once("error", reject);
        client.once("connect", () =>
          client.write(JSON.stringify(request) + "\n"),
        );
        client.on("data", (c) => {
          text += c;
          if (text.includes("\n")) {
            client.end();
            try {
              resolve(JSON.parse(text.split("\n")[0]));
            } catch (e) {
              reject(e);
            }
          }
        });
      });
    },
    async restart() {
      await stop();
      await launch();
    },
    async close() {
      await stop();
      await provider.close();
    },
  };
}
