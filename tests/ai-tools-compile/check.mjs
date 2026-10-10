import { spawnSync } from "node:child_process";

const manifest = "tests/ai-tools-compile/Cargo.toml";
function cargo(binary, json = false) {
  const arguments_ = ["check", "--locked", "--manifest-path", manifest, "--bin", binary];
  if (json) arguments_.push("--message-format=json");
  const result = spawnSync("cargo", arguments_, {
    encoding: "utf8",
    timeout: 180000,
    maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error || result.signal || result.status === null) {
    throw new Error(`${binary}: Cargo execution failed before diagnostic matching (${result.error?.code ?? result.signal ?? "missing status"})`);
  }
  return result;
}
for (const binary of ["read_only", "action_route"]) {
  const positive = cargo(binary);
  if (positive.status !== 0) {
    process.stderr.write(positive.stderr + positive.stdout);
    throw new Error(`${binary}: external public API compilation failed`);
  }
}
const negative = cargo("cannot_execute", true);
const errors = negative.stdout.split("\n").filter(Boolean).flatMap((line) => {
  try {
    const value = JSON.parse(line);
    return value.reason === "compiler-message" && value.message.level === "error" ? [value.message] : [];
  } catch {
    return [];
  }
});
const intended = errors.find((error) => error.code?.code === "E0599" && error.message.includes("execute") && error.spans.some((span) => span.is_primary && span.file_name.endsWith("cannot_execute.rs") && span.line_start === 4));
if (negative.status === 0 || errors.length !== 1 || !intended) {
  process.stderr.write(negative.stderr + negative.stdout);
  throw new Error("cannot_execute: expected exactly E0599 at source line 4");
}
console.log("External ReadContext read/query API passed; execute correctly rejected with E0599 at source line 4.");
