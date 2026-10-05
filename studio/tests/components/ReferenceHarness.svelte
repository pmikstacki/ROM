<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  import ValueEditor from "../../src/lib/renderers/ValueEditor.svelte";
  import {
    REFERENCE_LOOKUP,
    type ReferenceLookup,
  } from "../../src/lib/renderers/reference-lookup.ts";
  import { createApplication } from "../../src/lib/application/controller.ts";
  import { createClient } from "../../src/lib/client/client.ts";
  import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
  import type {
    ResourceDescriptor,
    WireValue,
    WireObject,
  } from "../../src/lib/client/types.ts";
  const params = new URLSearchParams(location.search);
  let count = $state(0),
    denied = $state(false),
    slow = $state(false),
    assignee = $state<WireValue>("existing-id"),
    hidden = $state<WireValue>("hidden-id"),
    error = $state("");
  const titleWithheld = params.has("withheld");
  const people: ResourceDescriptor = {
    kind: "people",
    version: 1,
    fields: titleWithheld ? [] : [{ name: "name", shape: { type: "string" } }],
    actions: [],
    action_inputs: [],
    presentation: {
      label: "Person",
      ...(titleWithheld ? {} : { title_field: "name" }),
    },
  };
  const tasks: ResourceDescriptor = {
    kind: "tasks",
    version: 1,
    fields: [],
    actions: [],
    action_inputs: [],
  };
  const candidates = [
    {
      key: { kind: "people", id: "  id with spaces  " },
      revision: 1n,
      value: { name: "Ada Lovelace" },
    },
    {
      key: { kind: "people", id: "α/people?exact#key" },
      revision: 2n,
      value: { name: "Grace Hopper" },
    },
  ];
  function reply(value: WireValue) {
    return new Response(stringifyWire(value), {
      headers: { "Content-Type": "application/json" },
    });
  }
  const client = createClient({
    base: "https://fixture.invalid/api",
    fetch: async (url, init) => {
      if (String(url).endsWith("/discover"))
        return reply({
          version: 1,
          resources: [tasks, people],
        } as unknown as WireValue);
      const body = parseWire(String(init?.body)) as WireObject;
      if (body.kind !== "people") return reply([]);
      count++;
      if (slow) await new Promise((resolve) => setTimeout(resolve, 600));
      if (denied)
        return new Response('{"error":"denied"}', {
          status: 403,
          headers: { "Content-Type": "application/json" },
        });
      return reply(candidates);
    },
  });
  const controller = createApplication(client);
  let snapshot = $state.raw(controller.state);
  const unsubscribe = controller.subscribe((next) => (snapshot = next));
  if (!params.has("no-context"))
    setContext<ReferenceLookup>(REFERENCE_LOOKUP, {
      descriptor: (kind) =>
        snapshot.phase === "ready"
          ? snapshot.descriptors.find((d) => d.kind === kind)
          : undefined,
      lookup: (kind, search, signal) =>
        controller.lookupResources(kind, search, signal),
    });
  onMount(() => {
    void controller.connect();
  });
  onDestroy(() => {
    unsubscribe();
    controller.disconnect();
  });
</script>

<main class="mx-auto max-w-md space-y-4 p-4">
  <h1>Reference fixture</h1>
  <div class="flex flex-wrap gap-2">
    <button onclick={() => (denied = true)}>Deny lookup</button>
    <button onclick={() => (slow = true)}>Slow lookup</button>
    <button onclick={() => controller.disconnect()}>Disconnect session</button>
    <button onclick={() => void controller.connect()}>Reconnect session</button>
  </div>
  <p data-testid="session">{snapshot.phase}</p>
  <p data-testid="selected-kind">{snapshot.kind}</p>
  <p data-testid="request-count">{count}</p>
  <ValueEditor
    shape={{ type: "reference", value: { kind: "people" } }}
    value={assignee}
    label="assignee"
    displayLabel="Assigned person"
    onchange={(next) => (assignee = next)}
    onerror={(next) => (error = next)}
    direct
  />
  <ValueEditor
    shape={{ type: "reference", value: { kind: "hidden" } }}
    value={hidden}
    label="hidden"
    displayLabel="Private link"
    onchange={(next) => (hidden = next)}
    direct
  />
  <output data-testid="stored-id">{stringifyWire(assignee)}</output>
  <output data-testid="stored-hidden-id">{stringifyWire(hidden)}</output>
  <p data-testid="field-error">{error}</p>
</main>
