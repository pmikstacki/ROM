import type { DurablePrincipal } from "../recovery/types.ts";
export interface SessionIdentity {
  principal: DurablePrincipal;
  generation: string;
  expiresAt: number;
}
export type SessionCheck =
  | { status: "authenticated"; identity: SessionIdentity }
  | { status: "anonymous" }
  | { status: "denied" }
  | { status: "transient"; code: string };
export interface SessionDriver {
  /** Clear only local credential authority; never sends a mutation. */
  invalidate?(): void;
  check(signal: AbortSignal): Promise<SessionCheck>;
  logout(signal: AbortSignal): Promise<void>;
}
export interface SessionLifecycleState {
  status:
    | "checking"
    | "authenticated"
    | "anonymous"
    | "denied"
    | "transient"
    | "disposed";
  identity: SessionIdentity | null;
  transientCode: string | null;
}
export interface SessionTransition {
  kind: "renewed" | "changed" | "cleared" | "transient";
  previous: SessionIdentity | null;
  next: SessionIdentity | null;
}
export interface SessionLifecycleOptions {
  driver: SessionDriver;
  /** Current UTC time, in seconds. */
  now: () => number;
  onTransition?: (transition: SessionTransition) => void | Promise<void>;
  /** Host scheduler for expiry; returns cancellation. Defaults to setTimeout. */
  schedule?: (task: () => void, milliseconds: number) => () => void;
}
export interface SessionLifecycle {
  readonly state: SessionLifecycleState;
  refresh(): Promise<SessionCheck>;
  logout(): Promise<void>;
  dismissTransient(): void;
  subscribe(listener: (state: SessionLifecycleState) => void): () => void;
  dispose(): void;
}
export interface BrowserSessionDriverOptions {
  base: string;
  /** Trusted host identity namespace; never inferred from a cookie or CSRF token. */
  authority: string;
  now: () => number;
  fetch?: typeof fetch;
  timeoutMs?: number;
}
export interface BrowserSessionDriver extends SessionDriver {
  /** Explicit low-level transport credential access, absent from lifecycle snapshots. */
  csrf(): string | undefined;
}
