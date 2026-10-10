<script lang="ts">
  import { HistoryList } from "rom-studio/ui/components";
  const longTitle = "Inspection ".repeat(45);
  let locale = $state("en-US");
  let authorityToken = $state(0);
  let selected = $state<string | null>('opaque-["local","human","alice"]');
  let empty = $state(false);
  let hold = $state(false);
  let uncertain = $state(false);
  let allow = $state(true);
  let attempts = $state(0);
  let settle: ((fail: boolean) => void) | undefined;
  const entries = [
    {
      id: 'opaque-["local","human","alice"]',
      title: longTitle,
      instant: Date.UTC(2026, 9, 8),
      count: 1000,
    },
    {
      id: "work-order-2",
      title: "Maintenance summary",
      instant: Date.UTC(2026, 9, 7),
      count: 1,
    },
  ];
  async function select(id: string) {
    attempts++;
    const owner = authorityToken;
    if (hold)
      await new Promise<void>((resolve, reject) => {
        settle = (fail) =>
          fail ? reject(Error("private exception")) : resolve();
      });
    if (owner === authorityToken && allow) selected = id;
  }
  const messages = $derived(
    (key: string, values?: Readonly<Record<string, string | number>>) => {
      if (key === "history.selectionFailed")
        return locale === "pl-PL"
          ? "Nie można otworzyć wpisu"
          : "Cannot open entry";
      return `${values?.formattedCount} ${locale === "pl-PL" ? "wpisów" : "entries"}`;
    },
  );
</script>

<main class="mx-auto max-w-xl min-w-0 p-4">
  <h1>Inspection history workspace</h1>
  <button
    onclick={() => {
      authorityToken = NaN;
    }}>Use NaN token</button
  >
  <button
    onclick={() => {
      locale = "pl-PL";
    }}>Use Polish</button
  >
  <button
    onclick={() => {
      empty = !empty;
    }}>Toggle empty</button
  >
  <button
    onclick={() => {
      hold = true;
    }}>Hold navigation</button
  >
  <button
    onclick={() => {
      uncertain = true;
    }}>Uncertain mutation</button
  >
  <button
    onclick={() => {
      allow = false;
    }}>Refuse navigation</button
  >
  <button
    onclick={() => {
      settle?.(true);
    }}>Fail navigation</button
  >
  <button
    onclick={() => {
      authorityToken++;
      selected = null;
    }}>Change authority</button
  >
  <output aria-label="Navigation attempts">{attempts}</output>
  <HistoryList
    entries={empty ? [] : entries}
    selectedId={selected}
    onSelect={select}
    {messages}
    {locale}
    {authorityToken}
    disabled={uncertain}
    empty={locale === "pl-PL" ? "Brak historii" : "No history"}
    label="Inspection history"
  />
</main>
