import { createClient, RemoteError, stringifyWire } from "rom-studio/client";
import type { ProjectedView, WireValue } from "rom-studio/client";
import { createObservation } from "rom-studio/observe";
import type { Observation, ObservationPhase } from "rom-studio/observe";
import {
  createSelection,
  createLatestRequest,
  captureExportSnapshot,
} from "rom-studio/ui";
import type { CompositionCommandResult } from "rom-studio/ui/components";
import { cloneView, field } from "./model.ts";

export interface PublicGuideState {
  rows: readonly ProjectedView[];
  selected: readonly string[];
  phase: ObservationPhase;
  sum: string;
  exportText: string;
  exportPhase: string;
  error: string;
}
const kind = "maintenance-guides",
  maxU64 = (1n << 64n) - 1n;
function nominal(view: ProjectedView) {
  const value = view.value?.nominal_voltage;
  const number =
    typeof value === "bigint"
      ? value
      : typeof value === "number" && Number.isSafeInteger(value)
        ? BigInt(value)
        : -1n;
  if (number < 0n || number > maxU64) throw Error("Invalid nominal value");
  return number;
}
function guide(view: ProjectedView) {
  if (
    view.key.kind !== kind ||
    !view.value ||
    Object.keys(view.value).some(
      (key) => !["title", "nominal_voltage", "updated_at"].includes(key),
    ) ||
    !field(view, "title") ||
    field(view, "title").length > 1024 ||
    !field(view, "updated_at") ||
    field(view, "updated_at").length > 128
  )
    throw Error("Invalid public guide");
  nominal(view);
  return cloneView(view);
}
function sum(rows: readonly ProjectedView[]) {
  if (!rows.length || rows.length > 20)
    throw Error("Select between one and twenty guides");
  return rows.reduce((total, view) => total + nominal(view), 0n);
}

/** Application-owned explicit public scope and arithmetic; no mutation controller or storage. */
export function createPublicGuideHost(
  generation: number,
  publish: (state: PublicGuideState) => void,
) {
  let state: PublicGuideState = {
    rows: [],
    selected: [],
    phase: "idle",
    sum: "",
    exportText: "",
    exportPhase: "idle",
    error: "",
  };
  let disposed = false,
    epoch = 0,
    exportedContext = "";
  let observation: Observation<ProjectedView> | undefined;
  const startAbort = new AbortController();
  const client = createClient({
    base: "/api",
    maxRows: 20,
    maxBytes: 65536,
    maxObservations: 1,
    timeoutMs: 5000,
  });
  const selection = createSelection({ ids: [], initial: [], multiple: true });
  const context = () => `public-guides:${generation}:${epoch}`;
  const emit = (patch: Partial<PublicGuideState>) => {
    if (!disposed) {
      state = { ...state, ...patch };
      publish(state);
    }
  };
  const current = (ticket: string) => !disposed && context() === ticket;
  const request = createLatestRequest({
    clone: (text: string) => text,
    classifyError: () => "Public export unavailable",
    onState: (next) =>
      emit({ exportText: next.value ?? "", exportPhase: next.phase }),
  });
  function clearResult() {
    epoch++;
    exportedContext = "";
    request.clear();
    emit({ sum: "", error: "" });
  }
  const selected = () =>
    state.rows.filter((row) => state.selected.includes(row.key.id)).map(guide);
  return {
    get state() {
      return state;
    },
    async start() {
      try {
        const discovery = await client.discover(startAbort.signal);
        if (disposed) return;
        if (
          discovery.resources.length !== 1 ||
          discovery.resources[0].kind !== kind ||
          stringifyWire(discovery as unknown as WireValue).includes(
            "internal_notes",
          )
        )
          throw Error("Unexpected public catalog");
        const query = {
          filters: [{ field: "title", value: "Electrical inspection" }],
          limit: 20,
        };
        const rows = await client.query(kind, query, startAbort.signal);
        if (disposed) return;
        rows.forEach(guide);
        observation = createObservation({
          scope: { kind: "public", key: "maintenance-public-guide-v1" },
          source: (signal) => client.observe(kind, query, signal),
          clone: guide,
          measure: (rows) =>
            new TextEncoder().encode(
              stringifyWire(rows as unknown as WireValue),
            ).length,
          limits: { maxRows: 20, maxBytes: 65536 },
          retry: {
            maxAttempts: 3,
            maxElapsedMs: 10000,
            delayMs: 100,
            clock: () => performance.now(),
          },
          classifyError: (error) =>
            error instanceof RemoteError && [401, 403].includes(error.status)
              ? { kind: "denied", code: "Public access denied" }
              : { kind: "transient", code: "Public connection unavailable" },
          onAuthorityLost: () => {
            clearResult();
            selection.clear();
            emit({ rows: [], selected: [] });
          },
        });
        observation.subscribe((next) => {
          if (disposed) return;
          if (
            stringifyWire(next.rows as unknown as WireValue) !==
            stringifyWire(state.rows as unknown as WireValue)
          )
            clearResult();
          selection.replaceAvailable(next.rows.map((row) => row.key.id));
          emit({
            rows: next.rows,
            selected: selection.selected,
            phase: next.phase,
          });
        });
        observation.start();
        observation.setVisible(document.visibilityState !== "hidden");
      } catch {
        if (!disposed) emit({ error: "Public guides unavailable" });
      }
    },
    toggle(id: string): CompositionCommandResult {
      selection.toggle(id);
      clearResult();
      emit({ selected: selection.selected });
      return "accepted";
    },
    compute() {
      try {
        emit({ sum: sum(selected()).toString(), error: "" });
      } catch {
        emit({ sum: "", error: "Select valid public guides" });
      }
    },
    async exportSelected() {
      try {
        const ticket = context(),
          rows = selected(),
          total = sum(rows);
        const snapshots = rows.map((view) =>
          captureExportSnapshot({
            identity: {
              principal: null,
              authorityTicket: ticket,
              resource: { kind, id: view.key.id, revision: view.revision },
              selectedDate: field(view, "updated_at"),
              format: "application/json",
              locale: "en-GB",
            },
            value: view,
            clone: guide,
          }),
        );
        exportedContext = "";
        await request.run(ticket, async (signal) => {
          for (const snapshot of snapshots) {
            const confirmed = guide(
              await client.read(kind, snapshot.identity.resource.id, signal),
            );
            if (
              !current(ticket) ||
              confirmed.revision !== snapshot.identity.resource.revision
            )
              throw Error("Public context changed");
          }
          if (!current(ticket)) throw Error("Public context changed");
          const bytes = stringifyWire(
            {
              scope: "maintenance-public-guide-v1",
              nominalValueSum: total,
              snapshots: snapshots.map((snapshot) => ({
                identity: snapshot.identity,
                value: snapshot.value,
              })),
            } as unknown as WireValue,
            65536,
          );
          if (!current(ticket)) throw Error("Public context changed");
          exportedContext = ticket;
          return bytes;
        });
      } catch {
        if (!disposed) emit({ error: "Select valid public guides" });
      }
    },
    download() {
      return state.exportPhase === "ready" && current(exportedContext)
        ? { text: state.exportText, context: exportedContext }
        : null;
    },
    isCurrent: current,
    visible(visible: boolean) {
      observation?.setVisible(visible);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      epoch++;
      exportedContext = "";
      startAbort.abort();
      observation?.dispose();
      request.clear();
      client.invalidateSession();
    },
  };
}
