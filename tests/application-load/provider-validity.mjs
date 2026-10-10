// Admission proof only. ROM still validates the actual provider signature and grant.
const providerPath = "/api/v3/providers/oauth2/1/";
function profile(value) {
  if (!["exploratory", "full"].includes(value))
    throw Error("closed load profile");
  return value === "full" ? 1200 : 300;
}
export function requireTokenWindow(
  timing,
  now,
  trafficMs,
  cleanupMs,
  selected,
) {
  const seconds = profile(selected);
  if (
    !timing ||
    Object.keys(timing).some(
      (key) => !["iat", "exp", "received_at_ms"].includes(key),
    ) ||
    ![
      timing.iat,
      timing.exp,
      timing.received_at_ms,
      now,
      trafficMs,
      cleanupMs,
    ].every(Number.isSafeInteger) ||
    timing.iat < 0 ||
    timing.exp <= timing.iat ||
    timing.exp - timing.iat > seconds ||
    timing.received_at_ms < timing.iat * 1000 ||
    timing.received_at_ms >= timing.exp * 1000 ||
    now < timing.received_at_ms ||
    trafficMs < 1 ||
    trafficMs > (selected === "full" ? 840000 : 150000) ||
    cleanupMs < 45000 ||
    cleanupMs > 120000
  )
    throw Error("bounded actual provider token timing required");
  const remaining = timing.exp * 1000 - now;
  if (!Number.isSafeInteger(remaining) || remaining < trafficMs + cleanupMs)
    throw Error("fresh login required before load traffic");
  return {
    remaining_ms: remaining,
    required_ms: trafficMs + cleanupMs,
    profile: selected,
  };
}
function validity(response) {
  const value = response?.access_token_validity;
  if (
    typeof value !== "string" ||
    !/^(minutes|seconds)=[1-9][0-9]{0,3}$/.test(value)
  )
    throw Error("bounded synthetic validity required");
  return value;
}
export async function withLoadValidity(api, selected, body, evidence) {
  profile(selected);
  if (
    typeof api?.request !== "function" ||
    typeof body !== "function" ||
    !Array.isArray(evidence) ||
    evidence.length > 8
  )
    throw Error("bounded provider validity scope required");
  const original = validity(await api.request(providerPath));
  evidence.push({ stage: "validity_read", original, profile: selected });
  if (selected === "exploratory") {
    if (original !== "minutes=5")
      throw Error("exploratory five-minute provider validity required");
    return body();
  }
  let attempted = false,
    failure;
  try {
    attempted = true;
    const changed = validity(
      await api.request(providerPath, "PATCH", {
        access_token_validity: "minutes=20",
      }),
    );
    if (
      changed !== "minutes=20" ||
      validity(await api.request(providerPath)) !== "minutes=20"
    )
      throw Error("synthetic load validity not confirmed");
    evidence.push({ stage: "validity_active", during: "minutes=20" });
    return await body();
  } catch (error) {
    failure = error;
    throw error;
  } finally {
    if (attempted) {
      let restored = false;
      try {
        const changed = validity(
          await api.request(providerPath, "PATCH", {
            access_token_validity: original,
          }),
        );
        restored =
          changed === original &&
          validity(await api.request(providerPath)) === original;
      } catch {
        restored = false;
      } finally {
        evidence.push({ stage: "validity_restore", restored });
      }
      if (!restored) {
        const restoration = Error(
          "synthetic provider validity restoration failed",
        );
        if (failure)
          throw new AggregateError(
            [failure, restoration],
            "load scope and restoration failed",
          );
        throw restoration;
      }
    }
  }
}
