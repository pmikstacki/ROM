import type { PendingIntentStore, StoredIntent } from "./types.ts";

export interface IndexedDbIntentStoreOptions {
  name: string;
  maxBytes: number;
  maxSlots: number;
  timeoutMs?: number;
  factory?: IDBFactory;
}
export interface IndexedDbIntentStore extends PendingIntentStore { close(): void }
const STORE = "records";
const size = (value: string) => new TextEncoder().encode(value).byteLength;
const identifier = (value: unknown): value is string => typeof value === "string" && !!value && size(value) <= 4096;
function intent(value: unknown, maxBytes: number): StoredIntent | null {
  if (value === undefined) return null;
  if (value === null || typeof value !== "object" || Array.isArray(value) ||
      Object.keys(value).sort().join(",") !== "payload,version") throw Error("invalid stored intent");
  const record = value as StoredIntent;
  if (!identifier(record.version) || typeof record.payload !== "string" ||
      size(JSON.stringify(record)) > maxBytes) throw Error("invalid stored intent");
  return { version: record.version, payload: record.payload };
}

/** Explicit host storage; bytes confer no authority and browser eviction remains possible. */
export async function openIndexedDbIntentStore(input: IndexedDbIntentStoreOptions): Promise<IndexedDbIntentStore> {
  const { name, maxBytes, maxSlots } = input;
  const timeout = input.timeoutMs ?? 10000;
  if (!identifier(name) || !Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 4 * 1024 * 1024 ||
      !Number.isSafeInteger(maxSlots) || maxSlots < 1 || maxSlots > 4096 ||
      !Number.isSafeInteger(timeout) || timeout < 1 || timeout > 60000) throw Error("invalid intent store configuration");
  const factory = input.factory ?? globalThis.indexedDB;
  if (!factory) throw Error("IndexedDB unavailable");
  const database = await new Promise<IDBDatabase>((resolve, reject) => {
    let settled = false;
    const request = factory.open(name, 1);
    const fail = (error: unknown) => { if (!settled) { settled = true; clearTimeout(timer); reject(error); } };
    const timer = setTimeout(() => fail(Error("intent store open timeout")), timeout);
    request.onblocked = () => fail(Error("intent store upgrade blocked"));
    request.onerror = () => fail(request.error ?? Error("intent store open failed"));
    request.onupgradeneeded = () => {
      if (settled) { request.transaction?.abort(); return; }
      try { request.result.createObjectStore(STORE); }
      catch (error) { request.transaction?.abort(); fail(error); }
    };
    request.onsuccess = () => {
      if (settled) { request.result.close(); return; }
      if (!request.result.objectStoreNames.contains(STORE)) {
        request.result.close(); fail(Error("intent store schema mismatch")); return;
      }
      settled = true; clearTimeout(timer); resolve(request.result);
    };
  });
  let closed = false;
  const active = new Set<() => void>();
  const close = () => {
    if (closed) return;
    closed = true;
    for (const abort of [...active]) abort();
    database.close();
  };
  database.onversionchange = close;
  database.onclose = close;
  function transaction<T>(write: boolean, operation: (store: IDBObjectStore, set: (value: T) => void, abort: (error: unknown) => void) => void): Promise<T> {
    return new Promise((resolve, reject) => {
      if (closed) { reject(Error("intent store closed")); return; }
      let tx: IDBTransaction;
      try {
        tx = write ? database.transaction(STORE, "readwrite", { durability: "strict" }) : database.transaction(STORE, "readonly");
      } catch (error) { reject(error); return; }
      let result!: T, settled = false;
      const finish = (error?: unknown) => {
        if (settled) return;
        settled = true; clearTimeout(timer); active.delete(cancel);
        if (error) reject(error); else resolve(result);
      };
      const abort = (error: unknown) => { try { tx.abort(); } catch {} finish(error); };
      const cancel = () => abort(Error("intent store closed"));
      const timer = setTimeout(() => abort(Error("intent store transaction timeout")), timeout);
      active.add(cancel);
      tx.oncomplete = () => finish();
      tx.onabort = () => finish(tx.error ?? Error("intent store transaction aborted"));
      try {
        if (write && tx.durability !== "strict") throw Error("strict IndexedDB durability unavailable");
        operation(tx.objectStore(STORE), value => { result = value; }, abort);
      } catch (error) { abort(error); }
    });
  }
  return {
    close,
    async read(slot) {
      if (!identifier(slot)) throw Error("invalid intent slot");
      return transaction<StoredIntent | null>(false, (store, set, abort) => {
        const request = store.get(slot);
        request.onsuccess = () => { try { set(intent(request.result, maxBytes)); } catch (error) { abort(error); } };
      });
    },
    async compareExchange(slot, expectedVersion, next) {
      if (!identifier(slot) || (expectedVersion !== null && !identifier(expectedVersion))) throw Error("invalid intent slot or version");
      if (next === undefined) throw Error("invalid stored intent");
      const candidate = next === null ? null : intent(next, maxBytes);
      return transaction<boolean>(true, (store, set, abort) => {
        const request = store.get(slot);
        request.onsuccess = () => {
          try {
            const current = intent(request.result, maxBytes);
            if ((current?.version ?? null) !== expectedVersion) { set(false); return; }
            const write = () => { if (candidate === null) store.delete(slot); else store.put(candidate, slot); set(true); };
            if (candidate !== null && current === null) {
              const count = store.count();
              count.onsuccess = () => { try { if (count.result >= maxSlots) throw Error("intent store slot limit"); write(); } catch (error) { abort(error); } };
            } else write();
          } catch (error) { abort(error); }
        };
      });
    },
  };
}
