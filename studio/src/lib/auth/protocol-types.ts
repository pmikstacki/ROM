/** Private validated browser protocol; credentials never enter lifecycle snapshots. */
export interface BrowserSession {
  authenticated: boolean;
  generation: string;
  csrf_token?: string;
  user_id?: string;
  expires_at?: number;
}
export interface ProviderChoice {
  id: string;
  label: string;
}
export type ValidatedBrowserSession =
  | { authenticated: false; generation: string }
  | {
      authenticated: true;
      generation: string;
      csrf_token: string;
      user_id: string;
      expires_at: number;
    };
export type BrowserProtocolResult =
  | { status: "session"; session: ValidatedBrowserSession }
  | { status: "expired" }
  | { status: "denied" }
  | {
      status: "transient";
      code:
        | "invalid_response"
        | "unavailable"
        | "closed"
        | "overloaded"
        | "internal";
    }
  | { status: "changed" };
export interface BrowserTransportOptions {
  /** Exact mount prefix, including its trailing slash when required by the host. */
  base: string;
  now: () => number;
  fetch?: typeof fetch;
  timeoutMs?: number;
}
export interface BrowserSessionTransport {
  refresh(signal: AbortSignal): Promise<BrowserProtocolResult>;
  providers(
    signal: AbortSignal,
  ): Promise<{ providers: ProviderChoice[]; primary: string | null }>;
  logout(signal: AbortSignal): Promise<void>;
  invalidate(): void;
  csrf(): string | undefined;
}
