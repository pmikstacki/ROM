<script lang="ts">
  import { SelectionCard, LayoutControls } from "rom-studio/ui/components";
  let acceptedWork = $state(0);
  let selected = $state<string[]>([]),
    calls = $state(0),
    details = $state(0);
  let authorityToken = $state<unknown>(0),
    recoveryToken = $state(0),
    disabled = $state(false),
    polish = $state(false);
  let mode = $state("accepted");
  let settle:
    ((value: "accepted" | "rejected" | "unknown") => void) | undefined;
  let layout = $state.raw([
    { id: "inspection/exact", x: 0, y: 0, width: 2, height: 2, visible: true },
  ]);
  const options = {
    columns: 6,
    maxRows: 6,
    maxItems: 1,
    catalog: [
      {
        id: "inspection/exact",
        minWidth: 1,
        maxWidth: 4,
        minHeight: 1,
        maxHeight: 4,
      },
    ],
  };
  async function outcome() {
    calls++;
    if (mode === "held")
      return await new Promise<"accepted" | "rejected" | "unknown">(
        (resolve) => {
          settle = resolve;
        },
      );
    if (mode === "throw") {
      acceptedWork++;
      throw Error("private backend exception");
    }
    return mode as "accepted" | "rejected" | "unknown";
  }
  async function toggle(id: string) {
    const owner = authorityToken;
    const result = await outcome();
    if (Object.is(owner, authorityToken) && result === "accepted")
      selected = selected.includes(id)
        ? selected.filter((value) => value !== id)
        : [...selected, id];
    return result;
  }
  const labels = $derived({
    left: "Move left",
    right: "Move right",
    up: "Move up",
    down: "Move down",
    wider: "Widen",
    narrower: "Narrow",
    taller: "Make taller",
    shorter: "Make shorter",
    show: "Show",
    hide: "Hide",
    invalid: "Invalid layout",
    failed: polish ? "Odrzucono zmianę" : "Change rejected",
    unknown: polish ? "Wynik nieznany" : "Outcome unknown",
  });
</script>

<main class="mx-auto max-w-2xl min-w-0 p-4">
  <h1>Equipment selection and layout</h1>
  <button onclick={() => (mode = "rejected")}>Reject commands</button>
  <button onclick={() => (mode = "unknown")}>Unknown commands</button>
  <button onclick={() => (mode = "held")}>Hold commands</button>
  <button onclick={() => (mode = "throw")}>Throw commands</button>
  <button onclick={() => (disabled = true)}>Disable controls</button>
  <button onclick={() => (disabled = false)}>Enable controls</button>
  <button onclick={() => (layout = [{ ...layout[0], width: 999 }])}
    >Corrupt layout</button
  >
  <button
    onclick={() => {
      authorityToken = {};
      selected = [];
    }}>Change authority</button
  >
  <button onclick={() => (authorityToken = NaN)}>Use NaN authority</button>
  <button onclick={() => settle?.("rejected")}>Reject held</button>
  <button
    onclick={() => {
      recoveryToken++;
      mode = "accepted";
    }}>Recover outcome</button
  >
  <button onclick={() => (polish = true)}>Use Polish</button>
  <SelectionCard
    id="equipment/exact?1"
    title="Inspection equipment"
    selected={selected.includes("equipment/exact?1")}
    {authorityToken}
    {recoveryToken}
    {disabled}
    onToggle={toggle}
    failedLabel={labels.failed}
    unknownLabel={labels.unknown}
  >
    <span class="block">{"Long inspection context ".repeat(15)}</span>
    {#snippet actions()}<button onclick={() => details++}
        >Equipment details</button
      >{/snippet}
  </SelectionCard>
  <SelectionCard
    id="equipment/exact?2"
    title="Backup equipment"
    selected={selected.includes("equipment/exact?2")}
    {authorityToken}
    {recoveryToken}
    {disabled}
    onToggle={toggle}
    failedLabel={labels.failed}
    unknownLabel={labels.unknown}
    ><span>Second independent selection</span></SelectionCard
  >
  <LayoutControls
    {layout}
    {options}
    label="Workspace layout"
    itemLabel={() => "Inspection panel"}
    {labels}
    {authorityToken}
    {recoveryToken}
    {disabled}
    onChange={async (_command, proposal) => {
      const owner = authorityToken;
      const result = await outcome();
      if (Object.is(owner, authorityToken) && result === "accepted")
        layout = proposal.map((item) => ({ ...item }));
      return result;
    }}
  />
  <output aria-label="Accepted work">{acceptedWork}</output>
  <output aria-label="Host calls">{calls}</output><output
    aria-label="Details calls">{details}</output
  >
  <output class="block min-w-0 break-all" aria-label="Confirmed layout"
    >{JSON.stringify(layout)}</output
  >
</main>
