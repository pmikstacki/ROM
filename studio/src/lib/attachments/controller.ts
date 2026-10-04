import { RemoteError } from "../client/client.ts";
import type {
  BlobCapabilities,
  BlobReservation,
  ProjectedView,
  RomClient,
} from "../client/types.ts";
export type AttachmentClient = Pick<
  RomClient,
  | "generation"
  | "reserveBlob"
  | "uploadBlob"
  | "detachBlob"
  | "query"
  | "downloadBlob"
>;
type Pending =
  | { step: "reserve" | "upload"; reservation: BlobReservation; file: Blob }
  | { step: "detach"; id: string };
export interface AttachmentState {
  busy: boolean;
  phase: "idle" | "preparing" | "pending" | "unknown" | "success" | "error";
  rows: ProjectedView[];
  result: ProjectedView | null;
  pending: Pending | null;
  error: string;
}
export function createAttachments(
  client: AttachmentClient,
  cap: BlobCapabilities,
  key: () => string = () => crypto.randomUUID(),
  hash: (bytes: ArrayBuffer) => Promise<string> = async (bytes) =>
    Array.from(
      new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
      (b) => b.toString(16).padStart(2, "0"),
    ).join(""),
) {
  let state: AttachmentState = {
    busy: false,
    phase: "idle",
    rows: [],
    result: null,
    pending: null,
    error: "",
  };
  const listeners = new Set<(state: AttachmentState) => void>(),
    requests = new AbortController();
  const generation = client.generation;
  let disposed = false,
    sequence = 0,
    running = false;
  const current = () => !disposed && generation === client.generation;
  const publish = (patch: Partial<AttachmentState>) => {
    if (!current()) return;
    state = { ...state, ...patch };
    for (const listener of listeners) listener(state);
  };
  const message = (error: unknown) =>
    error instanceof Error ? error.message : "Attachment operation failed.";
  async function refresh() {
    const selected = ++sequence;
    try {
      const rows = await client.query(
        cap.resource_kind,
        { limit: 50 },
        requests.signal,
      );
      if (selected === sequence) publish({ rows, error: "" });
    } catch (error) {
      if (selected === sequence)
        publish({ rows: [], result: null, error: message(error) });
    }
  }
  function blocked() {
    if (
      running ||
      state.phase === "pending" ||
      state.phase === "preparing" ||
      state.phase === "unknown"
    )
      throw Error("Resolve the current attachment operation first.");
  }
  async function run() {
    const pending = state.pending;
    if (!pending || !current() || running) return;
    running = true;
    publish({ phase: "pending", busy: true, error: "" });
    try {
      let result: ProjectedView;
      if (pending.step === "detach")
        result = await client.detachBlob(pending.id, requests.signal);
      else {
        if (pending.step === "reserve") {
          await client.reserveBlob(pending.reservation, requests.signal);
          pending.step = "upload";
        }
        result = await client.uploadBlob(
          pending.reservation.id,
          pending.file,
          requests.signal,
        );
      }
      publish({ phase: "success", pending: null, result });
      await refresh();
    } catch (error) {
      const confirmed =
        error instanceof RemoteError &&
        (error.category === "not_committed" ||
          (error.status > 0 &&
            error.status < 500 &&
            !["outcome_unknown", "timeout", "overloaded"].includes(
              error.category,
            )));
      publish({
        phase: confirmed ? "error" : "unknown",
        error: message(error),
        pending: confirmed ? null : pending,
        rows: confirmed ? [] : state.rows,
        result: confirmed ? null : state.result,
      });
    } finally {
      running = false;
      publish({ busy: false });
    }
  }
  return {
    get state() {
      return state;
    },
    subscribe(listener: (state: AttachmentState) => void) {
      listeners.add(listener);
      listener(state);
      return () => listeners.delete(listener);
    },
    refresh,
    async upload(id: string, store: string, file: Blob) {
      blocked();
      publish({ phase: "preparing", result: null, error: "" });
      try {
        if (
          !id ||
          id.length > 4096 ||
          !cap.stores.includes(store) ||
          file.size > cap.limits.blob_bytes ||
          BigInt(file.size) >
            BigInt(cap.limits.chunk_bytes) * BigInt(cap.limits.chunks)
        )
          throw Error(
            "File, Resource ID, or store exceeds the permitted limits.",
          );
        const bytes = await file.arrayBuffer();
        if (!current()) return;
        const frozen = new Blob([bytes], { type: "application/octet-stream" });
        const digest = await hash(bytes);
        if (!current()) return;
        const reservation = Object.freeze({
          id,
          store,
          digest,
          bytes: BigInt(bytes.byteLength),
          idempotency: key(),
        });
        publish({ pending: { step: "reserve", reservation, file: frozen } });
        await run();
      } catch (error) {
        publish({ phase: "error", error: message(error), pending: null });
      }
    },
    retry: run,
    async detach(id: string) {
      blocked();
      publish({ pending: { step: "detach", id }, result: null });
      await run();
    },
    async download(id: string) {
      if (!current()) throw Error("Attachment view is no longer active.");
      const bytes = await client.downloadBlob(id, requests.signal);
      if (!current()) throw Error("Attachment view is no longer active.");
      return bytes;
    },
    dispose() {
      disposed = true;
      requests.abort();
      listeners.clear();
      state = {
        busy: false,
        phase: "idle",
        rows: [],
        result: null,
        pending: null,
        error: "",
      };
    },
  };
}
