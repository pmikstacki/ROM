import { createClient } from "../client/client.ts";
import type { RomClient } from "../client/types.ts";
import { createBrowserSessionDriver } from "../auth/browser-driver.ts";
import { createBrowserSessionTransport } from "../auth/browser-transport.ts";
import { createSessionLifecycle } from "../auth/lifecycle.ts";
import type { SessionLifecycle } from "../auth/types.ts";
import { createApplication } from "./controller.ts";
import type { StudioAuthProfile } from "./session-types.ts";

function validate(profile: StudioAuthProfile) {
  const store = (value: StudioAuthProfile["recovery"]["intentStore"]) =>
    value &&
    typeof value.read === "function" &&
    typeof value.compareExchange === "function";
  const config = profile?.recovery;
  if (
    !profile ||
    typeof profile.authority !== "string" ||
    !profile.authority ||
    profile.authority.length > 4096 ||
    typeof profile.now !== "function" ||
    !config ||
    typeof config.namespace !== "string" ||
    !config.namespace ||
    config.namespace.length > 4096 ||
    !store(config.intentStore) ||
    !store(config.editorStore) ||
    !Number.isSafeInteger(config.maxBytes) ||
    config.maxBytes < 1 ||
    [
      config.slot,
      config.newVersion,
      config.newCommandKey,
      config.retryEpoch,
    ].some((value) => typeof value !== "function")
  )
    throw Error("Invalid Studio auth profile configuration.");
}
/** Trusted host composition. No persistence, authority namespace or credentials are inferred. */
export function createAppSession(options: {
  base: string;
  profile: StudioAuthProfile;
  client?: RomClient;
  fetch?: typeof fetch;
}) {
  validate(options.profile);
  const profile = options.profile;
  const driver = createBrowserSessionDriver({
    base: options.base,
    authority: profile.authority,
    now: profile.now,
    fetch: options.fetch,
  });
  // Provider discovery uses the same validated protocol implementation, without a credential acquisition.
  const providerTransport = createBrowserSessionTransport({
    base: options.base,
    now: profile.now,
    fetch: options.fetch,
  });
  const client =
    options.client ??
    createClient({
      base: `${options.base.replace(/\/$/, "")}/api`,
      csrf: driver.csrf,
      fetch: options.fetch,
    });
  let lifecycle!: SessionLifecycle,
    viewEpoch = 0,
    disposed = false;
  let providerRequest: AbortController | null = null;
  const controller = createApplication(
    client,
    undefined,
    async () => {
      await lifecycle.refresh();
      return (
        !disposed &&
        !!lifecycle.state.identity &&
        controller.state.session?.mutationAllowed === true
      );
    },
    { recovery: profile.recovery },
  );
  lifecycle = createSessionLifecycle({
    driver,
    now: profile.now,
    onTransition: async (transition) => {
      const ticket = ++viewEpoch;
      providerRequest?.abort();
      providerRequest = null;
      if (disposed) return;
      if (transition.kind === "transient") controller.pauseSession("transient");
      else {
        if (transition.next) controller.pauseSession("renewing");
        if (disposed || ticket !== viewEpoch) return;
        await controller.rebindSession({
          client,
          principal: transition.next?.principal ?? null,
        });
      }
    },
  });
  return {
    client,
    controller,
    lifecycle,
    refresh: lifecycle.refresh,
    logout: lifecycle.logout,
    loginUrl: (id: string) =>
      `${options.base.replace(/\/$/, "")}/auth/login/${encodeURIComponent(id)}`,
    async providers() {
      const ticket = viewEpoch,
        request = new AbortController();
      providerRequest?.abort();
      providerRequest = request;
      try {
        const choices = await providerTransport.providers(request.signal);
        if (disposed || ticket !== viewEpoch || request.signal.aborted)
          throw Error("Session changed.");
        return choices;
      } finally {
        if (providerRequest === request) providerRequest = null;
      }
    },
    destroy() {
      if (disposed) return;
      disposed = true;
      viewEpoch++;
      providerRequest?.abort();
      providerRequest = null;
      lifecycle.dispose();
      providerTransport.invalidate();
      controller.disconnect();
    },
  };
}
export type AppSession = ReturnType<typeof createAppSession>;
