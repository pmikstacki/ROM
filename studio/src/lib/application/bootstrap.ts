import { openIndexedDbIntentStore } from "../recovery/indexeddb-store.ts";
import { principalValue } from "../recovery/record.ts";
import {
  bootstrapIdentifier,
  parseStudioBootstrap,
} from "./bootstrap-config.ts";
import type { StudioBootstrapConfig } from "./bootstrap-config.ts";
import type { StudioAuthProfile } from "./session-types.ts";
export { parseStudioBootstrap } from "./bootstrap-config.ts";
export type {
  StudioBootstrapConfig,
  StudioBootstrapStore,
} from "./bootstrap-config.ts";

/** Explicit host storage composition. A store failure never falls back to volatile mutation state. */
export async function createStudioBootstrap(
  input: StudioBootstrapConfig,
  environment: {
    openStore?: typeof openIndexedDbIntentStore;
    now?: () => number;
    uuid?: () => string;
  } = {},
) {
  const config = parseStudioBootstrap(JSON.stringify(input));
  const open = environment.openStore ?? openIndexedDbIntentStore;
  const now = environment.now ?? (() => Math.floor(Date.now() / 1000));
  const uuid = environment.uuid ?? (() => crypto.randomUUID());
  if ([open, now, uuid].some((value) => typeof value !== "function"))
    throw Error("Invalid Studio bootstrap environment.");
  const intentStore = await open(config.recovery.intentStore);
  try {
    const editorStore = await open(config.recovery.editorStore);
    let closed = false;
    function scope(
      principal: Parameters<StudioAuthProfile["recovery"]["slot"]>[0],
      record: "intent" | "editor",
      target: string[],
    ) {
      const owner = principalValue(principal);
      if (
        owner.authority !== config.authority ||
        !["intent", "editor"].includes(record)
      )
        throw Error("Invalid Studio bootstrap scope.");
      return bootstrapIdentifier(
        JSON.stringify([
          config.recovery.namespace,
          record,
          owner.authority,
          owner.kind,
          owner.subject,
          ...target.map(bootstrapIdentifier),
        ]),
      );
    }
    const profile: StudioAuthProfile = {
      authority: config.authority,
      now,
      recovery: {
        namespace: config.recovery.namespace,
        intentStore,
        editorStore,
        maxBytes: config.recovery.maxBytes,
        retryEpoch: () => BigInt(config.recovery.retryEpoch),
        newVersion: () => bootstrapIdentifier(uuid()),
        newCommandKey: () => bootstrapIdentifier(uuid()),
        slot(principal, target, record) {
          return scope(principal, record, [target.kind, target.id]);
        },
        creationSlot(principal, kind, record) {
          return scope(principal, record, [kind]);
        },
      },
    };
    return {
      profile,
      close() {
        if (closed) return;
        closed = true;
        try {
          intentStore.close();
        } finally {
          editorStore.close();
        }
      },
    };
  } catch (error) {
    intentStore.close();
    throw error;
  }
}
