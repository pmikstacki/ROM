/** Wire values preserve large integers as bigint. Objects never confer identity or authority. */
export type WireValue =
  null | boolean | string | number | bigint | WireValue[] | WireObject;
export interface WireObject {
  [key: string]: WireValue;
}

export type Shape =
  | { type: "string" | "bool" | "u64" | "i64" | "f64" }
  | { type: "optional" | "nullable" | "list" | "map"; value: Shape }
  | { type: "enum"; value: string[] }
  | { type: "reference"; value: { kind: string } };
export interface CodecIdentity {
  name: string;
  version: number;
}
export type CodecWrapper = "optional" | "nullable" | "list" | "map";
export interface FieldDescriptor {
  name: string;
  shape: Shape;
  codec?: CodecIdentity;
  codec_wrappers?: CodecWrapper[];
  enum_labels?: Record<string, string>;
}
export type InputDescriptor =
  | { type: "unit" }
  | {
      type: "scalar";
      value: {
        shape: Shape;
        codec?: CodecIdentity;
        codec_wrappers?: CodecWrapper[];
        enum_labels?: Record<string, string>;
      };
    }
  | { type: "object"; value: FieldDescriptor[] };
export interface ActionInput {
  name: string;
  version: number;
  input: InputDescriptor | null;
}
/** Advisory metadata. It never changes accepted values or permissions. */
export interface FieldPresentation {
  label?: string;
  help?: string;
  group?: string;
}
export interface PresentationGroup {
  name: string;
  label: string;
}
export interface SettingsPresentation {
  group: string;
  label: string;
}
export interface ResourcePresentation {
  label?: string;
  title_field?: string;
  fields?: Record<string, FieldPresentation>;
  groups?: PresentationGroup[];
  settings?: SettingsPresentation;
}
export interface ResourceDescriptor {
  kind: string;
  version: number;
  fields: FieldDescriptor[];
  actions: string[];
  action_inputs: ActionInput[];
  presentation?: ResourcePresentation;
}
export interface Discovery {
  version: number;
  resources: ResourceDescriptor[];
}
export interface ProjectedView {
  key: { kind: string; id: string };
  revision: bigint;
  value: WireObject | null;
}

/** Omit does not set null. Remove is an explicit patch operation for an optional field. */
export type FieldIntent =
  { mode: "omit" | "remove" | "null" } | { mode: "value"; value: WireValue };
export type FieldUpdate = { op: "set"; value: WireValue } | { op: "remove" };
export type Operation =
  | { type: "create" | "replace"; input: WireObject }
  | { type: "patch"; input: Record<string, FieldUpdate> }
  | { type: "delete" }
  | { type: "action"; input: { name: string; input: WireValue } };
export interface Invocation {
  kind: string;
  id: string;
  expected: bigint | null;
  idempotency: string;
  retry_epoch?: bigint;
  operation: Operation;
}
export type CompareOp = "eq" | "ne" | "lt" | "le" | "gt" | "ge";
/** A moving boundary with exact native codec JSON. It grants no authority. */
export interface QueryAnchor {
  readonly kind: string;
  readonly id: string;
  readonly schema_version: number;
  readonly canonical: string;
}
export interface QuerySpec {
  filters?: { field: string; value: WireValue; absent?: boolean }[];
  comparisons?: {
    field: string;
    op: CompareOp;
    value: WireValue;
    absent?: boolean;
  }[];
  order?: { field: string; direction: "asc" | "desc" }[];
  after?: QueryAnchor | null;
  after_id?: string | null;
  limit?: number | null;
}
export interface ClientOptions {
  base: string;
  fetch?: typeof globalThis.fetch;
  csrf?: () => string | undefined;
  maxBytes?: number;
  timeoutMs?: number;
  maxRows?: number;
  maxObservations?: number;
}
export type MutationState =
  "pending" | "unknown" | "succeeded" | "rejected" | "conflict";
export interface PendingMutation {
  request: Invocation;
  fingerprint: string;
  generation: number;
  state: MutationState;
  result?: ProjectedView;
  error?: string;
}
export interface BlobCapabilities {
  version: 1;
  resource_kind: string;
  stores: string[];
  limits: { blob_bytes: number; chunk_bytes: number; chunks: number };
  operations: ("reserve" | "upload" | "download" | "detach")[];
}
export interface BlobReservation {
  id: string;
  store: string;
  digest: string;
  bytes: bigint;
  idempotency: string;
}
export interface RomClient {
  readonly generation: number;
  invalidateSession(): void;
  blobCapabilities(signal?: AbortSignal): Promise<BlobCapabilities>;
  reserveBlob(
    request: BlobReservation,
    signal?: AbortSignal,
  ): Promise<ProjectedView>;
  uploadBlob(
    id: string,
    file: Blob,
    signal?: AbortSignal,
  ): Promise<ProjectedView>;
  downloadBlob(id: string, signal?: AbortSignal): Promise<Uint8Array>;
  detachBlob(id: string, signal?: AbortSignal): Promise<ProjectedView>;
  discover(signal?: AbortSignal): Promise<Discovery>;
  read(kind: string, id: string, signal?: AbortSignal): Promise<ProjectedView>;
  query(
    kind: string,
    query: QuerySpec,
    signal?: AbortSignal,
  ): Promise<ProjectedView[]>;
  anchor(
    query: QuerySpec,
    last: ProjectedView,
    signal?: AbortSignal,
  ): Promise<QueryAnchor>;
  prepare(request: Invocation): PendingMutation;
  submit(
    mutation: PendingMutation,
    signal?: AbortSignal,
  ): Promise<ProjectedView>;
  observe(
    kind: string,
    query: QuerySpec,
    signal: AbortSignal,
  ): AsyncIterable<ProjectedView[]>;
  journal(
    kind: string,
    after?: WireObject | null,
    signal?: AbortSignal,
  ): Promise<WireValue>;
  work(
    route: "capabilities" | "list" | "read" | "control",
    request: WireObject,
    signal?: AbortSignal,
  ): Promise<WireValue>;
}
