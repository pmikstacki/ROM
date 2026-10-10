import type { ObservationScope } from "./scope.ts";
export type ObservationPhase =
  | "idle"
  | "connecting"
  | "fresh"
  | "stale"
  | "denied"
  | "exhausted"
  | "disposed";
export interface ObservationState<T> {
  readonly phase: ObservationPhase;
  readonly scope: ObservationScope;
  readonly rows: readonly T[];
  readonly stale: boolean;
  readonly code: string | null;
}
export type ObservationSource<T> = (
  signal: AbortSignal,
) => AsyncIterable<readonly T[]>;
export interface ObservationOptions<T> {
  scope: ObservationScope;
  source: ObservationSource<T>;
  /** Validate and detach a row; the host must bound the accepted shape. */
  clone(row: T): T;
  /** Exact encoded byte count for this host's accepted row representation. */
  measure(rows: readonly T[]): number;
  limits: { maxRows: number; maxBytes: number };
  retry: {
    maxAttempts: number;
    maxElapsedMs: number;
    delayMs: number;
    clock(): number;
  };
  classifyError(error: unknown): {
    kind: "transient" | "denied" | "fatal";
    code: string;
  };
  onAuthorityLost(scope: ObservationScope): void;
  /** Sanitized host callback diagnostics. Throwing subscribers are removed. */
  onCallbackError?(
    code: "ObservationSubscriber" | "ObservationAuthorityCallback",
  ): void;
}
export interface Observation<T> {
  readonly state: ObservationState<T>;
  start(): void;
  setVisible(visible: boolean): void;
  rebind(scope: ObservationScope, source: ObservationSource<T>): void;
  retry(): void;
  subscribe(listener: (state: ObservationState<T>) => void): () => void;
  dispose(): void;
}
