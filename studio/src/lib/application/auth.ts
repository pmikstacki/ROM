/** Stable application auth API backed by the shared validated browser protocol. */
export {
  createBrowserAuth,
  SessionExpiredError,
  SessionDeniedError,
} from "../auth/legacy-adapter.ts";
export type { BrowserSession, ProviderChoice } from "../auth/protocol-types.ts";
