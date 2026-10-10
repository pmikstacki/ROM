import { createSessionLifecycle, type SessionCheck } from "rom-studio/auth";

/** Installed public auth composition; fabricated authority, no provider requests. */
export async function publicAuthContract(): Promise<void> {
  let result: SessionCheck = {
    status: "authenticated",
    identity: {
      principal: { authority: "fixture", kind: "human", subject: "alice" },
      generation: "one",
      expiresAt: 200,
    },
  };
  const transitions: string[] = [];
  const session = createSessionLifecycle({
    now: () => 100,
    schedule: () => () => {},
    driver: { check: async () => result, logout: async () => {} },
    onTransition: ({ kind }) => { transitions.push(kind); },
  });
  try {
    await session.refresh();
    if (session.state.identity?.principal.subject !== "alice")
      throw Error("installed auth identity missing");
    result = { status: "transient", code: "unavailable" };
    await session.refresh();
    if (session.state.identity?.expiresAt !== 200)
      throw Error("installed auth transient extended or dropped authority");
    await session.logout();
    if (session.state.identity !== null || transitions.join(",") !== "changed,transient,cleared")
      throw Error("installed auth logout did not clear authority");
  } finally { session.dispose(); }
}
