import type { DurablePrincipal } from "../recovery/types.ts";
import type { RomClient } from "../client/types.ts";
import type {
  ApplicationBinding,
  ApplicationSessionState,
  SessionRenewalOutcome,
} from "./session-types.ts";
export function sameOwner(
  left: DurablePrincipal | null,
  right: DurablePrincipal | null,
): boolean {
  return (
    !!left &&
    !!right &&
    left.authority === right.authority &&
    left.kind === right.kind &&
    left.subject === right.subject
  );
}
export function createSessionBinding(options: {
  client(): RomClient;
  setClient(client: RomClient): void;
  fence(): void;
  clear(): void;
  rebind(binding: ApplicationBinding): Promise<void>;
  renew(preserve: boolean): Promise<SessionRenewalOutcome>;
  publish(state: ApplicationSessionState): void;
}) {
  let owner: DurablePrincipal | null = null,
    epoch = 0;
  return {
    get principal() {
      return owner ? { ...owner } : null;
    },
    pause(reason: "transient" | "renewing") {
      const ticket = ++epoch;
      options.fence();
      if (ticket !== epoch) return;
      void options.rebind({ client: options.client(), principal: owner });
      if (ticket !== epoch) return;
      options.publish({ status: reason, stale: true, mutationAllowed: false });
    },
    async rebind(binding: ApplicationBinding) {
      const ticket = ++epoch,
        preserve = sameOwner(owner, binding.principal);
      owner = binding.principal ? { ...binding.principal } : null;
      options.fence();
      if (ticket !== epoch) return;
      options.client().invalidateSession();
      if (ticket !== epoch) return;
      options.setClient(binding.client);
      if (ticket !== epoch) return;
      if (!preserve) options.clear();
      if (ticket !== epoch) return;
      await options.rebind(binding);
      if (ticket !== epoch) return;
      options.publish({
        status: owner ? "renewing" : "cleared",
        stale: true,
        mutationAllowed: false,
      });
      if (!owner || ticket !== epoch) return;
      const outcome = await options.renew(preserve);
      if (ticket !== epoch) return;
      options.publish({
        status: outcome === "fresh" ? "active" : outcome,
        stale: outcome === "transient",
        mutationAllowed: outcome === "fresh",
      });
    },
    clear() {
      const ticket = ++epoch;
      owner = null;
      options.fence();
      if (ticket !== epoch) return;
      options.clear();
      if (ticket !== epoch) return;
      void options.rebind({ client: options.client(), principal: null });
      if (ticket !== epoch) return;
      options.publish({
        status: "cleared",
        stale: false,
        mutationAllowed: false,
      });
    },
  };
}
