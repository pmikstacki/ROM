import { createLiveIntent } from "./live-intent.ts";
import { createSessionBinding, sameOwner } from "./session-binding.ts";
import { createMutationLane } from "./mutation-lane.ts";
import { createEditorDrafts } from "./editor-drafts.ts";
import { createCreationWorkflow } from "./creation-workflow.ts";
import type { CreationDraft } from "./creation-drafts.ts";
import type { CreationWorkflowState } from "./creation-workflow.ts";
import { resourceDraftIdentity } from "../resources/form-draft.ts";
import { copyApplicationState } from "./snapshot.ts";
import type {
  EditorSnapshot,
  EditorPersistenceState,
} from "./editor-drafts.ts";
import type {
  ManagedApplicationOptions,
  ApplicationSessionState,
  ApplicationRecoverySnapshot,
  SessionRenewalOutcome,
} from "./session-types.ts";
export type { StudioAuthProfile } from "./session-types.ts";
import { RemoteError } from "../client/client.ts";
import { stringifyWire } from "../client/codec.ts";
import { resourceTitle } from "../presentation/resource-presentation.ts";
import {
  REFERENCE_BYTES,
  REFERENCE_LIMIT,
  REFERENCE_SEARCH_BYTES,
  type ReferenceLookupResult,
} from "../renderers/reference-lookup.ts";
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
  session?: ApplicationSessionState;
  recovery?: ApplicationRecoverySnapshot | null;
  editor?: EditorPersistenceState;
  creation?: CreationWorkflowState | null;
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
  managed?: ManagedApplicationOptions,
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
  const liveIntent = createLiveIntent();
  let requests = new AbortController();
  const referenceRequests = new Set<AbortController>();
  const history: { query: QuerySpec; page: number }[] = [];
  let firstQuery: QuerySpec = { limit: 50 };
  const listeners = new Set<(state: ApplicationState) => void>();
  let privateDenied = false;
  let creationWork: ReturnType<typeof createCreationWorkflow> | null = null;
  let creationKind = "";
  let creationDefinition = "";
  function disposeCreation() {
    creationWork?.dispose();
    creationWork = null;
    creationKind = "";
    creationDefinition = "";
    update({ creation: null });
  }
  function creation(currentDefinition = false) {
    if (!managed?.recovery.creationSlot)
      throw Error("Creation recovery is not configured.");
    const descriptor = state.descriptors.find(
      (item) => item.kind === state.kind,
    );
    if (!descriptor || privateDenied)
      throw Error("Creation Resource authority unavailable.");
    const definition = resourceDraftIdentity(descriptor, "create");
    if (
      creationWork &&
      creationKind === state.kind &&
      (!currentDefinition || creationDefinition === definition)
    )
      return creationWork;
    creationWork?.guardNavigation();
    if (creationWork?.state.editor.snapshot)
      throw Error(
        "Resource definition changed. Discard creation edits before using the new definition.",
      );
    disposeCreation();
    creationKind = state.kind;
    creationDefinition = definition;
    creationWork = createCreationWorkflow({
      kind: state.kind,
      descriptor: definition,
      recovery: managed.recovery,
      binding: () => ({ client, principal: session?.principal ?? null }),
      allowed: () => !privateDenied && state.session?.mutationAllowed === true,
      draftAllowed: () =>
        !privateDenied &&
        state.session?.status !== "denied" &&
        !!session?.principal,
      publish: (creation) => update({ creation }),
    });
    update({ creation: creationWork.state });
    return creationWork;
  }
  function update(patch: Partial<ApplicationState>) {
    state = { ...state, ...patch };
    if (privateDenied)
      state = {
        ...state,
        descriptors: [],
        pending: null,
        rows: [],
        selected: null,
        work: null,
        error: "",
        query: { limit: 50 },
        editor: { status: "idle", target: null, snapshot: null },
        creation: null,
        recovery: state.recovery
          ? {
              target: state.recovery.target,
              state: {
                ...state.recovery.state,
                draft: null,
                result: null,
                error: null,
              },
            }
          : null,
      };
    for (const listener of listeners)
      listener(
        managed
          ? copyApplicationState(state, managed.recovery.maxBytes)
          : state,
      );
  }
  function stopLive() {
    liveIntent.stop();
    subscription?.abort();
    subscription = undefined;
    update({ live: false });
  }
  function message(problem: unknown) {
    return problem instanceof Error ? problem.message : "Operation failed.";
  }
  async function reconcileSelected(
    selected: ProjectedView | null,
    rows: ProjectedView[] | undefined,
    ticket: { epoch: number; navigation: number; selection: number },
    signal: AbortSignal,
  ) {
    const current = () =>
      ticket.epoch === epoch &&
      ticket.navigation === navigation &&
      ticket.selection === rowSelection &&
      !signal.aborted;
    if (!selected || !current()) return;
    const projected = rows?.find(
      (row) =>
        row.key.kind === selected.key.kind && row.key.id === selected.key.id,
    );
    try {
      const latest =
        projected ??
        (await client.read(selected.key.kind, selected.key.id, signal));
      if (current()) update({ selected: latest });
    } catch (problem) {
      if (current()) update({ selected: null, error: message(problem) });
    }
  }
  async function loadPage(
    kind: string,
    query: QuerySpec,
    page: number,
    retainSelected = false,
  ) {
    stopLive();
    const started = epoch,
      selected = ++navigation,
      selection = ++rowSelection,
      retained = retainSelected ? state.selected : null;
    update({
      kind,
      query,
      page,
      hasPrevious: history.length > 0,
      rows: [],
      selected: retained,
      busy: true,
      error: "",
    });
    let queryAccepted = false;
    try {
      const rows = await client.query(kind, query, requests.signal);
      if (started === epoch && selected === navigation) {
        queryAccepted = true;
        update({ rows });
        if (retainSelected)
          await reconcileSelected(
            retained,
            undefined,
            { epoch: started, navigation: selected, selection },
            requests.signal,
          );
        if (started === epoch && selected === navigation)
          update({ busy: false });
      }
    } catch (problem) {
      if (started === epoch && selected === navigation)
        update({
          error: message(problem),
          busy: false,
          ...(selection === rowSelection ? { selected: null } : {}),
        });
    }
    return { ticket: selected, queryAccepted };
  }
  function resetQuery(query: QuerySpec) {
    history.length = 0;
    const { after: _after, after_id: _id, ...initialQuery } = query;
    firstQuery = initialQuery;
    return firstQuery;
  }
  async function selectKind(kind: string, query: QuerySpec = { limit: 50 }) {
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
    await loadPage(kind, resetQuery(query), 1);
  }
  async function applyQuery(query: QuerySpec) {
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
    if (!state.kind) throw Error("Choose a Resource before applying a query.");
    const started = epoch,
      kind = state.kind,
      wasLive = state.live;
    const { ticket, queryAccepted } = await loadPage(
      kind,
      resetQuery(query),
      1,
      true,
    );
    if (
      started === epoch &&
      ticket === navigation &&
      state.kind === kind &&
      wasLive &&
      queryAccepted
    )
      void observe();
  }
  async function nextPage() {
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
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
      const { ticket } = await loadPage(
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
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
    if (state.busy) return;
    const previous = history.pop();
    if (!previous) return;
    const wasLive = state.live,
      started = epoch,
      kind = state.kind;
    const { ticket } = await loadPage(kind, previous.query, previous.page);
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
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
    if (state.busy) return;
    const wasLive = state.live,
      started = epoch,
      kind = state.kind;
    history.length = 0;
    const { ticket } = await loadPage(kind, firstQuery, 1);
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
    editors?.guardNavigation();
    lane?.guardNavigation();
    creationWork?.guardNavigation();
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
      managed?.recovery.creationSlot &&
      expected === null &&
      operation.type === "create"
    ) {
      editors?.guardNavigation();
      lane?.guardNavigation();
      const started = epoch;
      try {
        const result = await creation(true).submit(id, operation);
        if (started === epoch) {
          update({ selected: result });
          await refresh();
        }
      } catch (problem) {
        if (started === epoch) update({ error: message(problem) });
        throw problem;
      }
      return;
    }
    creationWork?.guardNavigation();
    if (lane) {
      editors?.guardNavigation();
      const started = epoch;
      try {
        const result = await lane.mutate(
          { kind: state.kind, id },
          expected,
          operation,
        );
        if (started === epoch) {
          update({ selected: result });
          await refresh();
        }
      } catch (problem) {
        if (started === epoch) update({ error: message(problem) });
        throw problem;
      }
      return;
    }
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
    if (lane) {
      const started = epoch;
      try {
        const result = await lane.retry();
        if (started === epoch) {
          update({ selected: result });
          await refresh();
        }
      } catch (problem) {
        if (started === epoch) update({ error: message(problem) });
        throw problem;
      }
      return;
    }
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
    liveIntent.start();
    if (managed && state.session?.status !== "active") return;
    return runObservation();
  }
  async function runObservation() {
    const started = epoch,
      nav = navigation,
      previous = subscription;
    previous?.abort();
    if (
      started !== epoch ||
      nav !== navigation ||
      subscription !== previous ||
      !liveIntent.wanted ||
      (managed && state.session?.status !== "active")
    )
      return;
    const controller = new AbortController();
    subscription = controller;
    update({ live: true, error: "" });
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
            liveIntent.snapshot();
            const selected = state.selected,
              selection = rowSelection;
            update({ rows });
            await reconcileSelected(
              selected,
              rows,
              { epoch: started, navigation: nav, selection },
              controller.signal,
            );
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
            revalidate &&
            liveIntent.recover()
          ) {
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
        liveIntent.stop();
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
  /** Query a disclosed target without changing the main Resource navigation or draft. */
  async function lookupResources(
    kind: string,
    search = "",
    external?: AbortSignal,
  ): Promise<ReferenceLookupResult> {
    const descriptor =
      state.phase === "ready"
        ? state.descriptors.find((candidate) => candidate.kind === kind)
        : undefined;
    if (!descriptor)
      return {
        status: "unavailable",
        message:
          "Reference candidates are unavailable. Enter an exact Resource ID.",
      };
    if (new TextEncoder().encode(search).byteLength > REFERENCE_SEARCH_BYTES)
      return { status: "error", message: "Reference search is too long." };
    if (referenceRequests.size >= 4)
      return {
        status: "error",
        message: "Reference lookup limit reached. Try again.",
      };
    const started = epoch,
      generation = client.generation,
      controller = new AbortController();
    const signal = AbortSignal.any([
      requests.signal,
      controller.signal,
      ...(external ? [external] : []),
    ]);
    const current = () =>
      !signal.aborted &&
      started === epoch &&
      generation === client.generation &&
      state.phase === "ready" &&
      state.descriptors.includes(descriptor);
    referenceRequests.add(controller);
    try {
      if (!current())
        return { status: "cancelled", message: "Reference lookup cancelled." };
      const rows = await client.query(kind, { limit: REFERENCE_LIMIT }, signal);
      if (!current())
        return { status: "cancelled", message: "Reference lookup cancelled." };
      // The SDK already bounds wire bytes and rows. This smaller admission limit bounds picker state.
      if (
        rows.length > REFERENCE_LIMIT ||
        rows.some((row) => row.key.kind !== kind)
      )
        throw Error("reference rows limit");
      stringifyWire(rows as unknown as WireValue, REFERENCE_BYTES);
      const needle = search.toLowerCase();
      const candidates = rows
        .filter((row) => row.value !== null)
        .map((row) => ({
          id: row.key.id,
          title: resourceTitle(descriptor, row),
        }))
        .filter(
          (candidate) =>
            candidate.id.toLowerCase().includes(needle) ||
            candidate.title.toLowerCase().includes(needle),
        );
      return {
        status: "ready",
        candidates,
        limited: rows.length === REFERENCE_LIMIT,
      };
    } catch (problem) {
      if (!current())
        return { status: "cancelled", message: "Reference lookup cancelled." };
      if (
        problem instanceof RemoteError &&
        (problem.category === "denied" || problem.status === 403)
      )
        return {
          status: "denied",
          message: "Reference lookup was denied. Enter an exact Resource ID.",
        };
      return {
        status: "error",
        message: "Reference lookup failed. Enter an exact Resource ID.",
      };
    } finally {
      referenceRequests.delete(controller);
      controller.abort();
    }
  }
  function fenceSession() {
    epoch++;
    navigation++;
    rowSelection++;
    requests.abort();
    requests = new AbortController();
    for (const request of referenceRequests) request.abort();
    referenceRequests.clear();
    subscription?.abort();
    subscription = undefined;
  }
  function denyProjection() {
    liveIntent.stop();
    privateDenied = true;
    fenceSession();
    const ticket = epoch;
    history.length = 0;
    update({
      rows: [],
      selected: null,
      work: null,
      hasPrevious: false,
      page: 1,
      editor: { status: "idle", target: null, snapshot: null },
      ...(state.recovery
        ? {
            recovery: {
              target: state.recovery.target,
              state: {
                ...state.recovery.state,
                draft: null,
                result: null,
                error: null,
              },
            },
          }
        : {}),
    });
    if (ticket === epoch) editors?.rebind(null);
  }
  async function renewSession(
    preserve: boolean,
  ): Promise<SessionRenewalOutcome> {
    if (!preserve || !state.kind) {
      await connect();
      return state.phase === "ready" && !state.error ? "fresh" : "transient";
    }
    const ticket = epoch,
      kind = state.kind,
      selected = state.selected;
    try {
      const discovered = await client.discover(requests.signal);
      if (ticket !== epoch) return "transient";
      if (!discovered.resources.some((item) => item.kind === kind)) {
        update({ descriptors: discovered.resources });
        if (ticket !== epoch) return "transient";
        denyProjection();
        return "denied";
      }
      const rows = await client.query(kind, state.query, requests.signal);
      if (ticket !== epoch) return "transient";
      let current = selected;
      if (selected) {
        current = await client.read(kind, selected.key.id, requests.signal);
      }
      if (ticket !== epoch) return "transient";
      privateDenied = false;
      update({
        descriptors: discovered.resources,
        rows,
        selected: current,
        error: "",
      });
      return "fresh";
    } catch (problem) {
      if (ticket !== epoch) return "transient";
      const denied =
        problem instanceof RemoteError &&
        ((problem.category === "denied" && problem.status === 403) ||
          (problem.category === "missing" && problem.status === 404));
      if (denied) denyProjection();
      if (ticket === epoch) update({ error: message(problem) });
      return denied ? "denied" : "transient";
    }
  }
  let session: ReturnType<typeof createSessionBinding> | null = null;
  const lane: ReturnType<typeof createMutationLane> | null = managed
    ? createMutationLane({
        recovery: managed.recovery,
        binding: () => ({ client, principal: session?.principal ?? null }),
        allowed: () => state.session?.mutationAllowed === true,
        publish: (recovery) => update({ recovery }),
      })
    : null;
  const editors: ReturnType<typeof createEditorDrafts> | null = managed
    ? createEditorDrafts({
        recovery: managed.recovery,
        binding: () => ({ client, principal: session?.principal ?? null }),
        publish: (editor) => update({ editor }),
      })
    : null;
  session = managed
    ? createSessionBinding({
        client: () => client,
        setClient: (next) => {
          client = next;
        },
        fence: fenceSession,
        rebind: (binding) => {
          editors!.rebind(binding.principal);
          return Promise.all([
            lane!.rebind(binding),
            creationWork?.rebind(binding),
          ]).then(() => {});
        },
        clear: () => {
          liveIntent.stop();
          disposeCreation();
          privateDenied = false;
          history.length = 0;
          update({
            phase: "disconnected",
            descriptors: [],
            kind: "",
            rows: [],
            selected: null,
            pending: null,
            recovery: null,
            editor: { status: "idle", target: null, snapshot: null },
            work: null,
            error: "",
            page: 1,
            hasPrevious: false,
          });
        },
        renew: renewSession,
        publish: (next) => {
          const publishedEpoch = epoch,
            publishedNavigation = navigation;
          update({ session: next, busy: false, live: false });
          if (
            publishedEpoch === epoch &&
            publishedNavigation === navigation &&
            next.status === "active" &&
            state.session?.status === "active" &&
            liveIntent.wanted &&
            !subscription
          )
            void runObservation();
        },
      })
    : null;
  function disconnect() {
    disposeCreation();
    if (session) {
      session.clear();
      lane?.dispose();
    }
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
  function createDraftWriter(id: string) {
    if (!session || !lane || !editors)
      throw Error("managed draft binding unavailable");
    const owner = session.principal;
    const target = { kind: state.kind, id };
    const check = () => {
      if (!sameOwner(owner, session?.principal ?? null))
        throw Error("Draft owner changed.");
      if (privateDenied || state.session?.status === "denied")
        throw Error("Draft authority denied.");
      if (
        target.kind !== state.kind ||
        state.selected?.key.kind !== target.kind ||
        state.selected.key.id !== target.id
      )
        throw Error("Draft target changed.");
    };
    check();
    return {
      refuse() {
        check();
        editors.refuse(target);
      },
      async stage(
        snapshot:
          EditorSnapshot | ((current: EditorSnapshot | null) => EditorSnapshot),
        operation: Operation | null,
      ) {
        check();
        let next: EditorSnapshot;
        try {
          const current = editors.state;
          next =
            typeof snapshot === "function"
              ? snapshot(
                  current.target?.kind === target.kind &&
                    current.target.id === target.id
                    ? current.snapshot
                    : null,
                )
              : snapshot;
        } catch (problem) {
          check();
          editors.refuse(target);
          throw problem;
        }
        check();
        await editors.stage(target, next);
        check();
        await lane.stage(target, operation);
        check();
      },
    };
  }
  return {
    get creationSupported() {
      return !!managed?.recovery.creationSlot;
    },
    createCreationWriter() {
      const owner = session?.principal ?? null,
        kind = state.kind;
      let workflow = creation();
      return {
        async stage(value: CreationDraft) {
          if (
            !sameOwner(owner, session?.principal ?? null) ||
            kind !== state.kind ||
            workflow !== creationWork
          )
            throw Error("Creation draft owner or kind changed.");
          const descriptor = state.descriptors.find(
            (item) => item.kind === kind,
          );
          if (
            !descriptor ||
            value.form.descriptor !==
              resourceDraftIdentity(descriptor, "create")
          )
            throw Error("Creation draft definition changed.");
          workflow = creation(true);
          await workflow.stage(value);
        },
      };
    },
    restoreCreation: () => creation().restore(),
    retryCreation: async () => {
      const started = epoch;
      const result = await creation().retry();
      if (started === epoch) {
        update({ selected: result });
        await refresh();
      }
    },
    discardCreation: (acknowledgePossibleCommit = false) =>
      creation().discard(acknowledgePossibleCommit),
    createDraftWriter,
    get state() {
      return managed
        ? copyApplicationState(state, managed.recovery.maxBytes)
        : state;
    },
    subscribe(listener: (state: ApplicationState) => void) {
      listeners.add(listener);
      listener(
        managed
          ? copyApplicationState(state, managed.recovery.maxBytes)
          : state,
      );
      return () => listeners.delete(listener);
    },
    pauseSession: (reason: "transient" | "renewing") => {
      if (!session) throw Error("managed session unavailable");
      session.pause(reason);
    },
    rebindSession: (
      binding: import("./session-types.ts").ApplicationBinding,
    ) => {
      if (!session) throw Error("managed session unavailable");
      return session.rebind(binding);
    },
    stageDraft: async (id: string, operation: Operation | null) => {
      if (!lane) throw Error("managed recovery unavailable");
      if (privateDenied)
        throw Error("Resource authority denied; draft unavailable.");
      return lane.stage({ kind: state.kind, id }, operation);
    },
    stageEditorDraft: (id: string, snapshot: EditorSnapshot) => {
      if (!editors) throw Error("managed recovery unavailable");
      return editors.stage({ kind: state.kind, id }, snapshot);
    },
    restoreSelectedIntent: async () => {
      if (
        !lane ||
        !editors ||
        !state.selected ||
        !state.session?.mutationAllowed
      )
        throw Error("Choose an explicit authorized recovery target.");
      const target = { ...state.selected.key },
        ticket = epoch;
      await lane.restore(target);
      if (ticket !== epoch) throw Error("recovery binding changed");
      await editors.restore(target);
      if (ticket !== epoch) throw Error("recovery binding changed");
    },
    discardSelectedIntent: (options: {
      acknowledgePossibleCommit: boolean;
    }) => {
      if (!lane) throw Error("managed recovery unavailable");
      return lane.discard(options.acknowledgePossibleCommit);
    },
    connect,
    selectKind,
    applyQuery,
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
    lookupResources,
    disconnect,
  };
}
export type ApplicationController = ReturnType<typeof createApplication>;
