import type { PendingIntentStore, StoredIntent } from 'rom-studio/recovery';

export async function openHostStorage(): Promise<{ intents: PendingIntentStore; readDraft(slot: string): Promise<string | null>; writeDraft(slot: string, text: string): Promise<void> }> {
  const database = await new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open('rom-recovery-host-fixture-v1', 1);
    request.onupgradeneeded = () => { request.result.createObjectStore('intents'); request.result.createObjectStore('editor-drafts'); };
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  function read<T>(store: string, slot: string): Promise<T | null> {
    return new Promise((resolve, reject) => {
      const transaction = database.transaction(store, 'readonly'), request = transaction.objectStore(store).get(slot);
      let value: T | null = null;
      request.onsuccess = () => { value = request.result ?? null; };
      transaction.oncomplete = () => resolve(value); transaction.onabort = () => reject(transaction.error);
    });
  }
  const intents: PendingIntentStore = {
    read: slot => read<StoredIntent>('intents', slot),
    compareExchange(slot, expected, next) {
      return new Promise((resolve, reject) => {
        const transaction = database.transaction('intents', 'readwrite', { durability: 'strict' }), store = transaction.objectStore('intents');
        let matched = false;
        const request = store.get(slot);
        request.onsuccess = () => {
          if ((request.result?.version ?? null) !== expected) return;
          matched = true;
          if (next === null) store.delete(slot); else store.put(next, slot);
        };
        transaction.oncomplete = () => resolve(matched); transaction.onabort = () => reject(transaction.error);
      });
    },
  };
  return {
    intents, readDraft: slot => read<string>('editor-drafts', slot),
    writeDraft(slot, text) {
      return new Promise((resolve, reject) => {
        const transaction = database.transaction('editor-drafts', 'readwrite', { durability: 'strict' });
        transaction.objectStore('editor-drafts').put(text, slot);
        transaction.oncomplete = () => resolve(); transaction.onabort = () => reject(transaction.error);
      });
    },
  };
}
