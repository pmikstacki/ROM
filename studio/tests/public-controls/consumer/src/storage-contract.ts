import { openIndexedDbIntentStore } from "rom-studio/recovery";

/** Actual browser database, selected explicitly by this disposable consumer. */
export async function publicStorageContract(): Promise<void> {
  await openDeadlineContract();
  const name = `installed-intents-${crypto.randomUUID()}`;
  const options = { name, maxBytes: 1024, maxSlots: 2, timeoutMs: 1000 };
  const first = await openIndexedDbIntentStore(options);
  const second = await openIndexedDbIntentStore(options);
  const value = { version: "one", payload: '{"integer":9007199254740993,"open":false,"code":null}' };
  try {
    const winners = await Promise.all([
      first.compareExchange("alice/note", null, value),
      second.compareExchange("alice/note", null, { version: "other", payload: "other" }),
    ]);
    if (winners.filter(Boolean).length !== 1) throw Error("IndexedDB CAS has multiple winners");
    const saved = await second.read("alice/note");
    if (!saved) throw Error("Committed intent missing");
    if (await first.compareExchange("alice/note", "wrong", null)) throw Error("Stale CAS changed intent");
    if (!(await first.compareExchange("alice/note", saved.version, value))) throw Error("Current CAS rejected");
    let invalid = false;
    try { await Reflect.apply(first.compareExchange, first, ["alice/note", "one", undefined]); } catch { invalid = true; }
    if (!invalid || (await first.read("alice/note"))?.version !== "one") throw Error("Undefined write deleted accepted intent");
    if (!(await first.compareExchange("alice/editor", null, { version: "draft", payload: "invalid raw text" }))) throw Error("Editor CAS rejected");
    let bounded = false;
    try { await first.compareExchange("extra", null, value); } catch { bounded = true; }
    if (!bounded || await first.read("extra") !== null) throw Error("Slot bound did not reject without write");
    bounded = false;
    try { await first.compareExchange("alice/note", "one", { version: "too-big", payload: "x".repeat(1024) }); } catch { bounded = true; }
    if (!bounded || (await second.read("alice/note"))?.version !== "one") throw Error("Byte bound changed accepted intent");
    first.close(); second.close();
    const reopened = await openIndexedDbIntentStore(options);
    try {
      if ((await reopened.read("alice/note"))?.payload !== value.payload) throw Error("Reopen changed exact intent bytes");
      if (!(await reopened.compareExchange("alice/editor", "draft", null))) throw Error("Delete CAS failed");
      if (!(await reopened.compareExchange("new-target", null, value))) throw Error("Deleted slot still consumes capacity");
      const pending = reopened.compareExchange("alice/note", "one", { version: "aborted", payload: "must not commit" });
      reopened.close();
      let aborted = false;
      try { await pending; } catch { aborted = true; }
      if (!aborted) throw Error("Close acknowledged an aborted write");
    } finally { reopened.close(); }
    const afterAbort = await openIndexedDbIntentStore(options);
    if ((await afterAbort.read("alice/note"))?.version !== "one") throw Error("Aborted write changed accepted intent");
    const upgraded = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open(name, 2);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(Error("Store did not close on version change"));
    });
    try {
      let invalidated = false;
      try { await afterAbort.read("alice/note"); } catch { invalidated = true; }
      if (!invalidated) throw Error("Version change did not invalidate the old store");
    } finally { afterAbort.close(); upgraded.close(); }
    let closed = false;
    try { await first.read("alice/note"); } catch { closed = true; }
    if (!closed) throw Error("Closed store accepted new work");
  } finally { first.close(); second.close(); }
  const corruptName = `corrupt-intents-${crypto.randomUUID()}`;
  const raw = await new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(corruptName, 1);
    request.onupgradeneeded = () => request.result.createObjectStore("records");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = raw.transaction("records", "readwrite");
      transaction.objectStore("records").put(null, "corrupt");
      transaction.oncomplete = () => resolve();
      transaction.onabort = () => reject(transaction.error);
    });
  } finally { raw.close(); }
  const corrupt = await openIndexedDbIntentStore({ ...options, name: corruptName });
  try {
    let rejected = false;
    try { await corrupt.read("corrupt"); } catch { rejected = true; }
    if (!rejected) throw Error("Malformed stored null was accepted as a missing record");
  } finally { corrupt.close(); }
}

/** Hold a real initial upgrade, then verify a timed-out queued open leaves no connection. */
async function openDeadlineContract(): Promise<void> {
  const name = `queued-intents-${crypto.randomUUID()}`;
  let release = false;
  let announce!: () => void;
  const started = new Promise<void>(resolve => { announce = resolve; });
  const raw = new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(name, 1);
    request.onupgradeneeded = () => {
      const store = request.result.createObjectStore("records");
      const stop = performance.now() + 2000;
      const keepActive = () => {
        if (release) return;
        if (performance.now() >= stop) { request.transaction?.abort(); return; }
        store.get("keep-active").onsuccess = keepActive;
      };
      keepActive(); announce();
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  // Observe rejection even if fixture setup itself fails.
  void raw.catch(() => {});
  await started;
  let timedOut = false;
  try {
    await openIndexedDbIntentStore({ name, maxBytes: 1024, maxSlots: 2, timeoutMs: 20 });
  } catch (error) { timedOut = error instanceof Error && error.message === "intent store open timeout"; }
  finally { release = true; }
  const database = await raw;
  database.close();
  if (!timedOut) throw Error("Queued database open did not respect its deadline");
  const upgraded = await new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(name, 2);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(Error("Timed-out late open leaked a blocking connection"));
  });
  upgraded.close();
}
