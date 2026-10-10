// Node 22.16+ test harness only. Reuse the maintained Studio operator parser.
import { registerHooks, stripTypeScriptTypes } from "node:module";
import {
  lstatSync,
  realpathSync,
  openSync,
  fstatSync,
  readSync,
  closeSync,
  constants,
} from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
const modules = [
  "codec",
  "normalization",
  "serialization",
  "validation",
  "work-validation",
].map(
  (name) => new URL(`../../studio/src/lib/client/${name}.ts`, import.meta.url),
);
const digest = (raw) => createHash("sha256").update(raw).digest("hex");
export function readParserSource(path) {
  const before = lstatSync(path);
  if (
    !before.isFile() ||
    before.isSymbolicLink() ||
    before.nlink !== 1 ||
    before.uid !== process.getuid() ||
    realpathSync(path) !== path ||
    before.size > 65536
  )
    throw Error("bounded operator parser source");
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = fstatSync(fd);
    if (
      stat.dev !== before.dev ||
      stat.ino !== before.ino ||
      stat.size !== before.size
    )
      throw Error("operator parser source changed before read");
    const bytes = Buffer.alloc(stat.size + 1),
      count = readSync(fd, bytes, 0, bytes.length, 0),
      after = fstatSync(fd);
    if (
      count !== stat.size ||
      after.size !== stat.size ||
      after.mtimeMs !== stat.mtimeMs
    )
      throw Error("operator parser source changed during read");
    return { stat, raw: bytes.subarray(0, count) };
  } finally {
    closeSync(fd);
  }
}
export function captureWorkParserSources() {
  let bytes = 0;
  return modules.map((url) => {
    const path = fileURLToPath(url),
      { stat: s, raw } = readParserSource(path);
    bytes += s.size;
    if (
      !s.isFile() ||
      s.isSymbolicLink() ||
      s.nlink !== 1 ||
      s.uid !== process.getuid() ||
      realpathSync(path) !== path ||
      s.size > 65536 ||
      bytes > 262144
    )
      throw Error("bounded operator parser source");
    return {
      path,
      device: s.dev,
      inode: s.ino,
      size: s.size,
      sha256: digest(raw),
    };
  });
}
let cached;
export async function loadWorkParser(expected = captureWorkParserSources()) {
  const sources = captureWorkParserSources();
  if (JSON.stringify(expected) !== JSON.stringify(sources))
    throw Error("operator parser source fence changed");
  if (cached) {
    const parser = await cached;
    if (JSON.stringify(parser.sources) !== JSON.stringify(sources))
      throw Error("compiled operator parser source changed");
    return parser;
  }
  cached = (async () => {
    const admitted = new Map(modules.map((url, i) => [url.href, sources[i]]));
    const hook = registerHooks({
      resolve(specifier, context, next) {
        if (admitted.has(context.parentURL)) {
          const target = new URL(specifier, context.parentURL).href;
          if (!admitted.has(target))
            throw Error("unknown operator parser dependency");
        }
        return next(specifier, context);
      },
      load(url, context, next) {
        const source = admitted.get(url);
        if (!source) {
          if (url.endsWith(".ts"))
            throw Error("unknown operator parser dependency");
          return next(url, context);
        }
        const { stat, raw } = readParserSource(source.path);
        if (
          stat.dev !== source.device ||
          stat.ino !== source.inode ||
          stat.size !== source.size ||
          raw.length !== source.size ||
          digest(raw) !== source.sha256
        )
          throw Error("operator parser source changed before compile");
        return {
          format: "module",
          source: stripTypeScriptTypes(raw.toString("utf8"), {
            mode: "strip",
            sourceUrl: url,
          }),
          shortCircuit: true,
        };
      },
    });
    try {
      const [work, codec] = await Promise.all([
        import(modules[4].href),
        import(modules[0].href),
      ]);
      if (
        JSON.stringify(captureWorkParserSources()) !== JSON.stringify(sources)
      )
        throw Error("operator parser source changed during compile");
      return {
        workResponse: work.workResponse,
        parseWire: codec.parseWire,
        sources,
      };
    } finally {
      hook.deregister();
    }
  })();
  try {
    return await cached;
  } catch (error) {
    cached = undefined;
    throw error;
  }
}
