/** Host-selected pending intent recovery; stored bytes confer no server authority. */
export { createMutationRecovery } from "./lib/recovery/controller.ts";
export { editorFingerprint } from "./lib/recovery/editor-fingerprint.ts";
export { restoreMutationRecovery } from "./lib/recovery/restore.ts";
export type { RestoreMutationRecoveryOptions } from "./lib/recovery/restore.ts";
export { openIndexedDbIntentStore } from "./lib/recovery/indexeddb-store.ts";
export type { IndexedDbIntentStore, IndexedDbIntentStoreOptions } from "./lib/recovery/indexeddb-store.ts";
export type * from "./lib/recovery/types.ts";
