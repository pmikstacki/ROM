import { parseWire, stringifyWire } from "../client/codec.ts";
import type { WireValue } from "../client/types.ts";
import {
  identifier,
  principalKey,
  principalValue,
} from "../recovery/record.ts";
import { createRecordStorage } from "../recovery/record-storage.ts";
import type {
  DurablePrincipal,
  PendingIntentStore,
} from "../recovery/types.ts";
import { restoreFormDraft } from "../resources/form-draft.ts";
import { copyEditorSnapshot } from "./snapshot.ts";
import type { EditorSnapshot } from "./editor-drafts.ts";

export interface CreationDraft {
  id: string;
  form: EditorSnapshot;
}
export interface CreationDraftState {
  status: "idle" | "writing" | "saved" | "error";
  snapshot: CreationDraft | null;
}
export { creationDraftMatches } from "./creation-proof.ts";
/** Local invalid input uses a form scope, not a synthetic Resource ID. */
export function createCreationDrafts(options: {
  namespace: string;
  kind: string;
  descriptor: string;
  maxBytes: number;
  principal(): DurablePrincipal | null;
  slot(principal: DurablePrincipal): string;
  store: PendingIntentStore;
  newVersion(): string;
  publish(state: CreationDraftState): void;
}) {
  const namespace = identifier(options.namespace),
    kind = identifier(options.kind),
    descriptor = options.descriptor,
    maxBytes = options.maxBytes;
  if (
    typeof descriptor !== "string" ||
    !descriptor ||
    !Number.isSafeInteger(maxBytes) ||
    maxBytes < 1
  )
    throw Error("invalid creation draft configuration");
  let owner = options.principal() ? principalValue(options.principal()) : null;
  let epoch = 0,
    latest = 0;
  let state: CreationDraftState = { status: "idle", snapshot: null };
  let storage: ReturnType<typeof createRecordStorage<CreationDraft>> | null =
    null;
  function copy(value: CreationDraft): CreationDraft {
    return { id: value.id, form: copyEditorSnapshot(value.form, maxBytes) };
  }
  const snapshot = (): CreationDraftState => {
    const current = options.principal();
    if (!owner || !current || principalKey(owner) !== principalKey(current))
      return { status: "idle", snapshot: null };
    return {
      status: state.status,
      snapshot: state.snapshot ? copy(state.snapshot) : null,
    };
  };
  const publish = () => options.publish(snapshot());
  function validate(value: unknown): CreationDraft {
    if (
      !value ||
      typeof value !== "object" ||
      Array.isArray(value) ||
      Object.keys(value).length !== 2 ||
      !Object.hasOwn(value, "id") ||
      !Object.hasOwn(value, "form")
    )
      throw Error("invalid creation draft");
    const input = value as CreationDraft;
    if (
      typeof input.id !== "string" ||
      !input.form ||
      input.form.baseRevision !== null
    )
      throw Error("invalid creation draft identity");
    if (
      Object.keys(input.form).some(
        (key) =>
          ![
            "mode",
            "descriptor",
            "baseRevision",
            "intents",
            "editors",
          ].includes(key),
      )
    )
      throw Error("invalid creation draft form");
    restoreFormDraft(
      input.form,
      { mode: "resource", descriptor, baseRevision: null },
      maxBytes,
    );
    return copy(input);
  }
  function check(ticket: number, principal: DurablePrincipal) {
    const current = options.principal();
    if (
      ticket !== epoch ||
      !owner ||
      !current ||
      principalKey(owner) !== principalKey(principal) ||
      principalKey(current) !== principalKey(principal)
    )
      throw Error("creation draft principal changed");
  }
  function envelope(payload: string, principal: DurablePrincipal) {
    const value = parseWire(payload, maxBytes);
    if (
      !value ||
      typeof value !== "object" ||
      Array.isArray(value) ||
      Object.keys(value).length !== 5 ||
      value.format !== "rom-creation-draft-v1" ||
      value.namespace !== namespace ||
      value.kind !== kind ||
      principalKey(principalValue(value.principal)) !== principalKey(principal)
    )
      throw Error("creation draft identity mismatch");
    return value;
  }
  function scope() {
    const current = options.principal();
    if (!owner || !current || principalKey(current) !== principalKey(owner))
      throw Error("creation draft principal unavailable");
    const principal = principalValue(owner),
      ticket = epoch;
    if (!storage)
      storage = createRecordStorage({
        store: options.store,
        slot: identifier(options.slot(principal)),
        newVersion: options.newVersion,
        encode(value: CreationDraft) {
          return stringifyWire(
            {
              format: "rom-creation-draft-v1",
              namespace,
              kind,
              principal,
              snapshot: validate(value),
            } as unknown as WireValue,
            maxBytes,
          );
        },
        decode(payload) {
          return validate(envelope(payload, principal).snapshot);
        },
      });
    return { principal, ticket, storage };
  }
  function fail(ticket: number) {
    if (ticket === epoch) {
      state = { ...state, status: "error" };
      publish();
    }
  }
  return {
    get state() {
      return snapshot();
    },
    guardNavigation() {
      if (state.status === "writing" || state.status === "error")
        throw Error("Resolve creation draft storage before navigation.");
    },
    async stage(value: CreationDraft) {
      const context = scope(),
        write = ++latest;
      let next: CreationDraft;
      try {
        next = validate(value);
        // Validate the whole envelope before publishing any successful local copy.
        stringifyWire(
          {
            format: "rom-creation-draft-v1",
            namespace,
            kind,
            principal: context.principal,
            snapshot: next,
          } as unknown as WireValue,
          maxBytes,
        );
      } catch (problem) {
        fail(context.ticket);
        throw problem;
      }
      state = { status: "writing", snapshot: next };
      publish();
      return context.storage.exclusive(async () => {
        try {
          check(context.ticket, context.principal);
          await context.storage.write(next);
          check(context.ticket, context.principal);
          if (write === latest) {
            state = { status: "saved", snapshot: next };
            publish();
          }
        } catch (problem) {
          fail(context.ticket);
          throw problem;
        }
      });
    },
    async restore() {
      if (!options.principal()) return;
      if (state.status === "writing")
        throw Error("Creation draft write pending.");
      const context = scope(),
        write = ++latest;
      state = { status: "writing", snapshot: null };
      publish();
      return context.storage.exclusive(async () => {
        try {
          check(context.ticket, context.principal);
          const next = await context.storage.read();
          check(context.ticket, context.principal);
          if (write !== latest)
            throw Error("creation draft changed during restore");
          state = { status: next ? "saved" : "idle", snapshot: next };
          publish();
        } catch (problem) {
          fail(context.ticket);
          throw problem;
        }
      });
    },
    async discard(expected?: CreationDraft) {
      if (state.status === "writing")
        throw Error("Creation draft write pending.");
      const context = scope();
      const match = expected ? validate(expected) : null;
      return context.storage.exclusive(async () => {
        check(context.ticket, context.principal);
        if (
          match &&
          (!state.snapshot ||
            stringifyWire(match as unknown as WireValue, maxBytes) !==
              stringifyWire(state.snapshot as unknown as WireValue, maxBytes))
        )
          return false;
        const write = ++latest;
        state = { ...state, status: "writing" };
        publish();
        try {
          if (match) await context.storage.write(null);
          else
            await context.storage.remove((payload) => {
              check(context.ticket, context.principal);
              if (payload !== null) envelope(payload, context.principal);
            });
          check(context.ticket, context.principal);
          if (write === latest) {
            state = { status: "idle", snapshot: null };
            publish();
          }
          return true;
        } catch (problem) {
          fail(context.ticket);
          throw problem;
        }
      });
    },
    rebind(principal: DurablePrincipal | null) {
      const same =
        owner && principal && principalKey(owner) === principalKey(principal);
      epoch++;
      latest++;
      owner = principal ? principalValue(principal) : null;
      if (!same) {
        storage = null;
        state = { status: "idle", snapshot: null };
        publish();
      } else if (state.status === "writing") {
        state = { ...state, status: "error" };
        publish();
      }
    },
  };
}
