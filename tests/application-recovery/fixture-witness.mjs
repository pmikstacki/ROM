import { readdirSync, readFileSync, lstatSync } from "node:fs";
import { createHash } from "node:crypto";
const root = "/root/ROM";
export function captureFixtureSources() {
  const paths = [
    "tests/application-recovery",
    "tests/identity/provider",
  ].flatMap((directory) =>
    readdirSync(`${root}/${directory}`)
      .filter((name) => name.endsWith(".mjs"))
      .map((name) => `${root}/${directory}/${name}`),
  );
  paths.push(
    `${root}/tests/identity/provider/host/assets/index.html`,
    `${root}/studio/package-lock.json`,
    `${root}/studio/node_modules/@playwright/test/package.json`,
  );
  return paths.sort().map((path) => {
    const value = lstatSync(path);
    if (!value.isFile() || value.size > 1048576)
      throw Error("bounded fixture source required");
    return {
      path,
      sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
    };
  });
}
