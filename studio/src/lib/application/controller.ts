import { RemoteError } from "../client/client.ts";
import type {
  RomClient,
  ResourceDescriptor,
  ProjectedView,
  PendingMutation,
  Operation,
  QuerySpec,
  WireValue,
  WireObject,
} from "../client/types.ts";
export interface ApplicationState {
  phase: "disconnected" | "connecting" | "ready" | "error";
  descriptors: ResourceDescriptor[];
  kind: string;
  rows: ProjectedView[];
  selected: ProjectedView | null;
  query: QuerySpec;
  page: number;
  hasPrevious: boolean;
  busy: boolean;
  error: string;
  pending: PendingMutation | null;
  live: boolean;
  work: WireValue;
}
export function createApplication(
  client: RomClient,
  key: () => string = () => globalThis.crypto.randomUUID(),
  revalidate?: () => Promise<boolean>,
) {
  let state: ApplicationState = {
    phase: "disconnected",
    descriptors: [],
    kind: "",
    rows: [],
    selected: null,
    query: { limit: 50 },
    page: 1,
    hasPrevious: false,
    busy: false,
    error: "",
    pending: null,
    live: false,
    work: null,
  };
  let epoch = 0,
    navigation = 0,
    rowSelection = 0,
    subscription: AbortController | undefined;
  let requests = new AbortController();
  const history: { query: QuerySpec; page: number }[] = [];
  let firstQuery: QuerySpec = { limit: 50 };
  const listeners = new Set<(state: ApplicationState) => void>();
  function update(patch: Partial<ApplicationState>) {
    state = { ...state, ...patch };
    for (const listener of listeners) listener(state);
  }
  function stopLive() {
    subscription?.abort();
    subscription = undefined;
    update({ live: false });
  }
  function message(problem: unknown) {
    return problem instanceof Error ? problem.message : "Operation failed.";
  }
  async function loadPage(kind: string, query: QuerySpec, page: number) {
    stopLive();
    const started = epoch,
      selected = ++navigation;
    rowSelection++;
    update({
      kind,
      query,
      page,
      hasPrevious: history.length > 0,
      rows: [],
      selected: null,
      busy: true,
      error: "",
    });
    try {
      const rows = await client.query(kind, query, requests.signal);
      if (started === epoch && selected === navigation)
        update({ rows, busy: false });
    } catch (problem) {
      if (started === epoch && selected === navigation)
        update({ error: message(problem), busy: false });
    }
    return selected;
  }
  async function selectKind(kind: string, query: QuerySpec = { limit: 50 }) {
    history.length = 0;
    const { after: _after, after_id: _id, ...initialQuery } = query;
    firstQuery = initialQuery;
    await loadPage(kind, firstQuery, 1);
  }
  async function nextPage() {
    if (state.busy || !state.rows.length) return;
    const started = epoch,
      nav = navigation,
      kind = state.kind,
      currentQuery = state.query,
      page = state.page,
      last = state.rows.at(-1)!;
    const wasLive = state.live;
    stopLive();
    update({ busy: true, error: "" });
    try {
      const anchor = await client.anchor(currentQuery, last, requests.signal);
      if (started !== epoch || nav !== navigation) return;
      history.push({ query: currentQuery, page });
      if (history.length > 128) history.shift();
      const ticket = await loadPage(
        kind,
        { ...currentQuery, after: anchor },
        page + 1,
      );
      if (
        started === epoch &&
        ticket === navigation &&
        state.kind === kind &&
        state.page === page + 1 &&
        wasLive &&
        !state.error
      )
        void observe();
    } catch (problem) {
      if (started === epoch && nav === navigation)
        update({
          busy: false,
          rows: [],
          selected: null,
          error: message(problem),
        });
    }
  }
  async function previousPage() {
    if (state.busy) return;
    const previous = history.pop();
    if (!previous) return;
    const wasLive = state.live,
      started = epoch,
      kind = state.kind;
    const ticket = await loadPage(kind, previous.query, previous.page);
    if (
      started === epoch &&
      ticket === navigation &&
      state.kind === kind &&
      state.page === previous.page &&
      wasLive &&
      !state.error
    )
      void observe();
  }
  async function firstPage() {
    if (state.busy) return;
    const wasLive = state.live,
      started = epoch,
      kind = state.kind;
    history.length = 0;
    const ticket = await loadPage(kind, firstQuery, 1);
    if (
      started === epoch &&
      ticket === navigation &&
      state.kind === kind &&
      state.page === 1 &&
      wasLive &&
      !state.error
    )
      void observe();
  }
  async function connect() {
    const started = ++epoch;
    requests.abort();
    requests = new AbortController();
    stopLive();
    update({
      phase: "connecting",
      descriptors: [],
      rows: [],
      selected: null,
      error: "",
    });
    try {
      const discovered = await client.discover(requests.signal);
      if (started !== epoch) return;
      update({ phase: "ready", descriptors: discovered.resources });
      if (discovered.resources[0])
        await selectKind(discovered.resources[0].kind);
    } catch (problem) {
      if (started === epoch)
        update({ phase: "error", error: message(problem) });
    }
  }
  async function selectRow(id: string) {
    const started = epoch,
      nav = navigation,
      selection = ++rowSelection;
    update({ selected: null, error: "" });
    try {
      const selected = await client.read(state.kind, id, requests.signal);
      if (started === epoch && nav === navigation && selection === rowSelection)
        update({ selected });
    } catch (problem) {
      if (started === epoch && nav === navigation && selection === rowSelection)
        update({ rows: [], selected: null, error: message(problem) });
    }
  }
  async function submit(mutation: PendingMutation) {
    const started = epoch;
    update({ busy: true, error: "", pending: mutation });
    try {
      const result = await client.submit(mutation, requests.signal);
      if (started === epoch) {
        update({ pending: mutation, selected: result, busy: false });
        await refresh();
      }
    } catch (problem) {
      if (started === epoch)
        update({ pending: mutation, error: message(problem), busy: false });
      throw problem;
    }
  }
  async function mutate(
    id: string,
    expected: bigint | null,
    operation: Operation,
  ) {
    if (
      state.pending?.state === "unknown" ||
      state.pending?.state === "pending"
    )
      throw Error(
        "Resolve the pending mutation before submitting another operation.",
      );
    if (!state.kind || !id.trim())
      throw Error("Choose a Resource and enter its ID.");
    return submit(
      client.prepare({
        kind: state.kind,
        id,
        expected,
        idempotency: key(),
        operation,
      }),
    );
  }
  async function retry() {
    if (state.pending?.state !== "unknown")
      throw Error("No unknown mutation to retry.");
    await submit(state.pending);
  }
  async function refresh() {
    const started = epoch,
      nav = navigation;
    try {
      const rows = await client.query(state.kind, state.query, requests.signal);
      if (started === epoch && nav === navigation) update({ rows, error: "" });
    } catch (problem) {
      if (started === epoch && nav === navigation)
        update({ error: message(problem) });
    }
  }
  async function observe() {
    stopLive();
    const started = epoch,
      nav = navigation,
      controller = new AbortController();
    subscription = controller;
    update({ live: true, error: "" });
    let consecutiveRecoveries = 0;
    try {
      while (
        !controller.signal.aborted &&
        started === epoch &&
        nav === navigation
      ) {
        try {
          for await (const rows of client.observe(
            state.kind,
            state.query,
            controller.signal,
          )) {
            if (
              started !== epoch ||
              nav !== navigation ||
              controller.signal.aborted
            )
              return;
            consecutiveRecoveries = 0;
            update({
              rows,
              selected: state.selected
                ? (rows.find((row) => row.key.id === state.selected!.key.id) ??
                  null)
                : null,
            });
          }
          return;
        } catch (problem) {
          if (
            started !== epoch ||
            nav !== navigation ||
            controller.signal.aborted
          )
            return;
          update({ rows: [], selected: null });
          if (
            problem instanceof RemoteError &&
            problem.category === "identity_expired" &&
            consecutiveRecoveries < 1 &&
            revalidate
          ) {
            consecutiveRecoveries++;
            let valid = false;
            try {
              valid = await revalidate();
            } catch {}
            if (
              started !== epoch ||
              nav !== navigation ||
              controller.signal.aborted
            )
              return;
            if (valid) continue;
          }
          update({
            error: `Live query stopped: ${message(problem)}. Refresh to recover.`,
          });
          return;
        }
      }
    } finally {
      if (subscription === controller) {
        subscription = undefined;
        update({ live: false });
      }
    }
  }
  async function loadWork() {
    const started = epoch;
    update({ work: null, error: "" });
    try {
      const work = await client.work("capabilities", {}, requests.signal);
      if (started === epoch) update({ work });
    } catch (problem) {
      if (started === epoch) update({ error: message(problem) });
    }
  }
  async function inspectWork(
    route: "list" | "read" | "control",
    request: WireObject,
  ) {
    const started = epoch;
    const result = await client.work(route, request, requests.signal);
    if (started !== epoch) throw Error("Session changed.");
    return result;
  }
  function disconnect() {
    epoch++;
    navigation++;
    rowSelection++;
    history.length = 0;
    requests.abort();
    stopLive();
    client.invalidateSession();
    update({
      phase: "disconnected",
      descriptors: [],
      kind: "",
      rows: [],
      selected: null,
      pending: null,
      error: "",
      busy: false,
      work: null,
      page: 1,
      hasPrevious: false,
    });
  }
  return {
    get state() {
      return state;
    },
    subscribe(listener: (state: ApplicationState) => void) {
      listeners.add(listener);
      listener(state);
      return () => listeners.delete(listener);
    },
    connect,
    selectKind,
    nextPage,
    previousPage,
    firstPage,
    selectRow,
    mutate,
    retry,
    refresh,
    observe,
    stopLive,
    loadWork,
    inspectWork,
    disconnect,
  };
}
export type ApplicationController = ReturnType<typeof createApplication>;
