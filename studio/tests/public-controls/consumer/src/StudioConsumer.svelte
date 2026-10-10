<script lang="ts">
  import { onDestroy } from "svelte";
  import { App, registerRenderer } from "rom-studio";
  import CustomTitle from "./CustomTitle.svelte";
  const unregister = registerRenderer({ name: "installed-title", version: 1 }, CustomTitle, { layout: "inline" });
  onDestroy(unregister);
</script>
<App />
