/** Deployment-owned configuration. This metadata confers no server authority. */
export interface StudioBootstrapStore {
  name: string;
  maxBytes: number;
  maxSlots: number;
  timeoutMs: number;
}
export interface StudioBootstrapConfig {
  version: 1;
  authority: string;
  recovery: {
    namespace: string;
    retryEpoch: string;
    maxBytes: number;
    intentStore: StudioBootstrapStore;
    editorStore: StudioBootstrapStore;
  };
}
const bytes = (value: string) => new TextEncoder().encode(value).byteLength;
const fail = (): never => {
  throw Error("Invalid Studio bootstrap configuration.");
};
function fields(value: unknown, keys: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).sort().join(",") !== [...keys].sort().join(",")
  )
    fail();
  return value as Record<string, unknown>;
}
export function bootstrapIdentifier(value: unknown): string {
  if (
    typeof value !== "string" ||
    !value ||
    bytes(value) > 4096 ||
    /[\p{Cc}\p{Surrogate}]/u.test(value)
  )
    return fail();
  return value;
}
function integer(value: unknown, min: number, max: number): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    Object.is(value, -0) ||
    value < min ||
    value > max
  )
    return fail();
  return value;
}
function store(value: unknown, payloadBytes: number) {
  const data = fields(value, ["name", "maxBytes", "maxSlots", "timeoutMs"]);
  bootstrapIdentifier(data.name);
  // Reserve bytes for the versioned record envelope, not an additional payload allocation.
  integer(data.maxBytes, payloadBytes + 4096, 4 * 1024 * 1024);
  integer(data.maxSlots, 1, 4096);
  integer(data.timeoutMs, 1, 60000);
}
export function parseStudioBootstrap(text: string): StudioBootstrapConfig {
  try {
    if (typeof text !== "string" || bytes(text) > 65536) fail();
    const data = fields(JSON.parse(text), ["version", "authority", "recovery"]);
    if (data.version !== 1) fail();
    bootstrapIdentifier(data.authority);
    const recovery = fields(data.recovery, [
      "namespace",
      "retryEpoch",
      "maxBytes",
      "intentStore",
      "editorStore",
    ]);
    bootstrapIdentifier(recovery.namespace);
    if (
      typeof recovery.retryEpoch !== "string" ||
      !/^(0|[1-9][0-9]{0,19})$/.test(recovery.retryEpoch) ||
      BigInt(recovery.retryEpoch) > 18446744073709551615n
    )
      fail();
    const maxBytes = integer(recovery.maxBytes, 1, 1024 * 1024);
    store(recovery.intentStore, maxBytes);
    store(recovery.editorStore, maxBytes);
    return data as unknown as StudioBootstrapConfig;
  } catch {
    return fail();
  }
}
