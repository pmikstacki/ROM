import { identifier } from "./record.ts";
import type { PendingIntentStore } from "./types.ts";

/** Serialized CAS ownership shared by local drafts and durable mutation records. */
export function createRecordStorage<T>(options: {
  store: PendingIntentStore;
  slot: string;
  newVersion(): string;
  decode(payload: string): T;
  encode(value: T): string;
}) {
  const slot = identifier(options.slot);
  let version: string | null = null;
  let tail = Promise.resolve();
  return {
    exclusive<R>(operation: () => Promise<R>): Promise<R> {
      const current = tail.then(operation);
      tail = current.then(
        () => {},
        () => {},
      );
      return current;
    },
    async read(): Promise<T | null> {
      const stored = await options.store.read(slot);
      if (!stored) {
        version = null;
        return null;
      }
      identifier(stored.version);
      const record = options.decode(stored.payload);
      version = stored.version;
      return record;
    },
    async write(next: T | null) {
      const nextVersion = next ? identifier(options.newVersion()) : null;
      if (nextVersion !== null && nextVersion === version)
        throw Error("recovery version reused");
      const payload = next ? options.encode(next) : null;
      if (
        !(await options.store.compareExchange(
          slot,
          version,
          payload === null ? null : { version: nextVersion!, payload },
        ))
      )
        throw Error("recovery storage conflict");
      version = nextVersion;
    },
    /** Explicit local deletion validates scope independently of a changed value schema. */
    async remove(validate: (payload: string | null) => void) {
      const stored = await options.store.read(slot);
      const observed = stored ? identifier(stored.version) : null;
      validate(stored?.payload ?? null);
      if (!(await options.store.compareExchange(slot, observed, null)))
        throw Error("recovery storage conflict");
      version = null;
    },
  };
}
