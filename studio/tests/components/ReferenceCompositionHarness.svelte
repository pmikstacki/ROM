<script lang="ts">
  import { ReferencePicker } from "rom-studio/ui/components";
  const descriptor = {
    kind: "people",
    version: 1,
    fields: [],
    actions: [],
    action_inputs: [],
  };
  let authorityToken = $state(0),
    value = $state("manual/exact"),
    count = $state(0),
    aborted = $state(0);
  let mode = $state("ready"),
    readonly = $state(false),
    revokeOnSelection = $state(false),
    localized = $state(false);
  const lookup = {
    descriptor: () => descriptor,
    lookup: async (_kind: string, search: string, signal: AbortSignal) => {
      count++;
      const captured = mode;
      signal.addEventListener("abort", () => aborted++, { once: true });
      if (captured === "slow")
        await new Promise((resolve) => setTimeout(resolve, 600));
      if (captured === "denied")
        return {
          status: "denied" as const,
          message: "Unavailable to this owner. Enter an exact ID.",
        };
      const candidates =
        captured === "overflow"
          ? Array.from({ length: 21 }, (_, index) => ({
              id: String(index),
              title: "Overflow",
            }))
          : [{ id: "  α/exact?key#id  ", title: "Owner private label" }];
      return { status: "ready" as const, candidates, limited: false };
    },
  };
</script>

<main class="max-w-md p-4">
  <h1>Independent reference form</h1>
  <button onclick={() => authorityToken++}>Change authority</button>
  <button onclick={() => (mode = "slow")}>Slow response</button>
  <button onclick={() => (mode = "denied")}>Deny response</button>
  <button onclick={() => (mode = "overflow")}>Overflow response</button>
  <button onclick={() => (readonly = true)}>Make readonly</button>
  <button onclick={() => (revokeOnSelection = true)}
    >Revoke during selection</button
  >
  <button onclick={() => (localized = true)}>Use Polish labels</button>
  <ReferencePicker
    kind="people"
    {value}
    label="Assigned person"
    {lookup}
    {authorityToken}
    {readonly}
    messages={localized
      ? {
          chooseLabel: "Wybierz osobę",
          searchLabel: "Szukaj osób",
          loading: "Wczytywanie osób…",
        }
      : {}}
    onchange={(next) => {
      value = next as string;
      if (revokeOnSelection) authorityToken++;
    }}
  />
  <output aria-label="Exact ID">{JSON.stringify(value)}</output>
  <output aria-label="Lookup count">{count}</output>
  <output aria-label="Aborted count">{aborted}</output>
</main>
