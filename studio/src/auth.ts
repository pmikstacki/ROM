/** Optional host session lifecycle; credentials are not observable state. */
export { createSessionLifecycle } from "./lib/auth/lifecycle.ts";
export type * from "./lib/auth/types.ts";
export { createBrowserSessionDriver } from "./lib/auth/browser-driver.ts";
export {
  createBrowserAuth,
  SessionExpiredError,
  SessionDeniedError,
} from "./lib/auth/legacy-adapter.ts";
