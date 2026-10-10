<script lang="ts">
  import PublicFormsConsumer from "./PublicFormsConsumer.svelte";
  const showForms = new URLSearchParams(location.search).has("forms");
  import PublicConversationDetailsConsumer from "./PublicConversationDetailsConsumer.svelte";
  const showLayoutAcceptance = new URLSearchParams(location.search).has("layout-acceptance");
  import UiCompositionConsumer from "./UiCompositionConsumer.svelte";
  const showUi = new URLSearchParams(location.search).has("ui");
  import StudioConsumer from "./StudioConsumer.svelte";
  const showStudio = new URLSearchParams(location.search).has("studio");
  import { publicClientContract } from "./client-contract.ts";
  let contract = $state<string>("not run");
  import { Input, Button, Textarea, NativeSelect, NativeSelectOption, NativeSelectOptGroup, Checkbox, Slider, Label } from "rom-studio/controls";
  let text = $state("initial");
  let draft = $state("draft");
  let selection = $state("first");
  let checked = $state(false);
  let amount = $state(25);
  let inputRef = $state<HTMLInputElement | null>(null);
  let textareaRef = $state<HTMLTextAreaElement | null>(null);
  let selectRef = $state<HTMLSelectElement | null>(null);
  let optionRef = $state<HTMLOptionElement | null>(null);
  let groupRef = $state<HTMLOptGroupElement | null>(null);
  let buttonRef = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  let checkboxRef = $state<HTMLButtonElement | null>(null);
  let sliderRef = $state<HTMLSpanElement | null>(null);
  let labelRef = $state<HTMLLabelElement | null>(null);
</script>

{#if showForms}
  <PublicFormsConsumer />
{:else if showLayoutAcceptance}
  <PublicConversationDetailsConsumer />
{:else if showUi}
  <UiCompositionConsumer />
{:else if showStudio}
  <StudioConsumer />
{:else}
<main>
  <h1>Public control consumer</h1>
  <Button onclick={async () => {
    try { contract = JSON.stringify(await publicClientContract()); }
    catch (error) { contract = JSON.stringify({ error: error instanceof Error ? error.message : "Public contract failed" }); }
  }}>Prepare installed mutation</Button>
  <output data-testid="client-contract">{contract}</output>
  <Label for="text" class="consumer-label" bind:ref={labelRef}>Name</Label>
  <Input id="text" class="consumer-input" bind:value={text} bind:ref={inputRef} />
  <Label for="notes">Notes</Label>
  <Textarea id="notes" class="compact-composer" bind:value={draft} bind:ref={textareaRef} aria-invalid="true" />
  <Label for="choice">Choice</Label>
  <NativeSelect id="choice" class="consumer-select" bind:value={selection} bind:ref={selectRef}>
    <NativeSelectOptGroup label="Available" bind:ref={groupRef}>
      <NativeSelectOption value="first" bind:ref={optionRef}>First</NativeSelectOption>
      <NativeSelectOption value="second">Second</NativeSelectOption>
    </NativeSelectOptGroup>
  </NativeSelect>
  <div class="check-row"><Checkbox id="accepted" bind:checked bind:ref={checkboxRef} /><Label for="accepted">Accept</Label></div>
  <Slider type="single" bind:value={amount} bind:ref={sliderRef} min={0} max={100} step={5} aria-label="Amount" />
  <Button class="consumer-button" bind:ref={buttonRef} onclick={() => textareaRef?.focus()}>Focus notes</Button>
  <Input aria-label="Disabled input" disabled value="locked" />
  <Button disabled>Disabled button</Button>
  <output data-testid="values">{JSON.stringify({ text, draft, selection, checked, amount })}</output>
  <output data-testid="refs">{JSON.stringify({ input: inputRef?.tagName, textarea: textareaRef?.tagName, select: selectRef?.tagName, option: optionRef?.tagName, group: groupRef?.tagName, button: buttonRef?.tagName, checkbox: checkboxRef?.tagName, slider: sliderRef?.tagName, label: labelRef?.tagName })}</output>
</main>
{/if}

<style>
  main { max-width: 32rem; padding: 16px; display: grid; gap: 12px; }
  .check-row { display: flex; align-items: center; gap: 12px; }
  main :global(.compact-composer) { field-sizing: fixed; height: 72px; min-height: 72px; max-height: 72px; overflow: auto; }
  main :global(.consumer-select) { width: 100%; }
  main :global(.consumer-select [data-slot="native-select"]) { border-width: 3px; }
  output { overflow-wrap: anywhere; }
</style>
