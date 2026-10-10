import { parseWire, stringifyWire } from "../client/codec.ts";
import { copyEditorSnapshot, copyEditorState } from "./snapshot.ts";
import type { WireObject, WireValue } from "../client/types.ts";
import type { DurablePrincipal, RecoveryBinding } from "../recovery/types.ts";
import {
  identifier,
  principalKey,
  principalValue,
  revision,
} from "../recovery/record.ts";
import type { EditorDraft } from "../renderers/editor-draft.ts";
import type {
  ManagedApplicationOptions,
  ResourceTarget,
} from "./session-types.ts";

export interface EditorSnapshot {
  mode: "resource" | "action";
  descriptor: string;
  baseRevision: bigint | null;
  intents: WireObject;
  editors: Record<string, EditorDraft>;
}
export interface EditorPersistenceState {
  status: "idle" | "writing" | "saved" | "error";
  target: ResourceTarget | null;
  snapshot: EditorSnapshot | null;
}
/** Invalid editor text is stored separately from accepted mutation wire. Hosts exclude credentials. */
export function createEditorDrafts(options: {
  recovery: ManagedApplicationOptions["recovery"];
  binding(): RecoveryBinding;
  publish(state: EditorPersistenceState): void;
}) {
  let owner: DurablePrincipal | null = options.binding().principal;
  let epoch = 0,
    pendingWrites = 0,
    latestWrite = 0,
    version: string | null = null,
    slot: string | null = null;
  let state: EditorPersistenceState = {
    status: "idle",
    target: null,
    snapshot: null,
  };
  let tail = Promise.resolve();
  const copy = () => copyEditorState(state, options.recovery.maxBytes);
  function publish() {
    options.publish(copy());
  }
  function check(ticket: number, principal: DurablePrincipal) {
    if (
      ticket !== epoch ||
      !owner ||
      principalKey(owner) !== principalKey(principal)
    )
      throw Error("editor principal changed");
  }
  function scope(target: ResourceTarget) {
    const principal = options.binding().principal;
    if (!principal || !owner || principalKey(principal) !== principalKey(owner))
      throw Error("editor principal unavailable");
    const next = { kind: identifier(target.kind), id: identifier(target.id) };
    const nextSlot = identifier(
      options.recovery.slot(principal, next, "editor"),
    );
    if (slot !== nextSlot) {
      if (state.status === "writing" || state.status === "error")
        throw Error("Resolve editor draft storage before navigation.");
      slot = nextSlot;
      version = null;
    }
    return {
      ticket: epoch,
      principal: principalValue(principal),
      target: next,
      slot: nextSlot,
    };
  }
  function serial<T>(operation: () => Promise<T>): Promise<T> {
    const next = tail.then(operation);
    tail = next.then(
      () => {},
      () => {},
    );
    return next;
  }
  function encode(snapshot: EditorSnapshot, context: ReturnType<typeof scope>) {
    const object = (value: unknown) =>
      !!value &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      [null, Object.prototype].includes(Object.getPrototypeOf(value));
    if (
      !object(snapshot) ||
      Object.keys(snapshot).some(
        (key) =>
          ![
            "mode",
            "descriptor",
            "baseRevision",
            "intents",
            "editors",
          ].includes(key),
      ) ||
      !object(snapshot.intents) ||
      !object(snapshot.editors)
    )
      throw Error("invalid editor snapshot");
    if (
      !["resource", "action"].includes(snapshot.mode) ||
      typeof snapshot.descriptor !== "string" ||
      !snapshot.descriptor
    )
      throw Error("invalid editor snapshot");
    revision(snapshot.baseRevision);
    return stringifyWire(
      {
        format: "rom-editor-draft-v1",
        namespace: options.recovery.namespace,
        principal: context.principal,
        target: context.target,
        ...snapshot,
      } as unknown as WireValue,
      options.recovery.maxBytes,
    );
  }
  return {
    get state() {
      return copy();
    },
    guardNavigation() {
      if (state.status === "writing" || state.status === "error")
        throw Error("Resolve editor draft storage before navigation.");
    },
    refuse(target: ResourceTarget) {
      const context = scope(target);
      check(context.ticket, context.principal);
      latestWrite++;
      state = { ...state, target: context.target, status: "error" };
      publish();
    },
    stage(target: ResourceTarget, snapshot: EditorSnapshot) {
      const context = scope(target);
      const write = ++latestWrite;
      let payload: string;
      try {
        payload = encode(snapshot, context);
      } catch (problem) {
        state = { ...state, status: "error", target: context.target };
        publish();
        return Promise.reject(problem);
      }
      const nextSnapshot = copyEditorSnapshot(
        snapshot,
        options.recovery.maxBytes,
      );
      state = {
        status: "writing",
        target: context.target,
        snapshot: nextSnapshot,
      };
      pendingWrites++;
      const operation = serial(async () => {
        check(context.ticket, context.principal);
        try {
          const nextVersion = identifier(options.recovery.newVersion());
          if (nextVersion === version)
            throw Error("editor storage version reused");
          if (
            !(await options.recovery.editorStore.compareExchange(
              context.slot,
              version,
              { version: nextVersion, payload },
            ))
          )
            throw Error("editor storage conflict");
          check(context.ticket, context.principal);
          version = nextVersion;
          if (write === latestWrite) {
            state = {
              status: "saved",
              target: context.target,
              snapshot: nextSnapshot,
            };
            publish();
          }
        } catch (problem) {
          if (context.ticket === epoch) {
            state = { ...state, status: "error" };
            publish();
          }
          throw problem;
        }
      }).finally(() => {
        pendingWrites--;
      });
      publish();
      return operation;
    },
    restore(target: ResourceTarget) {
      if (pendingWrites > 0 || state.status === "writing")
        return Promise.reject(
          Error("Editor draft write pending; restore is unavailable."),
        );
      const context = scope(target);
      const write = ++latestWrite;
      state = { status: "writing", target: context.target, snapshot: null };
      const operation = serial(async () => {
        check(context.ticket, context.principal);
        try {
          const stored = await options.recovery.editorStore.read(context.slot);
          check(context.ticket, context.principal);
          if (!stored) {
            version = null;
            if (write !== latestWrite)
              throw Error("editor draft changed during restore");
            state = { status: "idle", target: context.target, snapshot: null };
            publish();
            return;
          }
          const value = parseWire(
            stored.payload,
            options.recovery.maxBytes,
          ) as WireObject;
          if (
            !value ||
            typeof value !== "object" ||
            Array.isArray(value) ||
            Object.keys(value).some(
              (key) =>
                ![
                  "format",
                  "namespace",
                  "principal",
                  "target",
                  "mode",
                  "descriptor",
                  "baseRevision",
                  "intents",
                  "editors",
                ].includes(key),
            )
          )
            throw Error("invalid editor record");
          if (
            !value ||
            value.format !== "rom-editor-draft-v1" ||
            value.namespace !== options.recovery.namespace ||
            principalKey(principalValue(value.principal)) !==
              principalKey(context.principal)
          )
            throw Error("editor record owner mismatch");
          const recordTarget = value.target as WireObject;
          if (
            recordTarget?.kind !== context.target.kind ||
            recordTarget?.id !== context.target.id
          )
            throw Error("editor record target mismatch");
          const snapshot = {
            mode: value.mode,
            descriptor: value.descriptor,
            baseRevision: revision(value.baseRevision),
            intents: value.intents,
            editors: value.editors,
          } as unknown as EditorSnapshot;
          encode(snapshot, context);
          version = identifier(stored.version);
          if (write !== latestWrite)
            throw Error("editor draft changed during restore");
          state = { status: "saved", target: context.target, snapshot };
          publish();
        } catch (problem) {
          if (context.ticket === epoch && write === latestWrite) {
            state = { status: "error", target: context.target, snapshot: null };
            publish();
          }
          throw problem;
        }
      });
      publish();
      return operation;
    },
    rebind(principal: DurablePrincipal | null) {
      const same =
        owner && principal && principalKey(owner) === principalKey(principal);
      epoch++;
      latestWrite++;
      owner = principal ? principalValue(principal) : null;
      if (!same) {
        version = null;
        slot = null;
        state = { status: "idle", target: null, snapshot: null };
        publish();
      } else if (state.status === "writing") {
        state = { ...state, status: "error" };
        publish();
      }
    },
  };
}
