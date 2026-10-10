import { createClient, RemoteError, stringifyWire } from "rom-studio/client";
import type {
  RomClient,
  ProjectedView,
  Operation,
  WireValue,
  WireObject,
  ResourceDescriptor,
} from "rom-studio/client";
import { createObservation } from "rom-studio/observe";
import type { Observation, ObservationPhase } from "rom-studio/observe";
import {
  createMutationRecovery,
  openIndexedDbIntentStore,
} from "rom-studio/recovery";
import type {
  DurablePrincipal,
  IndexedDbIntentStore,
  MutationRecovery,
  MutationRecoveryState,
} from "rom-studio/recovery";
import {
  createSelection,
  createLatestRequest,
  captureExportSnapshot,
} from "rom-studio/ui";
import type {
  HistoryEntry,
  ReferenceLookup,
  CompositionCommandResult,
} from "rom-studio/ui/components";
import type { LayoutItem } from "rom-studio/ui";
import { cloneView, field, kinds, replacement } from "./model.ts";
import type { PortalKind } from "./model.ts";

export interface PortalState {
  subject: "alice" | "bob" | "guest" | "public-guest";
  generation: number;
  ready: boolean;
  error: string;
  lookup: ReferenceLookup | null;
  rows: Record<PortalKind, readonly ProjectedView[]>;
  phases: Record<PortalKind, ObservationPhase>;
  selected: readonly string[];
  history: readonly HistoryEntry[];
  selectedHistory: string | null;
  selectedHistoryRevision: string;
  exportText: string;
  exportPhase: string;
  recovery: Record<string, MutationRecoveryState>;
  stored: Record<string, boolean>;
}
export function createPortalHost(
  publish: (state: PortalState) => void,
  initialSubject: PortalState["subject"] = "alice",
) {
  let state: PortalState = {
    subject: initialSubject,
    generation: 0,
    ready: false,
    error: "",
    lookup: null,
    rows: {
      equipment: [],
      inspections: [],
      "work-orders": [],
      "portal-settings": [],
    },
    phases: {
      equipment: "idle",
      inspections: "idle",
      "work-orders": "idle",
      "portal-settings": "idle",
    },
    selected: [],
    history: [],
    selectedHistory: null,
    selectedHistoryRevision: "",
    exportText: "",
    exportPhase: "idle",
    recovery: {},
    stored: {},
  };
  let client: RomClient,
    store: IndexedDbIntentStore | undefined,
    disposed = false;
  let storeOpening: Promise<IndexedDbIntentStore> | undefined;
  let descriptors: ResourceDescriptor[] = [];
  let observations: Observation<ProjectedView>[] = [];
  const lanes = new Map<string, MutationRecovery>();
  let selection = createSelection({ ids: [], initial: [], multiple: true });
  let historyViews = new Map<string, ProjectedView>();
  const emit = (patch: Partial<PortalState> = {}) => {
    if (!disposed) {
      state = { ...state, ...patch };
      publish(state);
    }
  };
  const principal = (): DurablePrincipal | null =>
    state.subject === "guest" || state.subject === "public-guest"
      ? null
      : {
          authority: "maintenance-portal",
          kind: "human",
          subject: state.subject,
        };
  const owns = (ticket: number) => !disposed && state.generation === ticket;
  const exportRequest = createLatestRequest({
    clone: (text: string) => text,
    classifyError: () => "Export unavailable",
    onState: (next) =>
      emit({ exportText: next.value ?? "", exportPhase: next.phase }),
  });
  function newClient() {
    const credential =
      state.subject === "alice"
        ? "fixture-alice"
        : state.subject === "bob"
          ? "fixture-bob"
          : "unverified";
    return createClient({
      base: "/api",
      timeoutMs: 5000,
      maxRows: 50,
      maxBytes: 131072,
      maxObservations: 4,
      fetch: (input, init) =>
        fetch(input, {
          ...init,
          headers: {
            ...Object.fromEntries(new Headers(init?.headers)),
            authorization: "Bearer " + credential,
          },
        }),
    });
  }
  function slot(kind: string, id: string) {
    return JSON.stringify(["maintenance-portal", state.subject, kind, id]);
  }
  function update(view: ProjectedView) {
    const kind = view.key.kind as PortalKind;
    if (!kinds.includes(kind)) throw Error("Unexpected Resource kind");
    const rows = state.rows[kind].filter((item) => item.key.id !== view.key.id);
    if (view.value) rows.push(cloneView(view));
    emit({ rows: { ...state.rows, [kind]: rows } });
  }
  function lane(kind: string, id: string) {
    const key = kind + "/" + id;
    if (!store || !principal())
      throw Error("Private write authority unavailable");
    if (!lanes.has(key)) {
      const ticket = state.generation;
      const controller = createMutationRecovery({
        namespace: "maintenance-portal-fixture-v1",
        slot: slot(kind, id),
        target: { kind, id },
        binding: { client, principal: principal() },
        store,
        newVersion: () => crypto.randomUUID(),
        maxBytes: 65536,
      });
      lanes.set(key, controller);
      controller.subscribe((next) => {
        if (owns(ticket))
          emit({ recovery: { ...state.recovery, [key]: next } });
      });
    }
    return lanes.get(key)!;
  }
  async function checkStored() {
    if (!store || !principal()) return;
    const ticket = state.generation,
      checks: Record<string, boolean> = {};
    for (const [kind, id] of [
      ["inspections", "check-1"],
      ["work-orders", "repair-1"],
      ["portal-settings", "workspace"],
    ]) {
      checks[kind + "/" + id] = Boolean(await store.read(slot(kind, id)));
      if (!owns(ticket)) return;
    }
    emit({ stored: checks });
  }
  async function initialize() {
    const ticket = state.generation;
    if (state.subject === "public-guest") {
      emit({ ready: true, error: "" });
      return;
    }
    client = newClient();
    try {
      if (!store && principal()) {
        storeOpening ??= openIndexedDbIntentStore({
          name: "rom-maintenance-portal-fixture-v1",
          maxBytes: 131072,
          maxSlots: 30,
        });
        store = await storeOpening;
        if (!owns(ticket)) {
          if (disposed) store.close();
          return;
        }
      }
      descriptors = (await client.discover()).resources;
      if (!owns(ticket)) return;
      emit({ lookup });
      const capturedClient = client,
        owner = principal();
      if (!owner) throw Error("Private observation authority unavailable");
      observations = kinds.map((kind) => {
        const observation = createObservation({
          scope: { kind: "principal", principal: owner, generation: ticket },
          source: (signal) =>
            capturedClient.observe(kind, { limit: 50 }, signal),
          clone: cloneView,
          measure: (rows) =>
            new TextEncoder().encode(
              stringifyWire(rows as unknown as WireValue),
            ).length,
          limits: { maxRows: 50, maxBytes: 131072 },
          retry: {
            maxAttempts: 3,
            maxElapsedMs: 10000,
            delayMs: 100,
            clock: () => performance.now(),
          },
          classifyError: (error) =>
            error instanceof RemoteError &&
            (error.status === 401 || error.status === 403)
              ? { kind: "denied", code: "Authority denied" }
              : { kind: "transient", code: "Connection unavailable" },
          onAuthorityLost: () => {
            if (owns(ticket)) clearPrivate("Authority denied");
          },
        });
        observation.subscribe((next) => {
          if (!owns(ticket)) return;
          if (kind === "equipment") {
            selection.replaceAvailable(next.rows.map((row) => row.key.id));
          }
          emit({
            rows: { ...state.rows, [kind]: next.rows },
            phases: { ...state.phases, [kind]: next.phase },
            selected: selection.selected,
            ready: true,
          });
        });
        observation.start();
        observation.setVisible(document.visibilityState !== "hidden");
        return observation;
      });
      await checkStored();
    } catch {
      if (owns(ticket)) emit({ ready: true, error: "Workspace unavailable" });
    }
  }
  function clearPrivate(error = "") {
    state.generation++;
    client?.invalidateSession();
    observations.forEach((item) => item.dispose());
    observations = [];
    lanes.forEach((item) => item.dispose());
    lanes.clear();
    descriptors = [];
    historyViews.clear();
    selection.clear();
    exportRequest.clear();
    emit({
      ready: false,
      error,
      lookup: null,
      rows: {
        equipment: [],
        inspections: [],
        "work-orders": [],
        "portal-settings": [],
      },
      phases: {
        equipment: "idle",
        inspections: "idle",
        "work-orders": "idle",
        "portal-settings": "idle",
      },
      selected: [],
      history: [],
      selectedHistory: null,
      selectedHistoryRevision: "",
      recovery: {},
      stored: {},
      exportText: "",
    });
  }
  async function commit(
    kind: string,
    id: string,
    operation: Operation,
  ): Promise<CompositionCommandResult> {
    const ticket = state.generation,
      key = kind + "/" + id;
    const view = state.rows[kind as PortalKind].find(
      (row) => row.key.id === id,
    );
    if (!view || state.stored[key]) return "unknown";
    let recovery: MutationRecovery;
    try {
      recovery = lane(kind, id);
      if (recovery.state.hasUnresolvedIntent) return "unknown";
      await recovery.stage(operation);
      if (!owns(ticket)) return "unknown";
      await recovery.begin({
        expected: view.revision,
        idempotency: crypto.randomUUID(),
      });
      const confirmed = await recovery.retry();
      if (!owns(ticket)) return "unknown";
      update(confirmed);
      await recovery.discard({ acknowledgePossibleCommit: true });
      // Keep the confirmed success label while the persistent slot is cleared.
      emit({
        recovery: {
          ...state.recovery,
          [key]: {
            ...recovery.state,
            phase: "succeeded",
            commitKnowledge: "committed",
            result: confirmed,
          },
        },
      });
      return "accepted";
    } catch {
      if (!owns(ticket)) return "unknown";
      const current = lanes.get(key)?.state;
      emit({
        error:
          current?.phase === "unknown"
            ? "Outcome unknown. Restore and retry the original intent."
            : "Save was not confirmed.",
      });
      return current?.hasUnresolvedIntent ? "unknown" : "rejected";
    }
  }
  const lookup: ReferenceLookup = {
    descriptor: (kind) => descriptors.find((item) => item.kind === kind),
    async lookup(kind, search, signal) {
      const ticket = state.generation,
        current = client;
      try {
        const rows = await current.query(kind, { limit: 20 }, signal);
        if (!owns(ticket))
          return { status: "cancelled", message: "Authority changed" };
        const candidates = rows
          .map((row) => ({
            id: row.key.id,
            title: field(row, "title") || row.key.id,
          }))
          .filter(
            (row) =>
              row.title
                .toLocaleLowerCase("en-GB")
                .includes(search.toLocaleLowerCase("en-GB")) ||
              row.id.includes(search),
          );
        return { status: "ready", candidates, limited: rows.length === 20 };
      } catch {
        return {
          status: "unavailable",
          message: "Authorized candidates unavailable",
        };
      }
    },
  };
  return {
    get state() {
      return state;
    },
    lookup,
    async start() {
      await initialize();
    },
    async switchSubject(subject: PortalState["subject"]) {
      clearPrivate();
      emit({ subject });
      await initialize();
    },
    toggle(id: string): CompositionCommandResult {
      selection.toggle(id);
      exportRequest.clear();
      emit({ selected: selection.selected });
      return "accepted";
    },
    visible(next: boolean) {
      observations.forEach((item) => item.setVisible(next));
    },
    async saveInspection(notes: string, date: string, equipment: WireValue) {
      if (
        notes.length > 4096 ||
        typeof equipment !== "string" ||
        equipment.length > 1024 ||
        date.length > 128
      )
        return "rejected";
      const view = state.rows.inspections.find(
        (row) => row.key.id === "check-1",
      );
      if (!view) return "rejected";
      return commit("inspections", "check-1", {
        type: "replace",
        input: replacement(view, { notes, inspected_at: date, equipment }),
      });
    },
    async saveLayout(layout: readonly LayoutItem[]) {
      const view = state.rows["portal-settings"].find(
        (row) => row.key.id === "workspace",
      );
      if (!view) return "rejected";
      const history = layout.find((item) => item.id === "history");
      return commit("portal-settings", "workspace", {
        type: "replace",
        input: replacement(view, {
          layout: JSON.stringify(layout),
          show_history: history?.visible ?? false,
        }),
      });
    },
    completeWork() {
      return commit("work-orders", "repair-1", {
        type: "action",
        input: { name: "complete", input: null },
      });
    },
    async restore(kind: string, id: string) {
      const ticket = state.generation,
        subject = state.subject;
      try {
        const controller = lane(kind, id);
        await controller.restore();
        if (owns(ticket) && state.subject === subject)
          emit({
            stored: { ...state.stored, [kind + "/" + id]: false },
            error: "",
          });
      } catch {
        if (owns(ticket) && state.subject === subject)
          emit({
            error:
              kind === "inspections"
                ? "Pending inspection restore unavailable"
                : kind === "work-orders"
                  ? "Pending work order restore unavailable"
                  : "Pending Settings restore unavailable",
          });
      }
    },
    async retry(kind: string, id: string) {
      const ticket = state.generation,
        controller = lane(kind, id);
      try {
        const result = await controller.retry();
        if (!owns(ticket)) return;
        update(result);
        await controller.discard({ acknowledgePossibleCommit: true });
        emit({
          recovery: {
            ...state.recovery,
            [kind + "/" + id]: {
              ...controller.state,
              phase: "succeeded",
              result,
              commitKnowledge: "committed",
            },
          },
          error: "",
        });
      } catch {
        if (owns(ticket)) emit({ error: "Original intent still unresolved" });
      }
    },
    async history() {
      const ticket = state.generation,
        subject = state.subject;
      try {
        const result = await client.journal("inspections");
        if (!owns(ticket) || state.subject !== subject) return;
        const batch = result as WireObject;
        const entries: HistoryEntry[] = [];
        const views = new Map<string, ProjectedView>();
        for (const event of batch.events as WireObject[]) {
          const view = event.view as unknown as ProjectedView;
          const id = String(event.position);
          const instant = Date.parse(field(view, "inspected_at"));
          if (!Number.isFinite(instant)) continue;
          entries.push({
            id,
            title: `Inspection ${view.key.id} · revision ${view.revision}`,
            instant,
          });
          views.set(id, cloneView(view));
        }
        historyViews = views;
        emit({ history: entries, error: "" });
      } catch {
        if (owns(ticket) && state.subject === subject)
          emit({ error: "Inspection history unavailable" });
      }
    },
    selectHistory(id: string) {
      const view = historyViews.get(id);
      if (!view) throw Error("History unavailable");
      emit({
        selectedHistory: id,
        selectedHistoryRevision: String(view.revision),
      });
    },
    async exportInspection() {
      const view = state.rows.inspections.find(
        (row) => row.key.id === "check-1",
      );
      if (!view) return;
      const ticket = state.generation,
        current = client;
      const capturedSelection = [...state.selected];
      const snapshot = captureExportSnapshot({
        identity: {
          principal: principal(),
          authorityTicket: String(ticket),
          resource: {
            kind: view.key.kind,
            id: view.key.id,
            revision: view.revision,
          },
          selectedDate: field(view, "inspected_at"),
          format: "application/json",
          locale: "en-GB",
        },
        value: view,
        clone: cloneView,
      });
      await exportRequest.run(
        JSON.stringify([
          ticket,
          view.key.id,
          String(view.revision),
          snapshot.identity.selectedDate,
        ]),
        async (signal) => {
          await current.read(view.key.kind, view.key.id, signal);
          if (!owns(ticket)) throw Error("Authority changed");
          return stringifyWire({
            identity: snapshot.identity,
            value: snapshot.value,
            equipmentSelection: capturedSelection,
          } as unknown as WireValue);
        },
      );
    },
    clearExport() {
      exportRequest.clear();
    },
    dispose() {
      if (disposed) return;
      clearPrivate();
      disposed = true;
      exportRequest.dispose();
      store?.close();
    },
  };
}
export type PortalHost = ReturnType<typeof createPortalHost>;
